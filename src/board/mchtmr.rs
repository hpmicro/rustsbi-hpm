#![allow(unused)]

use riscv::register::mip;
use rustsbi::Timer;

use super::pac::mchtmr::Mchtmr;
use crate::println;

pub struct MachineTimer {
    inner: Mchtmr,
}

impl MachineTimer {
    pub fn new(mchtmr: Mchtmr) -> Self {
        Self { inner: mchtmr }
    }

    #[inline(always)]
    pub fn time(&self) -> u32 {
        self.inner.mtime().read() as u32
    }

    #[inline(always)]
    pub fn timeh(&self) -> u32 {
        (self.inner.mtime().read() >> 32) as u32
    }

    #[inline(always)]
    pub fn time64(&self) -> u64 {
        self.inner.mtime().read()
    }

    #[inline(always)]
    pub fn set_timecmp(&self, timecmp: u64) {
        self.inner.mtimecmp().write(|w| *w = timecmp);
        unsafe { mip::clear_stimer() }
    }
}

impl Timer for MachineTimer {
    #[inline(always)]
    fn set_timer(&self, stime_value: u64) {
        self.set_timecmp(stime_value);
    }
}
