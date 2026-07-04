#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONFIG_FILE="${CONFIG_FILE:-"$SCRIPT_DIR/.config"}"
IMAGE_FILE="${IMAGE_FILE:-"$SCRIPT_DIR/build/osmium.img"}"
ASSUME_YES=n

usage() {
    cat >&2 <<EOF
Usage: ${0##*/} [--yes]

Write build/osmium.img to the USB disk configured in .config:
  export USB_DISK=/dev/rdiskN

Options:
  --yes    Skip the interactive confirmation prompt.
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

disk_info() {
    diskutil info "$1" 2>/dev/null
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

        if ! info="$(disk_info "/dev/$partition_id")"; then
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

normalize_usb_disk() {
    local input="$1"
    local disk_id

    disk_id="${input##*/}"
    disk_id="${disk_id#r}"

    if [[ ! "$disk_id" =~ ^disk[0-9]+$ ]]; then
        echo "USB_DISK must be a whole disk such as /dev/rdisk4, not: $input" >&2
        exit 2
    fi

    printf '%s\n' "$disk_id"
}

image_size_bytes() {
    stat -f '%z' "$IMAGE_FILE"
}

disk_size_bytes() {
    local disk="$1"
    local info

    info="$(disk_info "$disk")"
    printf '%s\n' "$info" | sed -n 's/.*(\([0-9][0-9]*\) Bytes).*/\1/p' | head -n 1
}

human_size() {
    local bytes="$1"

    numfmt --to=iec --suffix=B "$bytes" 2>/dev/null || printf '%s bytes' "$bytes"
}

confirm_write() {
    local reply

    if [[ "$ASSUME_YES" == "y" ]]; then
        return
    fi

    echo
    echo "This will overwrite ${BLOCK_DEVICE} (${RAW_DEVICE})."
    read -r -p "Type YES to continue: " reply

    if [[ "$reply" != "YES" ]]; then
        echo "Aborted."
        exit 1
    fi
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --yes|-y)
            ASSUME_YES=y
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            usage
            exit 2
            ;;
    esac
done

if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "This script uses diskutil and only runs on macOS." >&2
    exit 1
fi

if ! command -v diskutil >/dev/null 2>&1; then
    echo "diskutil was not found." >&2
    exit 1
fi

if [[ ! -f "$CONFIG_FILE" ]]; then
    echo "Missing config file: $CONFIG_FILE" >&2
    echo "Run ./osx-configure-usb-disk.sh first." >&2
    exit 1
fi

. "$CONFIG_FILE"

if [[ -z "${USB_DISK:-}" ]]; then
    echo "USB_DISK is not set in $CONFIG_FILE" >&2
    echo "Run ./osx-configure-usb-disk.sh first." >&2
    exit 1
fi

if [[ ! -s "$IMAGE_FILE" ]]; then
    echo "Missing built disk image: $IMAGE_FILE" >&2
    echo "Run ./build.sh and the Raspberry Pi disk script first." >&2
    exit 1
fi

DISK_ID="$(normalize_usb_disk "$USB_DISK")"
BLOCK_DEVICE="/dev/$DISK_ID"
RAW_DEVICE="/dev/r$DISK_ID"

if [[ ! -c "$RAW_DEVICE" ]]; then
    echo "Configured USB_DISK does not exist: $RAW_DEVICE" >&2
    exit 1
fi

INFO="$(disk_info "$BLOCK_DEVICE")"
PROTOCOL="$(printf '%s\n' "$INFO" | info_value "Protocol")"
INTERNAL="$(printf '%s\n' "$INFO" | info_value "Internal")"
LOCATION="$(printf '%s\n' "$INFO" | info_value "Device Location")"
WHOLE="$(printf '%s\n' "$INFO" | info_value "Whole")"
READ_ONLY="$(printf '%s\n' "$INFO" | info_value "Media Read-Only")"
MEDIA_NAME="$(printf '%s\n' "$INFO" | info_value "Device / Media Name")"
DISK_SIZE="$(disk_size_bytes "$BLOCK_DEVICE")"
IMAGE_SIZE="$(image_size_bytes)"

if [[ "$PROTOCOL" != "USB" || "$WHOLE" != "Yes" || ( "$INTERNAL" != "No" && "$LOCATION" != "External" ) ]]; then
    echo "$BLOCK_DEVICE is not an external USB whole disk." >&2
    exit 1
fi

if [[ "$READ_ONLY" == "Yes" ]]; then
    echo "$BLOCK_DEVICE is read-only." >&2
    exit 1
fi

if [[ -n "$DISK_SIZE" && "$IMAGE_SIZE" -gt "$DISK_SIZE" ]]; then
    echo "Image is larger than target disk." >&2
    echo "Image: $(human_size "$IMAGE_SIZE")" >&2
    echo "Disk:  $(human_size "$DISK_SIZE")" >&2
    exit 1
fi

[[ -n "$MEDIA_NAME" ]] || MEDIA_NAME="unknown media"

echo "Image:"
echo "  $IMAGE_FILE ($(human_size "$IMAGE_SIZE"))"
echo "Target USB disk:"
echo "  $BLOCK_DEVICE - $MEDIA_NAME, ${PROTOCOL}, mounted volumes: $(mounted_volume_summaries "$DISK_ID")"
echo "Raw write device:"
echo "  $RAW_DEVICE"
echo "Tip: press Ctrl-T while dd is running to print macOS dd progress."

confirm_write

echo "Unmounting $BLOCK_DEVICE..."
diskutil unmountDisk "$BLOCK_DEVICE"

echo "Writing image with dd..."
sudo dd if="$IMAGE_FILE" of="$RAW_DEVICE" bs=4m

echo "Flushing writes..."
sync

echo "Ejecting $BLOCK_DEVICE..."
diskutil eject "$BLOCK_DEVICE"

echo "Done."
