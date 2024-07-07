use core::fmt::{self, Write};

use hpm_ral as ral;

mod clock;
mod pin;
mod uart;

use clock::ClockConfigurator;
use pin::Gpio;
use uart::Uart;

use spin::lock_api::Mutex;

pub static UART: Mutex<Option<Uart<0>>> = Mutex::new(None);

pub fn board_init() {
    let sysctl = unsafe { ral::sysctl::SYSCTL::instance() };
    let pllctl = unsafe { ral::pllctl::PLLCTL::instance() };
    let ioc = unsafe { ral::ioc::IOC0::instance() };
    let pioc = unsafe { ral::ioc::PIOC10::instance() };
    let gpio0 = unsafe { ral::gpio::GPIO0::instance() };
    let uart0 = unsafe { ral::uart::UART0::instance() };

    let clock = unsafe { ClockConfigurator::new(sysctl, pllctl).freeze() };
    let gpio = Gpio::new(gpio0, ioc, pioc);
    let pins = gpio.split();
    let uart = Uart::new(uart0);

    pins.setup();

    uart.setup(115_200, clock.get_clk_uart0_freq());
    *UART.lock() = Some(uart);
}

#[inline]
pub fn print(args: fmt::Arguments) {
    let mut guard = UART.lock();

    guard.as_mut().unwrap().write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($fmt: literal $(, $($args: tt)+)?) => {
        $crate::board::print(format_args!($fmt $(, $($args)+)?));
    };
}

#[macro_export]
macro_rules! println {
    ($fmt: literal $(, $($args: tt)+)?) => {
        $crate::board::print(format_args!(concat!($fmt, "\n") $(, $($args)+)?));
    };
}
