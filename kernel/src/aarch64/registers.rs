use core::arch::asm;

pub fn get_esr() -> u64 {
    let mut esr: u64;
    unsafe {
        asm!("mrs {}, esr_el1", out(reg) esr, options(nomem, nostack));
    }
    esr
}

pub fn get_cntfrq() -> u64 {
    let mut cntfrq: u64;
    unsafe {
        asm!("mrs {}, cntfrq_el0", out(reg) cntfrq, options(nomem, nostack));
    }
    cntfrq
}

pub fn get_cntvct() -> u64 {
    let mut cntvct: u64;
    unsafe {
        asm!("mrs {}, cntvct_el0", out(reg) cntvct, options(nomem, nostack));
    }
    cntvct
}

pub fn set_cntv_ctl(cntv_ctl: u64) {
    unsafe {
        asm!("msr cntv_ctl_el0, {}", in(reg) cntv_ctl, options(nomem, nostack));
    }
}

pub fn set_cntv_cval(cntv_cval: u64) {
    unsafe {
        asm!("msr cntv_cval_el0, {}", in(reg) cntv_cval, options(nomem, nostack));
    }
}
