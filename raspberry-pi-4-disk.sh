#!/usr/bin/env bash

set -e
set +x

mkdir -p ./build/uefi_firmware
rm -rf ./build/uefi_firmware/*
cd ./build/uefi_firmware
curl -L https://github.com/pftf/RPi4/releases/download/v1.35/RPi4_UEFI_Firmware_v1.35.zip --output uefi_firmware.zip
unzip uefi_firmware.zip
rm -f uefi_firmware.zip

mcopy -D overwrite -i ../osmium.img ./* ::

cd ..
echo "Ready to write to the Raspberry Pi SD card from:"
echo "$(pwd)/osmium.img"
