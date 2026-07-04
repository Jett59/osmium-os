#!/bin/sh

set +x
set -e

mkdir -p build/initial_ramdisk
cp -r user/build/* build/initial_ramdisk

cd build/initial_ramdisk
COPYFILE_DISABLE=1 tar --no-xattrs --format ustar -cf ../initial_ramdisk.tar *
cd ..
