#!/bin/sh

set +x
set -e

if [ -z $SOURCED_CONFIG ]; then
    echo "Warning: .config not sourced into current shell. Doing it now."
    echo "Run source .config to prevent this warning."
    . ./.config
fi

# Find the command line option for the given $PROFILE (debug should have no option, otherwise --$PROFILE)
PROFILE_OPTION=""

if [ "$PROFILE" != "debug" ]; then
    PROFILE_OPTION="--$PROFILE"
fi

export PROFILE_OPTION

cd kernel
./build.sh
cd ..

./build-$ARCH.sh
