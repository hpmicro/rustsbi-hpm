use core::fmt::{self, Write};

use hpm_metapac as pac;
use spin::lock_api::Mutex;

mod clock;
mod femc;
mod pin;
mod uart;

use clock::{clocks, ClockConfigurator};
use femc::Sdram;
use pin::PinCtrl;
use uart::Uart;

pub static UART: Mutex<Option<Uart>> = Mutex::new(None);

#[macro_export]
macro_rules! print {
    ($fmt: literal $(, $($args: tt)+)?) => {
        $crate::board::_print(format_args!($fmt $(, $($args)+)?));
    };
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => {{
        $crate::board::_print(core::format_args!($($arg)*));
        $crate::println!();
    }}
}

pub fn board_init() {
    hpm_rt::cache::icache_enable();

    let sysctl = pac::SYSCTL;
    let pllctl = pac::PLLCTL;
    let clock = unsafe { ClockConfigurator::new(sysctl, pllctl).freeze() };

    let gpio0 = pac::GPIO0;
    let ioc = pac::IOC;
    let pioc = pac::PIOC;
    let pinctrl = PinCtrl::new(gpio0, ioc, pioc);
    let pins = pinctrl.split();
    pins.setup();

    let uart0 = pac::UART0;
    let uart = Uart::new(uart0);
    uart.setup(115_200, clock.get_clk_freq(clocks::URT0));
    *UART.lock() = Some(uart);

    let cpu0_clock_freq = clock.get_cpu0_clk_freq();
    let sdram_clock_freq = clock.get_clk_freq(clocks::FEMC);
    let sdram = Sdram::new(pac::FEMC).config();
    println!(
        "\
[rustsbi pre-init] CPU0 clock frequency  : {}Hz
[rustsbi pre-init] SDRAM clock frequency : {}Hz
[rustsbi pre-init] SDRAM base address    : {:#010x}",
        cpu0_clock_freq,
        sdram_clock_freq,
        sdram.base_address()
    );
}

#[inline]
pub fn _print(args: fmt::Arguments) {
    let mut guard = UART.lock();

    guard.as_mut().unwrap().write_fmt(args).unwrap();
}
