#![allow(unused)]

use core::{convert::Infallible, mem::MaybeUninit};

use rustsbi::{spec::binary::SbiRet, HartMask, RustSBI};

static mut SBI: MaybeUninit<FixedRustSBI> = MaybeUninit::uninit();

pub(crate) struct Impl;
pub(crate) type FixedRustSBI<'a> =
    RustSBI<&'a Impl, Infallible, Infallible, Infallible, &'a Impl, Infallible>;

pub(crate) fn init() {
    unsafe {
        SBI = MaybeUninit::new(
            rustsbi::Builder::new_machine()
                .with_timer(&Impl)
                .with_reset(&Impl)
                .build(),
        )
    }
}

#[inline]
pub(crate) fn sbi<'a>() -> &'static mut FixedRustSBI<'a> {
    unsafe { SBI.assume_init_mut() }
}

impl rustsbi::Timer for Impl {
    fn set_timer(&self, stime_value: u64) {
        unimplemented!()
    }
}

impl rustsbi::Reset for Impl {
    fn system_reset(&self, _reset_type: u32, _reset_reason: u32) -> SbiRet {
        unimplemented!()
    }
}
