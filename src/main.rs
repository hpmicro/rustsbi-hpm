#![no_std]
#![no_main]
#![deny(warnings)]
mod board;
mod constants {
    /// 特权软件入口。
    pub(crate) const SUPERVISOR_ENTRY: usize = 0x8020_0000;
}

use constants::*;
use riscv::register::mhartid;

extern crate panic_halt;

#[hpm_rt::entry]
fn main() -> ! {
    extern "C" {
        fn _start();
    }
    let hartid = mhartid::read();

    board::board_init();

    // Print startup messages
    print!(
        "\
[rustsbi] RustSBI version {ver_sbi}, adapting to RISC-V SBI v2.0.0
{logo}
[rustsbi] Implementation     : RustSBI-HPM Version {ver_impl}
[rustsbi] Platform Name      : {model}
[rustsbi] Boot HART          : {hartid}
[rustsbi] Firmware Address   : {firmware:#x}
[rustsbi] Supervisor Address : {SUPERVISOR_ENTRY:#x}
",
        ver_sbi = rustsbi::VERSION,
        logo = rustsbi::LOGO,
        ver_impl = env!("CARGO_PKG_VERSION"),
        model = "HPM6750EVKMINI",
        firmware = _start as usize,
    );

    loop {}
}
