.code32

#define MULTIBOOT2_MAGIC 0xE85250D6
#define MULTIBOOT2_ARCHITECTURE 0

.section .multiboot
multiboot_header:
.long MULTIBOOT2_MAGIC /*Magic*/
.long MULTIBOOT2_ARCHITECTURE /*Architecture (i386)*/
.long multiboot_header_end - multiboot_header /*Header length*/
.long 0x100000000 - (MULTIBOOT2_MAGIC + MULTIBOOT2_ARCHITECTURE + (multiboot_header_end - multiboot_header)) /*Checksum*/
/*Tags*/

/*End tag*/
.align 8
.short 0 /*Type*/
.short 0 /*Flags*/
.long 8 /*Size*/
multiboot_header_end:

.section .bootstrap.text, "ax", @progbits
.globl _start
_start:
/*we must not trash ebx since it holds the mbi pointer*/

/*page tables:*/
mov $pml3, %eax
or $3, %eax
mov %eax, pml4

mov $pml2, %eax
or $3, %eax
mov %eax, pml3

/* Use huge page tables to make this easier */
mov $0x83, %eax
mov $512, %ecx
mov $pml2, %edi
1: mov %eax, (%edi)
add $0x200000, %eax
add $8, %edi
dec %ecx
jnz 1b

/*it's a bit of a hack, but since our virtual address starts in the last two gigabytes, the simplest way (that I can think of) to make this work is to add a new entry in the pml4 which points back to the same pml3, then set the second-last pml3 entry to point to the pml2.*/
mov $pml3, %eax
or $3, %eax
mov %eax, pml4 + (4096 - 8)
mov $pml2, %eax
or $3, %eax
mov %eax, pml3 + (4096 - 16)

mov $pml4, %eax
mov %eax, %cr3

/*now the three-step long mode transition*/
movl %cr4, %eax
orl $(1 << 5), %eax
movl %eax, %cr4

movl $0xC0000080, %ecx
rdmsr
orl $(1 << 8), %eax
wrmsr

movl %cr0, %eax
orl $(1 << 31), %eax
movl %eax, %cr0

lgdt low_gdt_pointer
jmp $8, $_start64

.code64
_start64:
lea high_gdt_pointer(%rip), %rax
lgdt (%rax)

add $KERNEL_VIRTUAL_OFFSET, %rbx /*remember that we have a physical address*/
mov %rbx, mbi_pointer

lea stack_end(%rip), %rsp
call kmain
1:
hlt
jmp 1b

.section .bootstrap.data, "aw", @progbits
low_gdt_pointer:
.word gdt_end - gdt - 1
.quad gdt - /*KERNEL_VIRTUAL_OFFSET*/ 0xffffffff80000000

.data
high_gdt_pointer:
.word gdt_end - gdt - 1
.quad gdt

#define GDT_EXECUTABLE (1 << 43)
#define GDT_NOT_SYSTEM (1 << 44)
#define GDT_PRESENT (1 << 47)
#define GDT_LONG (1 << 53)

gdt:
.quad 0
.quad GDT_PRESENT | GDT_LONG | GDT_NOT_SYSTEM | GDT_EXECUTABLE /*Kernel code segment*/
gdt_end:

.section .bootstrap.bss, "aw", @nobits
.align 4096
pml4:
.fill 4096
pml3:
.fill 4096
pml2:
.fill 4096

.bss
stack:
.fill 8192
stack_end:

.globl mbi_pointer
mbi_pointer:
.quad 0
