#!/bin/sh

if [ ! -f build/flash0.img ]; then
    dd if=/dev/zero of=build/flash0.img bs=1M count=64
    dd if=/usr/share/qemu-efi-aarch64/QEMU_EFI.fd of=build/flash0.img conv=notrunc
fi

qemu-system-aarch64 -machine virt -cpu cortex-a57 -drive file=build/flash0.img,format=raw,if=pflash -hda build/osmium.img -device virtio-gpu-pci
