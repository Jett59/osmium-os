#!/usr/bin/env bash

set -e

mkdir -p ./build/uefi_firmware
cd ./build/uefi_firmware
curl -L https://github.com/pftf/RPi4/releases/download/v1.34/RPi4_UEFI_Firmware_v1.34.zip --output uefi_firmware.zip
unzip uefi_firmware.zip
rm -f uefi_firmware.zip


