#!/bin/bash

set -e
set +x

source .config

# Find the command line option for the given $PROFILE (debug should have no option, otherwise --$PROFILE)
PROFILE_OPTION=""

if [ "$PROFILE" != "debug" ]; then
    PROFILE_OPTION="--$PROFILE"
fi

cargo build --target ./targets/$ARCH.json $PROFILE_OPTION

mkdir -p build/isoroot/boot/grub
cp target/$ARCH/$PROFILE/osmium build/isoroot/boot/osmium
strip build/isoroot/boot/osmium
cp grub/config.cfg build/isoroot/boot/grub/grub.cfg

grub-mkrescue -d /usr/lib/grub/i386-pc -o build/osmium.iso build/isoroot

# Set up a symbolic link so that it is easy to find the most recently built target directory.
rm -f build/target
ln -s `realpath target/$ARCH/$PROFILE` build/target
