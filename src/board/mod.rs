use core::fmt::{self, Write};

use hpm_metapac as pac;

mod clock;
mod pin;
mod uart;

use clock::{clocks, ClockConfigurator};
use pin::PinCtrl;
use uart::Uart;

use spin::lock_api::Mutex;

pub static UART: Mutex<Option<Uart>> = Mutex::new(None);

pub fn board_init() {
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
}

#[inline]
pub fn _print(args: fmt::Arguments) {
    let mut guard = UART.lock();

    guard.as_mut().unwrap().write_fmt(args).unwrap();
}

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
