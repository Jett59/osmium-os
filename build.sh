#!/bin/bash

set -e
set +x

source .config

cargo build --target ./targets/$ARCH.json -Z build-std="core,alloc" --release

mkdir -p build/isoroot/boot/grub
cp target/$ARCH/release/osmium build/isoroot/boot/osmium
strip build/isoroot/boot/osmium
cp grub/config.cfg build/isoroot/boot/grub/grub.cfg
grub-mkrescue -d /usr/lib/grub/i386-pc -o build/osmium.iso build/isoroot
