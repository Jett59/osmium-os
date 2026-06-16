#!/bin/sh

set -e
set +x

mkdir -p build/isoroot/boot/grub
cp kernel/build/osmium build/isoroot/boot/osmium
cp build/initial_ramdisk.tar build/isoroot/boot/initial_ramdisk.tar
cp grub/config.cfg build/isoroot/boot/grub/grub.cfg

# Check if docker image exists, if not build it
if ! docker image inspect osmium-iso-builder:latest > /dev/null 2>&1; then
    echo "Docker image 'osmium-builder:latest' not found. Building it now..."
    docker build --platform linux/amd64 -t osmium-iso-builder:latest -f build-x86_64.Dockerfile .
else
    echo "Docker image 'osmium-builder:latest' found. Using existing image."
fi

docker run --platform linux/amd64 --rm -v "$(pwd)/build:/build" osmium-iso-builder:latest
