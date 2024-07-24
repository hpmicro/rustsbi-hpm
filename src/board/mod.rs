use core::{
    fmt::{self, Write},
    mem::MaybeUninit,
};

use hpm_metapac as pac;
use hpm_rt;
use spin::lock_api::Mutex;

mod clock;
mod femc;
mod mchtmr;
mod pin;
mod uart;

use clock::{clocks, ClockConfigurator};
use femc::Sdram;
pub use mchtmr::MachineTimer;
use pin::PinCtrl;
use uart::Uart;

static UART: Mutex<MaybeUninit<Uart>> = Mutex::new(MaybeUninit::uninit());

#[macro_export]
macro_rules! print {
    ($fmt: literal $(, $($args: tt)+)?) => {
        $crate::board::putchar(format_args!($fmt $(, $($args)+)?));
    };
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => {{
        $crate::board::putchar(core::format_args!($($arg)*));
        $crate::println!();
    }}
}

pub fn board_init() {
    hpm_rt::cache::icache_enable();

    let clock = unsafe { ClockConfigurator::new(pac::SYSCTL, pac::PLLCTL).freeze() };

    let pinctrl = PinCtrl::new(pac::GPIO0, pac::IOC, pac::PIOC);
    let pins = pinctrl.split();
    pins.setup();

    let uart = Uart::new(pac::UART0);
    uart.setup(115_200, clock.get_clk_freq(clocks::URT0));
    *UART.lock() = MaybeUninit::new(uart);

    let cpu0_clock_freq = clock.get_cpu0_clk_freq();
    let mchtmr_clock_freq = clock.get_clk_freq(clocks::MCT0);
    let sdram_clock_freq = clock.get_clk_freq(clocks::FEMC);
    let sdram = Sdram::new(pac::FEMC).config();
    println!(
        "\
[rustsbi pre-init] CPU0 clock frequency  : {}Hz
[rustsbi pre-init] MCHTMT clock frequency  : {}Hz
[rustsbi pre-init] SDRAM clock frequency : {}Hz
[rustsbi pre-init] SDRAM base address    : {:#010x}",
        cpu0_clock_freq,
        mchtmr_clock_freq,
        sdram_clock_freq,
        sdram.base_address()
    );
}

#[inline]
pub fn putchar(args: fmt::Arguments) {
    let mut guard = UART.lock();

    unsafe { guard.assume_init_mut().write_fmt(args).unwrap() }
}

#[inline]
pub fn getchar() -> usize {
    let mut guard = UART.lock();
    let mut c: u8 = 0;

    unsafe {
        if guard.assume_init_mut().receive_byte(&mut c) {
            c as _
        } else {
            usize::MAX
        }
    }
}

pub fn board_init_timer() -> MachineTimer {
    MachineTimer::new(pac::MCHTMR)
}
