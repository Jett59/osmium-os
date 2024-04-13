#!/bin/sh

set +x
set -e

mkdir -p build/initial_ramdisk
cp -r user/build/* build/initial_ramdisk

cd build/initial_ramdisk
tar -cf ../initial_ramdisk.tar *
cd ..
