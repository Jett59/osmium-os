SOURCES=$(wildcard $(ARCH)/**/*.S $(ARCH)/*.S)
OBJECTS=$(SOURCES:.S=.o)

AS=clang -target $(ARCH)-unknown-elf
ASFLAGS=-g

libasm-$(ARCH).a: $(OBJECTS) Makefile
	llvm-ar r $@ $(OBJECTS)

%.o: %.S Makefile
	$(AS) $(ASFLAGS) -c -o $@ $<
