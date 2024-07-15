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

use fast_trap::{FastContext, FastResult};
use rustsbi::RustSBI;

use constants::*;
use extension::SBI;
use riscv_spec::*;
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

extern "C" fn fast_handler(
    mut ctx: FastContext,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    a5: usize,
    a6: usize,
    a7: usize,
) -> FastResult {
    use riscv::register::{
        mcause::{self, Exception as E, Trap as T},
        mtval, sstatus,
    };

    #[inline]
    fn boot(mut ctx: FastContext, start_addr: usize, opaque: usize) -> FastResult {
        unsafe {
            sstatus::clear_sie();
        }
        ctx.regs().a[0] = riscv::register::mhartid::read();
        ctx.regs().a[1] = opaque;
        ctx.regs().pc = start_addr;
        ctx.call(2)
    }
    loop {
        match local_hsm().start() {
            Ok(supervisor) => {
                mstatus::update(|bits| {
                    *bits &= !mstatus::MPP;
                    *bits |= mstatus::MPIE | mstatus::MPP_SUPERVISOR;
                });
                mie::write(mie::MSIE | mie::MTIE);
                break boot(ctx, supervisor.start_addr, supervisor.opaque);
            }
            _ => match mcause::read().cause() {
                // SBI call
                T::Exception(E::SupervisorEnvCall) => {
                    use sbi_spec::{base, hsm, legacy};
                    let mut ret = SBI.handle_ecall(a7, a6, [ctx.a0(), a1, a2, a3, a4, a5]);
                    if ret.is_ok() {
                        match (a7, a6) {
                            // 关闭
                            (hsm::EID_HSM, hsm::HART_STOP) => continue,
                            // 不可恢复挂起
                            (hsm::EID_HSM, hsm::HART_SUSPEND)
                                if matches!(ctx.a0() as u32, hsm::suspend_type::NON_RETENTIVE) =>
                            {
                                break boot(ctx, a1, a2);
                            }
                            // legacy console 探测
                            (base::EID_BASE, base::PROBE_EXTENSION)
                                if matches!(
                                    ctx.a0(),
                                    legacy::LEGACY_CONSOLE_PUTCHAR | legacy::LEGACY_CONSOLE_GETCHAR
                                ) =>
                            {
                                ret.value = 1;
                            }
                            _ => {}
                        }
                    } else {
                        match a7 {
                            legacy::LEGACY_CONSOLE_PUTCHAR => {
                                print!("{}", ctx.a0() as u8 as char);
                                ret.error = 0;
                                ret.value = a1;
                            }
                            legacy::LEGACY_CONSOLE_GETCHAR => {
                                // let mut c = 0u8;
                            }
                            _ => {}
                        }
                    }
                    ctx.regs().a = [ret.error, ret.value, a2, a3, a4, a5, a6, a7];
                    mepc::next();
                    break ctx.restore();
                }
                // 其他陷入
                trap => {
                    println!(
                        "
-----------------------------
> trap:    {trap:?}
> mstatus: {:#018x}
> mepc:    {:#018x}
> mtval:   {:#018x}
-----------------------------
            ",
                        mstatus::read(),
                        mepc::read(),
                        mtval::read()
                    );
                    panic!("stopped with unsupported trap")
                }
            },
        }
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("[rustsbi-panic] hart {} {info}", riscv::register::mhartid::read());
    println!("[rustsbi-panic] system shutdown scheduled due to RustSBI panic");
    loop {}
}

extern "C" {
    fn _start();
}
