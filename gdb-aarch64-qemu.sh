#!/bin/sh

./qemu-aarch64.sh -S -s &
QEMU_PID=$!
sleep 5 # It takes a little while to start qemu.

gdb-multiarch -x aarch64-kernel.gdb

echo "Killing $QEMU_PID"
kill $QEMU_PID
