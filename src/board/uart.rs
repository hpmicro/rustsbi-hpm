use core::fmt::{self, Write};

use hpm_hal::{
    Peri,
    gpio::Pin,
    mode::Blocking,
    uart::{self, Uart},
};
use spin::Mutex;

pub type HalUart = Uart<'static, Blocking>;

static UART: Mutex<Option<HalUart>> = Mutex::new(None);

pub fn init_uart<T, TX, RX>(
    uart: Peri<'static, T>,
    tx: Peri<'static, TX>,
    rx: Peri<'static, RX>,
    baudrate: u32,
) where
    T: uart::Instance,
    TX: Pin + uart::TxPin<T>,
    RX: Pin + uart::RxPin<T>,
{
    let mut config = uart::Config::default();
    config.baudrate = baudrate;

    tx.set_as_ioc_gpio();
    rx.set_as_ioc_gpio();

    let uart = Uart::new_blocking(uart, rx, tx, config).unwrap();
    UART.lock().replace(uart);
}

#[inline]
pub fn putchar(args: fmt::Arguments) {
    if let Some(uart) = UART.lock().as_mut() {
        uart.write_fmt(args).unwrap();
    }
}

#[inline]
pub fn getchar() -> usize {
    use embedded_hal_nb::serial::Read;

    if let Some(uart) = UART.lock().as_mut() {
        match uart.read() {
            Ok(c) => c as _,
            Err(_) => usize::MAX,
        }
    } else {
        usize::MAX
    }
}
