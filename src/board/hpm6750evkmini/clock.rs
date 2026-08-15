use hpm_hal::pac;
use hpm_hal::sysctl::{ClockConfig, ClockMux};

const CLK_24M_HZ: u32 = 24_000_000;
pub const MCHTMR_CLOCK_HZ: u32 = 1_000_000;
const MCHTMR_CLOCK_DIVIDER: u32 = CLK_24M_HZ / MCHTMR_CLOCK_HZ;

pub fn configure_mchtmr_clock() {
    set_clock(
        pac::clocks::MCT0,
        ClockConfig::new(ClockMux::CLK_24M, MCHTMR_CLOCK_DIVIDER as u16),
    );
}

fn set_clock(clock: usize, config: ClockConfig) {
    let sysctl = pac::SYSCTL;

    sysctl.clock(clock).modify(|w| {
        w.set_mux(config.src);
        w.set_div(config.raw_div);
    });
    while sysctl.clock(clock).read().loc_busy() {}
}
