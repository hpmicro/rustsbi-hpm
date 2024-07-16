use fast_trap::{FastContext, FastResult};
use rustsbi::RustSBI;

use crate::extension::SBI;
use crate::local_hsm;
use crate::riscv_spec::*;
use crate::{print, println};

pub extern "C" fn fast_handler(
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
                            _ => unimplemented!(),
                        }
                    } else {
                        match a7 {
                            legacy::LEGACY_CONSOLE_PUTCHAR => {
                                print!("{}", ctx.a0() as u8 as char);
                                ret.error = 0;
                                ret.value = a1;
                            }
                            legacy::LEGACY_CONSOLE_GETCHAR => unimplemented!(),
                            _ => unimplemented!(),
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

// machine timer 中断代理
//
// # Safety
//
// 裸函数。
// #[naked]
// unsafe extern "C" fn mtimer() {
//     asm!(
//         // 换栈：
//         // sp      : M sp
//         // mscratch: S sp
//         "   csrrw sp, mscratch, sp",
//         // 保护
//         "   addi  sp, sp, -4*8
//             sd    ra, 0*8(sp)
//             sd    a0, 1*8(sp)
//             sd    a1, 2*8(sp)
//             sd    a2, 3*8(sp)
//         ",
//         // 清除 mtimecmp
//         "   la    a0, {clint_ptr}
//             ld    a0, (a0)
//             csrr  a1, mhartid
//             addi  a2, zero, -1
//             call  {set_mtimecmp}
//         ",
//         // 设置 stip
//         "   li    a0, {mip_stip}
//             csrrs zero, mip, a0
//         ",
//         // 恢复
//         "   ld    ra, 0*8(sp)
//             ld    a0, 1*8(sp)
//             ld    a1, 2*8(sp)
//             ld    a2, 3*8(sp)
//             addi  sp, sp,  4*8
//         ",
//         // 换栈：
//         // sp      : S sp
//         // mscratch: M sp
//         "   csrrw sp, mscratch, sp",
//         // 返回
//         "   mret",
//         mip_stip     = const (1 << 5),
//         clint_ptr    =   sym CLINT,
//         //                   Clint::write_mtimecmp_naked(&self, hart_idx, val)
//         set_mtimecmp =   sym Clint::write_mtimecmp_naked,
//         options(noreturn)
//     )
// }

// machine soft 中断代理
//
// # Safety
//
// 裸函数。
// #[naked]
// unsafe extern "C" fn msoft() {
//     asm!(
//         // 换栈：
//         // sp      : M sp
//         // mscratch: S sp
//         "   csrrw sp, mscratch, sp",
//         // 保护
//         "   addi sp, sp, -3*8
//             sd   ra, 0*8(sp)
//             sd   a0, 1*8(sp)
//             sd   a1, 2*8(sp)
//         ",
//         // 清除 msip 设置 ssip
//         "   la   a0, {clint_ptr}
//             ld   a0, (a0)
//             csrr a1, mhartid
//             call {clear_msip}
//             csrrsi zero, mip, 1 << 1
//         ",
//         // 恢复
//         "   ld   ra, 0*8(sp)
//             ld   a0, 1*8(sp)
//             ld   a1, 2*8(sp)
//             addi sp, sp,  3*8
//         ",
//         // 换栈：
//         // sp      : S sp
//         // mscratch: M sp
//         "   csrrw sp, mscratch, sp",
//         // 返回
//         "   mret",
//         clint_ptr  = sym CLINT,
//         //               Clint::clear_msip_naked(&self, hart_idx)
//         clear_msip = sym Clint::clear_msip_naked,
//         options(noreturn)
//     )
// }
