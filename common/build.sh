#!/bin/sh

set +x
set -e

cargo build --target $ARCH $PROFILE_OPTION -Zbuild-std=core,alloc
