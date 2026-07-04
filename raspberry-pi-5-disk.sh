#!/usr/bin/env bash

set -e
set +x

mkdir -p ./build/uefi_firmware
rm -rf ./build/uefi_firmware/*
cd ./build/uefi_firmware
curl -L https://github.com/NumberOneGit/rpi5-uefi/releases/download/v0.1/RPI5_D0.zip --output uefi_firmware.zip
unzip uefi_firmware.zip
rm -f uefi_firmware.zip

mcopy -D overwrite -i ../osmium.img ./* ::

cd ..
echo "Ready to write to the Raspberry Pi SD card from:"
echo "$(pwd)/osmium.img"
