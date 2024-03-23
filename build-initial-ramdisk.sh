#!/bin/sh

set +x
set -e

mkdir -p build/initial_ramdisk
cp -RT user/build build/initial_ramdisk

cd build/initial_ramdisk
tar -cf ../initial_ramdisk.tar *
cd ..
