#!/bin/sh

set -e

dd if=/dev/zero of=build/flash0.img bs=1M count=64

LINUX_APT_FILE="/usr/share/qemu-efi-aarch64/QEMU_EFI.fd"
LINUX_BUILD_FILE="/usr/local/share/qemu/edk2-aarch64-code.fd"
MAC_BREW_FILE="/opt/homebrew/share/qemu/edk2-aarch64-code.fd"
WINDOWS_FILE="C:\\Program Files\\qemu\\share\\edk2-aarch64-code.fd"
for FILE in "${LINUX_APT_FILE}" "${LINUX_BUILD_FILE}" "${MAC_BREW_FILE}" "${WINDOWS_FILE}"
do
  if [ -f "$FILE" ]; then
    echo "$FILE exists. Writing to flash0."
    dd if="$FILE" of=build/flash0.img conv=notrunc bs=1M
    break
  fi
done

echo "Launching qemu..."
exec qemu-system-aarch64 -machine virt -cpu cortex-a72 -drive file=build/flash0.img,format=raw,if=pflash -hda build/osmium.img -device ramfb $@
