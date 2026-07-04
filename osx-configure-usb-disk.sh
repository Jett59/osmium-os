#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONFIG_FILE="${CONFIG_FILE:-"$SCRIPT_DIR/.config"}"
EXPORT_NAME="${USB_DISK_EXPORT_NAME:-USB_DISK}"

usage() {
    cat >&2 <<EOF
Usage: ${0##*/} [diskN|/dev/diskN|/dev/rdiskN]

Find one external USB whole disk on macOS and write this to .config:
  export ${EXPORT_NAME}=/dev/rdiskN

If more than one USB disk is attached, pass the disk identifier explicitly.
EOF
}

info_value() {
    local key="$1"

    awk -F ':' -v key="$key" '
        {
            field = $1
            gsub(/^[ \t]+|[ \t]+$/, "", field)

            if (field == key) {
                sub(/^[^:]*:[ \t]*/, "", $0)
                gsub(/^[ \t]+|[ \t]+$/, "", $0)
                print $0
                exit
            }
        }
    '
}

normalize_disk_id() {
    local input="$1"
    local disk_id

    disk_id="${input##*/}"
    disk_id="${disk_id#r}"

    if [[ ! "$disk_id" =~ ^disk[0-9]+$ ]]; then
        echo "Invalid disk identifier: $input" >&2
        usage
        exit 2
    fi

    printf '%s\n' "$disk_id"
}

disk_info() {
    diskutil info "/dev/$1" 2>/dev/null
}

is_external_usb_whole_disk() {
    local disk_id="$1"
    local info protocol internal location whole

    if ! info="$(disk_info "$disk_id")"; then
        return 1
    fi

    protocol="$(printf '%s\n' "$info" | info_value "Protocol")"
    internal="$(printf '%s\n' "$info" | info_value "Internal")"
    location="$(printf '%s\n' "$info" | info_value "Device Location")"
    whole="$(printf '%s\n' "$info" | info_value "Whole")"

    [[ "$protocol" == "USB" && "$whole" == "Yes" && ( "$internal" == "No" || "$location" == "External" ) ]]
}

disk_summary() {
    local disk_id="$1"
    local info name size protocol mounted_volumes

    info="$(disk_info "$disk_id")"
    name="$(printf '%s\n' "$info" | info_value "Device / Media Name")"
    size="$(printf '%s\n' "$info" | info_value "Disk Size")"
    protocol="$(printf '%s\n' "$info" | info_value "Protocol")"
    mounted_volumes="$(mounted_volume_summaries "$disk_id")"

    [[ -n "$name" ]] || name="unknown media"
    [[ -n "$size" ]] || size="unknown size"
    [[ -n "$protocol" ]] || protocol="unknown protocol"

    printf '/dev/%s - %s, %s, %s, mounted volumes: %s\n' "$disk_id" "$name" "$size" "$protocol" "$mounted_volumes"
}

partition_ids() {
    local disk_id="$1"

    diskutil list "/dev/$disk_id" 2>/dev/null | awk -v disk="$disk_id" '
        $NF ~ "^" disk "s[0-9]+$" {
            print $NF
        }
    '
}

mounted_volume_summaries() {
    local disk_id="$1"
    local partition_id info mounted volume mount_point
    local summaries=()
    local result=""

    while IFS= read -r partition_id; do
        [[ -n "$partition_id" ]] || continue

        if ! info="$(disk_info "$partition_id")"; then
            continue
        fi

        mounted="$(printf '%s\n' "$info" | info_value "Mounted")"
        [[ "$mounted" == "Yes" ]] || continue

        volume="$(printf '%s\n' "$info" | info_value "Volume Name")"
        mount_point="$(printf '%s\n' "$info" | info_value "Mount Point")"

        [[ -n "$volume" && "$volume" != "Not applicable (no file system)" ]] || volume="/dev/$partition_id"
        [[ -n "$mount_point" && "$mount_point" != "Not applicable (no file system)" ]] || mount_point="unknown mount point"

        summaries+=("$volume at $mount_point")
    done < <(partition_ids "$disk_id")

    if [[ "${#summaries[@]}" -eq 0 ]]; then
        printf 'none'
        return
    fi

    for summary in "${summaries[@]}"; do
        result="${result:+$result; }$summary"
    done

    printf '%s' "$result"
}

find_external_physical_disks() {
    diskutil list external physical 2>/dev/null | awk '
        /^\/dev\/disk[0-9]+[ \t]+\(external, physical\):/ {
            sub("^/dev/", "", $1)
            print $1
        }
    '
}

write_disk_report() {
    local report_file="$1"
    shift

    : > "$report_file"

    if [[ "$#" -eq 0 ]]; then
        printf 'none\n' > "$report_file"
        return
    fi

    for disk_id in "$@"; do
        disk_summary "$disk_id" >> "$report_file"
    done
}

print_disk_report() {
    local report_file="$1"
    local selected_disk="$2"
    local selected_value="$3"

    echo "External physical disks seen:"
    sed 's/^/  /' "$report_file"
    echo "Selected disk:"
    echo "  /dev/${selected_disk} -> ${selected_value}"
}

write_config_export() {
    local name="$1"
    local value="$2"
    local config_file="$3"
    local mode tmp

    mkdir -p "$(dirname "$config_file")"
    touch "$config_file"

    mode="$(stat -f '%Lp' "$config_file" 2>/dev/null || stat -c '%a' "$config_file" 2>/dev/null || true)"
    tmp="$(mktemp "${config_file}.XXXXXX")"

    awk -v name="$name" '
        BEGIN {
            pattern = "^[ \t]*export[ \t]+" name "="
        }

        $0 == "# BEGIN configure-usb-disk" {
            in_block = 1
            next
        }

        $0 == "# END configure-usb-disk" {
            in_block = 0
            next
        }

        in_block {
            next
        }

        $0 ~ pattern {
            next
        }

        {
            print
        }
    ' "$config_file" > "$tmp"

    echo "export ${name}=${value}" >> "$tmp"

    [[ -z "$mode" ]] || chmod "$mode" "$tmp"
    mv "$tmp" "$config_file"
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
    usage
    exit 0
fi

if [[ $# -gt 1 ]]; then
    usage
    exit 2
fi

if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "This script uses diskutil and only runs on macOS." >&2
    exit 1
fi

if ! command -v diskutil >/dev/null 2>&1; then
    echo "diskutil was not found." >&2
    exit 1
fi

if [[ ! "$EXPORT_NAME" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]]; then
    echo "Invalid export variable name: $EXPORT_NAME" >&2
    exit 2
fi

seen_disks=()
while IFS= read -r disk_id; do
    [[ -n "$disk_id" ]] || continue
    seen_disks+=("$disk_id")
done < <(find_external_physical_disks)

if [[ $# -eq 1 ]]; then
    selected_disk="$(normalize_disk_id "$1")"

    if ! is_external_usb_whole_disk "$selected_disk"; then
        echo "/dev/$selected_disk is not an external USB whole disk." >&2
        exit 1
    fi
else
    candidates=()
    for disk_id in "${seen_disks[@]}"; do
        if is_external_usb_whole_disk "$disk_id"; then
            candidates+=("$disk_id")
        fi
    done

    case "${#candidates[@]}" in
        0)
            report_file="$(mktemp)"
            write_disk_report "$report_file" "${seen_disks[@]}"
            echo "External physical disks seen:" >&2
            sed 's/^/  /' "$report_file" >&2
            rm -f "$report_file"
            echo "No external USB whole disk found." >&2
            echo "Check with: diskutil list external physical" >&2
            exit 1
            ;;
        1)
            selected_disk="${candidates[0]}"
            ;;
        *)
            echo "More than one external USB whole disk found:" >&2
            report_file="$(mktemp)"
            write_disk_report "$report_file" "${seen_disks[@]}"
            echo "External physical disks seen:" >&2
            sed 's/^/  /' "$report_file" >&2
            rm -f "$report_file"
            echo "Run ${0##*/} diskN with the intended disk." >&2
            exit 1
            ;;
    esac
fi

usb_disk="/dev/r${selected_disk}"

selected_disk_was_seen=n
for disk_id in "${seen_disks[@]}"; do
    if [[ "$disk_id" == "$selected_disk" ]]; then
        selected_disk_was_seen=y
        break
    fi
done

if [[ "$selected_disk_was_seen" == "n" ]]; then
    seen_disks+=("$selected_disk")
fi

report_file="$(mktemp)"
trap 'rm -f "$report_file"' EXIT

write_disk_report "$report_file" "${seen_disks[@]}"
write_config_export "$EXPORT_NAME" "$usb_disk" "$CONFIG_FILE"
print_disk_report "$report_file" "$selected_disk" "$usb_disk"

echo "Wrote export ${EXPORT_NAME}=${usb_disk} to ${CONFIG_FILE}"
