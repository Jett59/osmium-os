#!/bin/bash

set -e
set +x

source .config

cargo build --target ./targets/$ARCH.json -Z build-std=core --release

mkdir -p build/isoroot/boot/grub
cp target/$ARCH/release/bare-bones-kernel build/isoroot/boot/bare-bones-kernel
strip build/isoroot/boot/bare-bones-kernel
cp grub/config.cfg build/isoroot/boot/grub/grub.cfg
grub-mkrescue -d /usr/lib/grub/i386-pc -o build/bare-bones.iso build/isoroot
