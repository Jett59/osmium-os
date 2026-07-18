use core::arch::asm;

use crate::unsafe_impl::init_cell::NoConcurrency;

#[inline(always)]
pub fn memory_barrier() {
    unsafe {
        asm!("dmb sy");
    }
}

#[inline(always)]
pub fn isb() {
    unsafe { asm!("isb", options(nomem, nostack)) }
}

#[inline(always)]
pub fn dsb_ish() {
    unsafe { asm!("dsb ish", options(nomem, nostack)) }
}

#[inline(always)]
pub unsafe fn write_ttbr0(ttbr0: u64) {
    unsafe { asm!("msr ttbr0_el1, {}", in(reg) ttbr0, options(nomem, nostack)) }
}

#[inline(always)]
pub fn yield_instruction() {
    unsafe { asm!("yield", options(nomem, nostack)) }
}

#[inline(always)]
pub fn enable_interrupts(_no_concurrency: NoConcurrency) {
    // SAFETY: by taking NoConcurrency by-value, we ensure that all future code is concurrency-safe.
    unsafe { asm!("msr daifclr, #15", options(nomem, nostack)) }
}

#[inline(always)]
pub unsafe fn eret(elr: u64, spsr: u64) -> ! {
    unsafe {
        asm!("msr elr_el1, {}", "msr spsr_el1, {}", "eret", in(reg) elr, in(reg) spsr, options(nomem, nostack, noreturn));
    }
}

#[inline(always)]
pub fn get_esr() -> u64 {
    let mut esr: u64;
    unsafe {
        asm!("mrs {}, esr_el1", out(reg) esr, options(nomem, nostack));
    }
    esr
}

#[inline(always)]
pub fn get_cntfrq() -> u64 {
    let mut cntfrq: u64;
    unsafe {
        asm!("mrs {}, cntfrq_el0", out(reg) cntfrq, options(nomem, nostack));
    }
    cntfrq
}

#[inline(always)]
pub fn get_cntvct() -> u64 {
    let mut cntvct: u64;
    unsafe {
        asm!("mrs {}, cntvct_el0", out(reg) cntvct, options(nomem, nostack));
    }
    cntvct
}

#[inline(always)]
pub fn set_cntv_ctl(mask: bool, enable: bool) {
    let cntv_ctl: u64 = ((mask as u64) << 1) | (enable as u64);
    unsafe {
        asm!("msr cntv_ctl_el0, {}", in(reg) cntv_ctl, options(nomem, nostack));
    }
}

#[inline(always)]
pub fn set_cntv_cval(cntv_cval: u64) {
    unsafe {
        asm!("msr cntv_cval_el0, {}", in(reg) cntv_cval, options(nomem, nostack));
    }
}
