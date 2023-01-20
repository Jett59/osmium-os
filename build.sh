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

./build-$ARCH.sh
