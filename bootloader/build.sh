#!/bin/sh

set +x
set -e

cargo build --target $ARCH-unknown-uefi $PROFILE_OPTION

mkdir -p build
cp target/$ARCH-unknown-uefi/$PROFILE/bootloader.efi build/
