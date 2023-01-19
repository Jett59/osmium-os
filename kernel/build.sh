#!/bin/sh

set +x
set -e

cargo build --target ./targets/$ARCH.json $PROFILE_OPTION -Zbuild-std=core,alloc

mkdir -p build

# Set up a symbolic link so that it is easy to find the most recently built target directory.
rm -f build/target
ln -s `realpath target/$ARCH/$PROFILE` build/target

cp build/target/osmium build/osmium
strip build/osmium
