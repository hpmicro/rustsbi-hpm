use super::pac;
use crate::board::delay::CycleDelay;
use crate::board::uart;
use crate::println;

mod clock;
mod sdram;

pub const PLATFORM: &str = "HPM6360EVK";

pub fn board_init() {
    hpm_rt::cache::icache_enable();
    hpm_rt::cache::dcache_enable();

    let p = hpm_hal::init(Default::default());

    let clocks = hpm_hal::sysctl::clocks();

    clock::configure_mchtmr_clock();

    uart::init_uart(p.UART0, p.PY06, p.PY07, 115_200);

    let mut delay = CycleDelay::new(clocks.cpu0.0);
    let sdram = sdram::init!(p, delay);

    #[cfg(feature = "sdram-rw-test")]
    sdram::run_rw_test(sdram.base_address);

    println!(
        "\
[rustsbi pre-init] CPU0 clock frequency   : {}Hz
[rustsbi pre-init] AXI clock frequency    : {}Hz
[rustsbi pre-init] AHB clock frequency    : {}Hz
[rustsbi pre-init] MCHTMR clock frequency : {}Hz
[rustsbi pre-init] SDRAM clock frequency  : {}Hz
[rustsbi pre-init] SDRAM base address     : {:#010x}
",
        clocks.cpu0.0,
        clocks.axi.0,
        clocks.ahb.0,
        clocks.get_clock_freq(pac::clocks::MCT0).0,
        clocks.get_clock_freq(pac::clocks::FEMC).0,
        sdram.base_address,
    );
}
