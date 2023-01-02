file build/target/osmium
target remote | qemu-system-x86_64 -S -gdb stdio -cdrom build/osmium.iso -no-reboot -no-shutdown -D qemu.log -d int
