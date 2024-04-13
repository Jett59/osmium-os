use bitflags::bitflags;
use core::arch::{asm, global_asm};

use crate::{arch::local_apic, lazy_init::lazy_static, print, println};

bitflags! {
    struct IdtFlags: u8 {
        const PRESENT = 1 << 7;
        const INTERRUPT_GATE = 0xE;
        const TRAP_GATE = 0xF;
        const KERNEL_PRIVILEGE = 0 << 5;
        const USER_PRIVILEGE = 3 << 5;
    }
}

#[repr(C, packed)]
struct IdtEntry {
    offset_low: u16,
    selector: u16,
    ist: u8,
    flags: IdtFlags,
    offset_middle: u16,
    offset_high: u32,
    zero: u32,
}

impl IdtEntry {
    const fn new(offset: u64, trap: bool) -> Self {
        Self {
            offset_low: offset as u16,
            selector: 0x08,
            ist: 0,
            flags: IdtFlags::from_bits_truncate(
                IdtFlags::PRESENT.bits()
                    | if trap {
                        IdtFlags::TRAP_GATE
                    } else {
                        IdtFlags::INTERRUPT_GATE
                    }
                    .bits()
                    | IdtFlags::USER_PRIVILEGE.bits(),
            ),
            offset_middle: (offset >> 16) as u16,
            offset_high: (offset >> 32) as u32,
            zero: 0,
        }
    }
}

macro_rules! asm_interrupt_handler {
    ($function_name:ident, $number:expr, $handler:ident, $error_code_length:expr) => {
            global_asm!(
                stringify!(.globl $function_name),
                stringify!(.type $function_name, @function),
                stringify!($function_name:),
                "push rax
                 push rcx
                 push rdx
                 push rbx
                 push rsi
                 push rdi
                 push rbp
                 push r8
                 push r9
                 push r10
                 push r11
                 push r12
                 push r13
                 push r14
                 push r15
                 mov rsi, rsp
                 mov rdi, {number}
                 call {handler}
                 pop r15
                 pop r14
                 pop r13
                 pop r12
                 pop r11
                 pop r10
                 pop r9
                 pop r8
                    pop rbp
                 pop rdi
                 pop rsi
                    pop rbx
                 pop rdx
                 pop rcx
                 pop rax
                 sub rsp, {error_code_length}
                 iretq",
                number = const $number,
                handler = sym $handler,
                error_code_length = const $error_code_length,
            );
            extern "C" {
                static $function_name: u8;
            }
    };
}

// The first 32 are CPU exceptions:
asm_interrupt_handler!(h0, 0, divide_by_zero, 0);
asm_interrupt_handler!(h1, 1, debug, 0);
asm_interrupt_handler!(h2, 2, non_maskable_interrupt, 0);
asm_interrupt_handler!(h3, 3, breakpoint, 0);
asm_interrupt_handler!(h4, 4, overflow, 0);
asm_interrupt_handler!(h5, 5, bound_range_exceeded, 0);
asm_interrupt_handler!(h6, 6, invalid_opcode, 0);
asm_interrupt_handler!(h7, 7, device_not_available, 0);
asm_interrupt_handler!(h8, 8, double_fault, 8);
asm_interrupt_handler!(h9, 9, coprocessor_segment_overrun, 0);
asm_interrupt_handler!(h10, 10, invalid_tss, 8);
asm_interrupt_handler!(h11, 11, segment_not_present, 8);
asm_interrupt_handler!(h12, 12, stack_segment_fault, 8);
asm_interrupt_handler!(h13, 13, general_protection_fault, 8);
asm_interrupt_handler!(h14, 14, page_fault, 8);
asm_interrupt_handler!(h15, 15, reserved, 0);
asm_interrupt_handler!(h16, 16, x87_floating_point, 0);
asm_interrupt_handler!(h17, 17, alignment_check, 8);
asm_interrupt_handler!(h18, 18, machine_check, 0);
asm_interrupt_handler!(h19, 19, simd_floating_point, 0);
asm_interrupt_handler!(h20, 20, virtualization, 0);
asm_interrupt_handler!(h21, 21, reserved, 0);
asm_interrupt_handler!(h22, 22, reserved, 0);
asm_interrupt_handler!(h23, 23, reserved, 0);
asm_interrupt_handler!(h24, 24, reserved, 0);
asm_interrupt_handler!(h25, 25, reserved, 0);
asm_interrupt_handler!(h26, 26, reserved, 0);
asm_interrupt_handler!(h27, 27, reserved, 0);
asm_interrupt_handler!(h28, 28, reserved, 0);
asm_interrupt_handler!(h29, 29, reserved, 0);
asm_interrupt_handler!(h30, 30, security_exception, 8);
asm_interrupt_handler!(h31, 31, reserved, 0);

macro_rules! asm_interrupt_handlers {
    ($($number:expr, $function_name:ident),* $(,)?) => {
        $(
            asm_interrupt_handler!($function_name, $number, handle_interrupt, 0);
        )*
    };
}

// I would really like something better than this, but declaritive macros just aren't powerful enough. Fortunately we can put it all on one line so it doesn't get in the way too much.
asm_interrupt_handlers! {
    32, h32, 33, h33, 34, h34, 35, h35, 36, h36, 37, h37, 38, h38, 39, h39, 40, h40, 41, h41, 42, h42, 43, h43, 44, h44, 45, h45, 46, h46, 47, h47, 48, h48, 49, h49, 50, h50, 51, h51, 52, h52, 53, h53, 54, h54, 55, h55, 56, h56, 57, h57, 58, h58, 59, h59, 60, h60, 61, h61, 62, h62, 63, h63, 64, h64, 65, h65, 66, h66, 67, h67, 68, h68, 69, h69, 70, h70, 71, h71, 72, h72, 73, h73, 74, h74, 75, h75, 76, h76, 77, h77, 78, h78, 79, h79, 80, h80, 81, h81, 82, h82, 83, h83, 84, h84, 85, h85, 86, h86, 87, h87, 88, h88, 89, h89, 90, h90, 91, h91, 92, h92, 93, h93, 94, h94, 95, h95, 96, h96, 97, h97, 98, h98, 99, h99, 100, h100, 101, h101, 102, h102, 103, h103, 104, h104, 105, h105, 106, h106, 107, h107, 108, h108, 109, h109, 110, h110, 111, h111, 112, h112, 113, h113, 114, h114, 115, h115, 116, h116, 117, h117, 118, h118, 119, h119, 120, h120, 121, h121, 122, h122, 123, h123, 124, h124, 125, h125, 126, h126, 127, h127, 128, h128, 129, h129, 130, h130, 131, h131, 132, h132, 133, h133, 134, h134, 135, h135, 136, h136, 137, h137, 138, h138, 139, h139, 140, h140, 141, h141, 142, h142, 143, h143, 144, h144, 145, h145, 146, h146, 147, h147, 148, h148, 149, h149, 150, h150, 151, h151, 152, h152, 153, h153, 154, h154, 155, h155, 156, h156, 157, h157, 158, h158, 159, h159, 160, h160, 161, h161, 162, h162, 163, h163, 164, h164, 165, h165, 166, h166, 167, h167, 168, h168, 169, h169, 170, h170, 171, h171, 172, h172, 173, h173, 174, h174, 175, h175, 176, h176, 177, h177, 178, h178, 179, h179, 180, h180, 181, h181, 182, h182, 183, h183, 184, h184, 185, h185, 186, h186, 187, h187, 188, h188, 189, h189, 190, h190, 191, h191, 192, h192, 193, h193, 194, h194, 195, h195, 196, h196, 197, h197, 198, h198, 199, h199, 200, h200, 201, h201, 202, h202, 203, h203, 204, h204, 205, h205, 206, h206, 207, h207, 208, h208, 209, h209, 210, h210, 211, h211, 212, h212, 213, h213, 214, h214, 215, h215, 216, h216, 217, h217, 218, h218, 219, h219, 220, h220, 221, h221, 222, h222, 223, h223, 224, h224, 225, h225, 226, h226, 227, h227, 228, h228, 229, h229, 230, h230, 231, h231, 232, h232, 233, h233, 234, h234, 235, h235, 236, h236, 237, h237, 238, h238, 239, h239, 240, h240, 241, h241, 242, h242, 243, h243, 244, h244, 245, h245, 246, h246, 247, h247, 248, h248, 249, h249, 250, h250, 251, h251, 252, h252, 253, h253, 254, h254, 255, h255,
}

#[repr(C)]
#[derive(Debug)]
struct SavedRegisters {
    // It's important that we get the order right. Remember that we pushed r15 last, so it is the first.
    r15: u64,
    r14: u64,
    r13: u64,
    r12: u64,
    r11: u64,
    r10: u64,
    r9: u64,
    r8: u64,
    rbp: u64,
    rdi: u64,
    rsi: u64,
    rbx: u64,
    rdx: u64,
    rcx: u64,
    rax: u64,
}

macro_rules! unhandled_interrupt {
    ($function_name:ident, $interrupt_name:expr) => {
        #[no_mangle]
        extern "C" fn $function_name(number: u64, saved_registers: &SavedRegisters) {
            panic!(
                "Unhandled interrupt: {} (0x{:x})\n{:x?}",
                $interrupt_name, number, saved_registers
            );
        }
    };
}

macro_rules! unhandled_interrupts {
    ($($function_name:ident $interrupt_name:expr),* $(,)?) => {
        $(
            unhandled_interrupt!($function_name, $interrupt_name);
        )*
    };
}

unhandled_interrupts!(divide_by_zero "divide by zero", debug "debug", non_maskable_interrupt "non maskable interrupt", breakpoint "breakpoint", overflow "overflow", bound_range_exceeded "bound range exceeded", device_not_available "device not available", invalid_opcode "invalid opcode", double_fault "double fault", coprocessor_segment_overrun "coprocessor segment overrun", invalid_tss "invalid tss", segment_not_present "segment not present", stack_segment_fault "stack segment fault", general_protection_fault "general protection fault", page_fault "page fault", reserved "reserved exception", x87_floating_point "x87 floating point", alignment_check "alignment check", machine_check "machine check", simd_floating_point "simd floating point", virtualization "virtualization", security_exception "security exception");

pub const TIMER_INTERRUPT: u8 = 0x20;

pub const SPURIOUS_INTERRUPT_VECTOR: u8 = 0xFF;

fn handle_interrupt(number: u64, saved_registers: &SavedRegisters) {
    if number == SPURIOUS_INTERRUPT_VECTOR as u64 {
        return;
    }
    // Testing code to make sure the timer IRQ is working properly.
    if number == TIMER_INTERRUPT as u64 {
        print!(".");
        unsafe { local_apic::set_timer(local_apic::get_timer_frequency()) };
        unsafe { local_apic::end_of_interrupt() };
        return;
    }

    println!("Interrupt: {}", number);
    println!("Saved registers: {:?}", saved_registers);
}

macro_rules! idt {
    ($($function_name:ident $trap:expr),* $(,)?) => {
        [
            $(
                unsafe {IdtEntry::new(&$function_name as *const _ as u64, $trap)},
            )*
        ]
    };
}

lazy_static! {
    static ref IDT: [IdtEntry; 256] = idt! {
        h0 true, h1 true, h2 true, h3 true, h4 true, h5 true, h6 true, h7 true, h8 true, h9 true, h10 true, h11 true, h12 true, h13 true, h14 true, h15 true, h16 true, h17 true, h18 true, h19 true, h20 true, h21 true, h22 true, h23 true, h24 true, h25 true, h26 true, h27 true, h28 true, h29 true, h30 true, h31 true, h32 false, h33 false, h34 false, h35 false, h36 false, h37 false, h38 false, h39 false, h40 false, h41 false, h42 false, h43 false, h44 false, h45 false, h46 false, h47 false, h48 false, h49 false, h50 false, h51 false, h52 false, h53 false, h54 false, h55 false, h56 false, h57 false, h58 false, h59 false, h60 false, h61 false, h62 false, h63 false, h64 false, h65 false, h66 false, h67 false, h68 false, h69 false, h70 false, h71 false, h72 false, h73 false, h74 false, h75 false, h76 false, h77 false, h78 false, h79 false, h80 false, h81 false, h82 false, h83 false, h84 false, h85 false, h86 false, h87 false, h88 false, h89 false, h90 false, h91 false, h92 false, h93 false, h94 false, h95 false, h96 false, h97 false, h98 false, h99 false, h100 false, h101 false, h102 false, h103 false, h104 false, h105 false, h106 false, h107 false, h108 false, h109 false, h110 false, h111 false, h112 false, h113 false, h114 false, h115 false, h116 false, h117 false, h118 false, h119 false, h120 false, h121 false, h122 false, h123 false, h124 false, h125 false, h126 false, h127 false, h128 false, h129 false, h130 false, h131 false, h132 false, h133 false, h134 false, h135 false, h136 false, h137 false, h138 false, h139 false, h140 false, h141 false, h142 false, h143 false, h144 false, h145 false, h146 false, h147 false, h148 false, h149 false, h150 false, h151 false, h152 false, h153 false, h154 false, h155 false, h156 false, h157 false, h158 false, h159 false, h160 false, h161 false, h162 false, h163 false, h164 false, h165 false, h166 false, h167 false, h168 false, h169 false, h170 false, h171 false, h172 false, h173 false, h174 false, h175 false, h176 false, h177 false, h178 false, h179 false, h180 false, h181 false, h182 false, h183 false, h184 false, h185 false, h186 false, h187 false, h188 false, h189 false, h190 false, h191 false, h192 false, h193 false, h194 false, h195 false, h196 false, h197 false, h198 false, h199 false, h200 false, h201 false, h202 false, h203 false, h204 false, h205 false, h206 false, h207 false, h208 false, h209 false, h210 false, h211 false, h212 false, h213 false, h214 false, h215 false, h216 false, h217 false, h218 false, h219 false, h220 false, h221 false, h222 false, h223 false, h224 false, h225 false, h226 false, h227 false, h228 false, h229 false, h230 false, h231 false, h232 false, h233 false, h234 false, h235 false, h236 false, h237 false, h238 false, h239 false, h240 false, h241 false, h242 false, h243 false, h244 false, h245 false, h246 false, h247 false, h248 false, h249 false, h250 false, h251 false, h252 false, h253 false, h254 false, h255 false,
    };
}

#[repr(C, packed)]
struct Idtr {
    limit: u16,
    base: u64,
}

pub fn init() {
    unsafe {
        let idtr = Idtr {
            limit: (core::mem::size_of::<[IdtEntry; 256]>() - 1) as u16,
            base: IDT.as_ptr() as u64,
        };
        asm!("lidt [{}]", in(reg) &idtr);
    }
}
