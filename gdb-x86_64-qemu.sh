#!/bin/sh

./qemu-x86_64.sh -S -s $@ &
QEMU_PID=$!
sleep 5 # It takes a little while to start qemu.

gdb-multiarch -x x86_64-kernel.gdb

echo "Killing $QEMU_PID"
kill $QEMU_PID
