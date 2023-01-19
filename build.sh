#!/bin/bash

set +x
set -e

source .config

# Find the command line option for the given $PROFILE (debug should have no option, otherwise --$PROFILE)
PROFILE_OPTION=""

if [ "$PROFILE" != "debug" ]; then
    PROFILE_OPTION="--$PROFILE"
fi

export PROFILE_OPTION

cd kernel && ./build.sh && cd ..

mkdir -p build/isoroot/boot/grub
cp kernel/build/osmium build/isoroot/boot/osmium
cp grub/config.cfg build/isoroot/boot/grub/grub.cfg

grub-mkrescue -d /usr/lib/grub/i386-pc -o build/osmium.iso build/isoroot
