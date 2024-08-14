#![no_std]
#![no_main]
#![feature(naked_functions, asm_const)]
#![deny(warnings)]

mod board;
mod extension;
mod loader;
mod pmp;
mod riscv_spec;
mod trap;
mod trap_stack;
mod constants {
    /// 特权软件入口。
    pub(crate) const SUPERVISOR_ENTRY: usize = 0x4000_0000;
    /// 设备树加载地址。
    pub(crate) const DTB_LOAD_ADDRESS: usize = 0x4030_0000;
    /// 每个硬件线程设置 16KiB 栈空间。
    pub(crate) const LEN_STACK_PER_HART: usize = 16 * 1024;
}

use core::arch::asm;
use riscv::register::{mcause, mtval};

use constants::*;
use riscv_spec::*;
use trap_stack::local_hsm;

/// 特权软件信息。
#[derive(Debug)]
struct Supervisor {
    start_addr: usize,
    opaque: usize,
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
[rustsbi] Devicetree Address : {DTB_LOAD_ADDRESS:#010x}
",
        rustsbi_version = rustsbi::VERSION,
        logo = rustsbi::LOGO,
        impl_version = env!("CARGO_PKG_VERSION"),
        model = "HPM6360EVK",
        firmware_address = _start as usize,
    );
    // 初始化 PMP
    pmp::set_pmp();
    // 显示 PMP 配置
    pmp::print_pmps();
    // 设置陷入栈
    trap_stack::prepare_for_trap();
    unsafe {
        // 加载内核
        loader::load_kernel();
        // 加载设备树
        loader::load_dtb()
    };
    // 设置内核入口
    local_hsm().prepare(Supervisor {
        start_addr: SUPERVISOR_ENTRY,
        opaque: DTB_LOAD_ADDRESS,
    });
    // 准备启动调度
    println!("\nStarting kernel ...\n");
    unsafe {
        asm!("csrw mideleg,    {}", in(reg) !0);
        asm!("csrw medeleg,    {}", in(reg) !0);
        asm!("csrw mcounteren, {}", in(reg) !0);
        use riscv::register::{medeleg, mtvec};
        medeleg::clear_supervisor_env_call();
        medeleg::clear_illegal_instruction();
        medeleg::clear_machine_env_call();
        medeleg::clear_store_fault();
        medeleg::clear_load_fault();
        mtvec::write(fast_trap::trap_entry as _, mtvec::TrapMode::Direct);
        asm!("j {trap_handler}",
            trap_handler = sym fast_trap::trap_entry,
            options(noreturn),
        );
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!(
        "[rustsbi-panic] hart {} {info}",
        riscv::register::mhartid::read()
    );
    println!(
        "-----------------------------
> mcause:  {:?}
> mdcause: {:#010x}
> mstatus: {:#010x}
> mepc:    {:#010x}
> mtval:   {:#010x}
-----------------------------",
        mcause::read().cause(),
        mdcause::read(),
        mstatus::read(),
        mepc::read(),
        mtval::read()
    );
    println!("[rustsbi-panic] system shutdown scheduled due to RustSBI panic");
    loop {}
}

extern "C" {
    fn _start();
}
