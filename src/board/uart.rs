#![allow(dead_code)]

use core::fmt::Write;

use super::pac::uart;

pub struct Uart {
    inner: uart::Uart,
}

impl Uart {
    pub fn new(uart: uart::Uart) -> Self {
        Self { inner: uart }
    }

    pub fn setup(&self, buadrate: u32, clock_src_freq: u32) {
        let uart = &self.inner;

        // Disable all interrupt
        uart.dlm().write(|w| w.set_dlm(0));

        // Set DLAB to 1
        uart.lcr().modify(|m| m.set_dlab(true));
        // Calculate baud rate
        let div = clock_src_freq / (buadrate * 16);
        uart.dll().write(|m| m.set_dll(div as u8));
        uart.dlm().write(|m| m.set_dlm((div >> 8) as u8));
        // Set DLAB to 0
        uart.lcr().modify(|m| m.set_dlab(false));

        // Word length to 8 bits
        uart.lcr().modify(|m| m.set_wls(3));
        // Enable TX and RX FIFO
        uart.fcr().modify(|m| m.set_fifoe(true))
    }

    #[inline]
    fn is_tx_fifo_empty(&self) -> bool {
        self.inner.lsr().read().thre()
    }

    #[inline]
    fn is_data_ready(&self) -> bool {
        self.inner.lsr().read().dr()
    }

    #[inline]
    pub fn send_byte(&self, byte: u8) {
        while !self.is_tx_fifo_empty() {}
        self.inner.dll().write(|w| w.set_dll(byte));
    }

    #[inline]
    pub fn receive_byte(&self, byte: &mut u8) -> bool {
        if self.is_data_ready() {
            *byte = self.inner.rbr().read().rbr();
            true
        } else {
            false
        }
    }
}

impl Write for Uart {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for ch in s.bytes() {
            self.send_byte(ch);
        }
        Ok(())
    }
}
