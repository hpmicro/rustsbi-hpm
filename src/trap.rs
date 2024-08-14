use fast_trap::{EntireContext, EntireContextSeparated, EntireResult, FastContext, FastResult};
use riscv::register::{
    mcause::{self, Exception as E, Interrupt as I, Trap as T},
    mip, mtval, scause, sepc, sstatus, stval, stvec,
};
use riscv_decode::{decode, Instruction};
use rustsbi::RustSBI;

use crate::extension::SBI;
use crate::local_hsm;
use crate::riscv_spec::*;
use crate::{board, print};

static mut S_LR_ADDR: usize = 0;
/// `csrrw zero, time, zero`
const BKPT_INST: usize = 0xc0101073;
static mut BKPT_INST_ADDR: usize = 0;
static mut BKPT_RESERVED_INST: usize = 0;

macro_rules! amo {
    ($ctx:expr, $inst:ident, $operation:expr) => {{
        let tmp = read_register($ctx, $inst.rs1());
        let a = *(tmp as *const _);
        let b = read_register($ctx, $inst.rs2());
        if ($inst.rd() != 0) {
            write_register($ctx, $inst.rd(), a);
        }
        *(tmp as *mut _) = $operation(a, b);
    }};
}

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
fn check_trap_privilege_mode() {
    if mstatus::read() & mstatus::MPP == mstatus::MPP_MACHINE {
        panic!("{:?} from M-MODE", mcause::read().cause());
    }
}

#[inline]
unsafe fn delegate() {
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

#[inline]
fn illegal_instruction_handler(mut ctx: FastContext) -> Result<FastResult, FastContext> {
    let inst = decode(mtval::read() as u32);
    match inst {
        Ok(Instruction::Csrrs(csr)) => match csr.csr() as usize {
            CSR_TIME => {
                ctx.regs().a[(csr.rd() - 10) as usize] = SBI.timer.time() as usize;
            }
            CSR_TIMEH => {
                ctx.regs().a[(csr.rd() - 10) as usize] = SBI.timer.timeh() as usize;
            }
            _ => return Err(ctx),
        },
        Ok(Instruction::Csrrw(csr)) => unsafe {
            if csr.csr() as usize == CSR_TIME && mepc::read() == BKPT_INST_ADDR {
                clear_breakpoint();
                return Ok(ctx.continue_with(atomic_emulation_wrapper, ()));
            } else {
                return Err(ctx);
            }
        },
        _ => return Err(ctx),
    }
    mepc::next();
    Ok(ctx.restore())
}

unsafe fn find_next_sc(addr: usize) -> Result<usize, ()> {
    let mut addr = addr;
    for _ in 0..16 {
        let inst = (addr as *const u32).read();
        if let Ok(Instruction::ScW(_)) = decode(inst) {
            return Ok(addr);
        } else if (inst & 0xFF) != 0b11 {
            // RVC instruction
            addr += 2;
        } else {
            addr += 4;
        }
    }
    Err(())
}

unsafe fn set_breakpoint(addr: usize) {
    let addr = addr as *mut usize;
    BKPT_RESERVED_INST = addr.read();
    BKPT_INST_ADDR = addr as usize;
    *addr = BKPT_INST;
    fence_i();
}

unsafe fn clear_breakpoint() {
    if BKPT_INST_ADDR != 0 {
        let addr = BKPT_INST_ADDR as *mut usize;
        *addr = BKPT_RESERVED_INST;
        BKPT_INST_ADDR = 0;
        fence_i();
    }
}

unsafe fn write_register(ctx: &mut EntireContextSeparated, r: u32, value: usize) {
    let r = r as usize;
    match r {
        // x0
        0 => {}
        // gp
        3 => core::arch::asm!("c.mv gp, {}", in(reg) value),
        // tp
        4 => core::arch::asm!("c.mv tp, {}", in(reg) value),
        5..=7 => ctx.regs().t[r - 5] = value,
        8..=9 => {
            ctx.regs().s[r - 8] = value;
        }
        10..=17 => {
            ctx.regs().a[r - 10] = value;
        }
        18..=27 => {
            ctx.regs().s[r - 16] = value;
        }
        28..=31 => {
            ctx.regs().t[r - 25] = value;
        }
        _ => panic!("invalid register number: {}", r),
    }
}

fn read_register(ctx: &mut EntireContextSeparated, r: u32) -> usize {
    let r = r as usize;
    match r {
        // x0
        0 => 0,
        // gp
        3 => unsafe {
            let value: usize;
            core::arch::asm!("c.mv {}, gp", out(reg) value);
            value
        },
        4 => unsafe {
            let value: usize;
            core::arch::asm!("c.mv {}, tp", out(reg) value);
            value
        },
        5..=7 => ctx.regs().t[r - 5],
        8..=9 => ctx.regs().s[r - 8],
        10..=17 => ctx.regs().a[r - 10],
        18..=27 => ctx.regs().s[r - 16],
        28..=31 => ctx.regs().t[r - 25],
        _ => panic!("invalid register number: {}", r),
    }
}

unsafe fn atomic_emulation(mut ctx: EntireContextSeparated) -> EntireResult {
    let inst = (mepc::read() as *const u32).read_unaligned();
    let decoded_inst = decode(inst);
    match decoded_inst {
        Ok(Instruction::LrW(lr)) => {
            let rs1 = lr.rs1();
            let rd = lr.rd();
            S_LR_ADDR = read_register(&mut ctx, rs1);
            let tmp: usize = *(S_LR_ADDR as *const _);
            write_register(&mut ctx, rd, tmp);

            // Clear old breakpoint and set a new one
            clear_breakpoint();
            let sc_inst_addr = find_next_sc(mepc::read()).unwrap_or_else(|_| {
                panic!("[rustsbi] unable to find matching sc instruction");
            });
            set_breakpoint(sc_inst_addr);
        }
        Ok(Instruction::ScW(sc)) => {
            let rs1 = sc.rs1();
            let rs2 = sc.rs2();
            let rd = sc.rd();
            let tmp: usize = read_register(&mut ctx, rs1);
            if tmp != S_LR_ADDR {
                write_register(&mut ctx, rd, 1);
            } else {
                *(S_LR_ADDR as *mut _) = read_register(&mut ctx, rs2);
                write_register(&mut ctx, rd, 0);
                S_LR_ADDR = 0;
            }
        }
        Ok(Instruction::AmoswapW(amo)) => {
            amo!(&mut ctx, amo, |_, b| b);
        }
        Ok(Instruction::AmoaddW(amo)) => {
            amo!(&mut ctx, amo, |a, b| a + b);
        }
        Ok(Instruction::AmoxorW(amo)) => {
            amo!(&mut ctx, amo, |a, b| a ^ b);
        }
        Ok(Instruction::AmoandW(amo)) => {
            amo!(&mut ctx, amo, |a, b| a & b);
        }
        Ok(Instruction::AmoorW(amo)) => {
            amo!(&mut ctx, amo, |a, b| a | b);
        }
        Ok(Instruction::AmominW(amo)) => {
            amo!(&mut ctx, amo, |a, b| (a as isize).min(b as isize));
        }
        Ok(Instruction::AmomaxW(amo)) => {
            amo!(&mut ctx, amo, |a, b| (a as isize).max(b as isize));
        }
        Ok(Instruction::AmominuW(amo)) => {
            amo!(&mut ctx, amo, |a: usize, b| a.min(b));
        }
        Ok(Instruction::AmomaxuW(amo)) => {
            amo!(&mut ctx, amo, |a: usize, b| a.max(b));
        }
        _ => {
            delegate();
            return ctx.restore();
        }
    }
    mepc::next();
    ctx.restore()
}

extern "C" fn atomic_emulation_wrapper(ctx: EntireContext) -> EntireResult {
    let (ctx, _) = ctx.split();
    unsafe { atomic_emulation(ctx) }
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
                            legacy::LEGACY_CONSOLE_GETCHAR => {
                                ret.error = board::getchar();
                                ret.value = a1;
                            }
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
                    check_trap_privilege_mode();
                    ctx.regs().a = [ctx.a0(), a1, a2, a3, a4, a5, a6, a7];
                    break illegal_instruction_handler(ctx).unwrap_or_else(|ctx| unsafe {
                        delegate();
                        ctx.restore()
                    });
                }
                T::Exception(E::LoadFault) => {
                    check_trap_privilege_mode();
                    ctx.regs().a = [ctx.a0(), a1, a2, a3, a4, a5, a6, a7];
                    break ctx.continue_with(atomic_emulation_wrapper, ());
                }
                T::Exception(E::StoreFault) => {
                    check_trap_privilege_mode();
                    ctx.regs().a = [ctx.a0(), a1, a2, a3, a4, a5, a6, a7];
                    break ctx.continue_with(atomic_emulation_wrapper, ());
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
