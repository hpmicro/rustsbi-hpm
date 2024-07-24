use fast_trap::{FastContext, FastResult};
use riscv::register::{
    mcause::{self, Exception as E, Interrupt as I, Trap as T},
    mip, mtval, scause, sepc, sstatus, stval, stvec,
};
use rustsbi::RustSBI;

use crate::extension::SBI;
use crate::local_hsm;
use crate::print;
use crate::riscv_spec::*;

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

#[inline]
fn delegate() {
    unsafe {
        sepc::write(mepc::read());
        scause::write(mcause::read().bits());
        stval::write(mtval::read());
        sstatus::clear_sie();
        if mstatus::read() & mstatus::MPP == mstatus::MPP_SUPERVISOR {
            sstatus::set_spp(sstatus::SPP::Supervisor);
        } else {
            sstatus::set_spp(sstatus::SPP::User);
        }
        mstatus::update(|bits| {
            *bits &= !mstatus::MPP;
            *bits |= mstatus::MPP_SUPERVISOR;
        });
        mepc::write(stvec::read().address());
    }
}

#[inline]
fn illegal_instruction_handler(ctx: &mut FastContext) -> bool {
    use riscv_decode::{decode, Instruction};

    let inst = decode(mtval::read() as u32);
    match inst {
        Ok(Instruction::Csrrs(csr)) => match csr.csr() {
            CSR_TIME => {
                assert!(
                    10 <= csr.rd() && csr.rd() <= 17,
                    "Unsupported CSR rd: {}",
                    csr.rd()
                );
                ctx.regs().a[(csr.rd() - 10) as usize] = SBI.timer.time() as usize;
            }
            CSR_TIMEH => {
                assert!(
                    10 <= csr.rd() && csr.rd() <= 17,
                    "Unsupported CSR rd: {}",
                    csr.rd()
                );
                ctx.regs().a[(csr.rd() - 10) as usize] = SBI.timer.timeh() as usize;
            }
            _ => return false,
        },
        _ => return false,
    }
    mepc::next();
    true
}

#[no_mangle]
#[link_section = ".trap"]
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
                            _ => (),
                        }
                    } else {
                        match a7 {
                            legacy::LEGACY_CONSOLE_PUTCHAR => {
                                print!("{}", ctx.a0() as u8 as char);
                                ret.error = 0;
                                ret.value = a1;
                            }
                            legacy::LEGACY_CONSOLE_GETCHAR => unimplemented!(),
                            _ => unimplemented!(
                                "EID: {:#010x} FID: {:#010x} is not implemented!",
                                a7,
                                a6
                            ),
                        }
                    }
                    ctx.regs().a = [ret.error, ret.value, a2, a3, a4, a5, a6, a7];
                    mepc::next();
                    break ctx.restore();
                }
                T::Exception(E::IllegalInstruction) => {
                    if mstatus::read() & mstatus::MPP == mstatus::MPP_MACHINE {
                        panic!("Illegal instruction exception from M-MODE");
                    }
                    ctx.regs().a = [ctx.a0(), a1, a2, a3, a4, a5, a6, a7];
                    if !illegal_instruction_handler(&mut ctx) {
                        delegate();
                    }
                    break ctx.restore();
                }
                T::Interrupt(I::MachineTimer) => {
                    ctx.regs().a = [ctx.a0(), a1, a2, a3, a4, a5, a6, a7];
                    SBI.timer.set_timecmp(u64::MAX);
                    unsafe {
                        mip::set_stimer();
                    }
                    break ctx.restore();
                }
                // 其他陷入
                trap => {
                    panic!("stopped with unsupported {trap:?}")
                }
            },
        }
    }
}
