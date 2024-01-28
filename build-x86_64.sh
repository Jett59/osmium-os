#!/bin/sh

set -e
set +x

mkdir -p build/isoroot/boot/grub
cp kernel/build/osmium build/isoroot/boot/osmium
cp build/initramfs.tar build/isoroot/boot/initramfs.tar
cp grub/config.cfg build/isoroot/boot/grub/grub.cfg

grub-mkrescue -d /usr/lib/grub/i386-pc -o build/osmium.iso build/isoroot
