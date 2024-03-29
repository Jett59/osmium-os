#!/bin/sh

set +x
set -e

mkdir -p build

cd bootloader
./build.sh
cd ..

# Ref: https://superuser.com/questions/1657478/how-make-a-bootable-iso-for-my-uefi-application-bare-bones#comment2537987_1657538
dd if=/dev/zero of=build/osmium.img bs=48000000 count=1
mformat -i build/osmium.img ::

mmd -i build/osmium.img ::/EFI
mmd -i build/osmium.img ::/EFI/BOOT
mcopy -i build/osmium.img bootloader/build/bootloader.efi ::/EFI/BOOT/BOOTAA64.EFI

mmd -i build/osmium.img ::/boot
mmd -i build/osmium.img ::/boot/beryllium
mcopy -i build/osmium.img config/boot.toml ::/boot/beryllium/boot.toml
mcopy -i build/osmium.img kernel/build/osmium ::/boot/osmium
mcopy -i build/osmium.img build/initial_ramdisk.tar ::/boot/initial_ramdisk.tar
