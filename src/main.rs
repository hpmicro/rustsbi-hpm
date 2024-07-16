#![no_std]
#![no_main]
#![feature(naked_functions, asm_const)]
#![deny(warnings)]

mod board;
mod extension;
mod pmp;
mod riscv_spec;
mod trap;
mod trap_stack;
mod constants {
    /// 特权软件入口。
    pub(crate) const SUPERVISOR_ENTRY: usize = 0x0108_0000;
    /// 每个硬件线程设置 16KiB 栈空间。
    pub(crate) const LEN_STACK_PER_HART: usize = 16 * 1024;
}

use core::arch::asm;

use constants::*;
use trap_stack::local_hsm;

const TEST_KERNEL: &'static [u8] = include_bytes!("kernel.bin");

/// 特权软件信息。
#[derive(Debug)]
struct Supervisor {
    start_addr: usize,
    opaque: usize,
}

fn load_test_kernel() {
    let dst: &mut [u8] =
        unsafe { core::slice::from_raw_parts_mut(SUPERVISOR_ENTRY as *mut u8, TEST_KERNEL.len()) };
    dst.copy_from_slice(TEST_KERNEL);
}

#[hpm_rt::entry]
fn main() -> ! {
    let hartid = riscv::register::mhartid::read();

    board::board_init();

    // Print startup messages
    print!(
        "\
[rustsbi] RustSBI version {rustsbi_version}, adapting to RISC-V SBI v2.0.0
{logo}
[rustsbi] Implementation     : RustSBI-HPM Version {impl_version}
[rustsbi] Platform Name      : {model}
[rustsbi] Boot HART          : {hartid}
[rustsbi] Firmware Address   : {firmware_address:#010x}
[rustsbi] Supervisor Address : {SUPERVISOR_ENTRY:#010x}
",
        rustsbi_version = rustsbi::VERSION,
        logo = rustsbi::LOGO,
        impl_version = env!("CARGO_PKG_VERSION"),
        model = "HPM6360EVK",
        firmware_address = _start as usize,
    );
    // 初始化 PMP
    set_pmp();
    // 显示 PMP 配置
    pmp::print_pmps();
    // 设置陷入栈
    trap_stack::prepare_for_trap();
    // 加载内核
    load_test_kernel();
    // 设置内核入口
    local_hsm().prepare(Supervisor {
        start_addr: SUPERVISOR_ENTRY,
        opaque: Default::default(),
    });
    // 准备启动调度
    unsafe {
        asm!("csrw mideleg,    {}", in(reg) !0);
        asm!("csrw medeleg,    {}", in(reg) !0);
        asm!("csrw mcounteren, {}", in(reg) !0);
        use riscv::register::{medeleg, mtvec};
        medeleg::clear_supervisor_env_call();
        medeleg::clear_machine_env_call();
        mtvec::write(fast_trap::trap_entry as _, mtvec::TrapMode::Direct);
        asm!("j {trap_handler}",
            trap_handler = sym fast_trap::trap_entry,
            options(noreturn),
        );
    }
}

/// 设置 PMP。
fn set_pmp() {
    use riscv::register::*;
    unsafe {
        // All memory RWX
        pmpcfg0::set_pmp(0, Range::OFF, Permission::NONE, false);
        pmpaddr0::write((_start as usize) >> 2);
        pmpcfg0::set_pmp(1, Range::TOR, Permission::NONE, false);
        pmpaddr1::write((0x01080000) >> 2);
        pmpcfg0::set_pmp(2, Range::TOR, Permission::RWX, false);
        pmpaddr2::write((0x01100000) >> 2);
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!(
        "[rustsbi-panic] hart {} {info}",
        riscv::register::mhartid::read()
    );
    println!("[rustsbi-panic] system shutdown scheduled due to RustSBI panic");
    loop {}
}

extern "C" {
    fn _start();
}
