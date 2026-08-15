use core::arch::asm;

use super::pac;
use crate::board::delay::CycleDelay;
use crate::board::uart;
use crate::println;

mod clock;
mod sdram;

pub const PLATFORM: &str = "HPM6750EVKMINI";

pub fn board_init() {
    hpm_rt::cache::icache_enable();
    hpm_rt::cache::dcache_enable();

    let p = hpm_hal::init(Default::default());

    clock::configure_mchtmr_clock();

    uart::init_uart(p.UART0, p.PY06, p.PY07, 115_200);

    let clocks = hpm_hal::sysctl::clocks();
    let mut delay = CycleDelay::new(clocks.cpu0.0);
    let sdram = sdram::init!(p, delay);

    #[cfg(feature = "sdram-rw-test")]
    sdram::run_rw_test(sdram.base_address);

    println!(
        "\
[rustsbi pre-init] CPU0 clock frequency   : {}Hz
[rustsbi pre-init] CPU1 clock frequency   : {}Hz
[rustsbi pre-init] AHB clock frequency    : {}Hz
[rustsbi pre-init] MCHTMR clock frequency : {}Hz
[rustsbi pre-init] SDRAM clock frequency  : {}Hz
[rustsbi pre-init] SDRAM base address     : {:#010x}
",
        clocks.cpu0.0,
        clocks.cpu1.0,
        clocks.ahb.0,
        clocks.get_clock_freq(pac::clocks::MCT0).0,
        clocks.get_clock_freq(pac::clocks::FEMC).0,
        sdram.base_address,
    );

    unsafe {
        // Allow Supervisor/User mode to access cache control register (CSR 0x7ca)
        // to write back/invalidate cache lines.
        asm!(
            "csrrs zero, 0x7ca, {cctl_suen}",
            cctl_suen = in(reg) 0x1 << 8,
        );
    }
}
