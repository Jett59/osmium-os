#!/bin/sh

set -e

echo "Launching qemu..."
exec qemu-system-x86_64 -cdrom build/osmium.iso -no-reboot -no-shutdown $@
