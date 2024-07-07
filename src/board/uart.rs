#![allow(dead_code)]

use core::fmt::Write;
use core::ops::Deref;

use hpm_ral::uart;
use hpm_ral::{modify_reg, read_reg, write_reg};

pub struct Uart<const N: u8> {
    inner: uart::Instance<N>,
}

impl<const N: u8> Uart<N> {
    pub fn new(uart: uart::Instance<N>) -> Self {
        Self { inner: uart }
    }

    pub fn setup(&self, buadrate: u32, clock_src_freq: u32) {
        // Disable all interrupt
        write_reg!(uart, self.inner, DLM, 0);
        // Set DLAB to 1
        modify_reg!(uart, self.inner, LCR, DLAB: 1);

        let div = clock_src_freq / (buadrate * 16);
        modify_reg!(uart, self.inner, DLL, DLL: div);
        modify_reg!(uart, self.inner, DLM, DLM: div >> 8);

        // Set DLAB to 0
        modify_reg!(uart, self.inner, LCR, DLAB: 0);
        // Word length to 8 bits
        modify_reg!(uart, self.inner, LCR, WLS: Bits8);
        // Enable TX and RX FIFO and DMA
        modify_reg!(uart, self.inner, FCR, FIFOE: 1);
    }

    #[inline]
    fn is_tx_fifo_empty(&self) -> bool {
        return read_reg!(uart, self.inner, LSR, THRE) == 1;
    }

    #[inline]
    pub fn send_byte(&self, byte: u8) {
        while !self.is_tx_fifo_empty() {}
        write_reg!(uart, self.inner, DLL, DLL: byte as u32);
    }
}

impl<const N: u8> Write for Uart<N> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        match self.inner.deref() as *const uart::RegisterBlock {
            uart::UART0 => {
                for ch in s.bytes() {
                    self.send_byte(ch);
                }
                Ok(())
            }
            _ => Err(core::fmt::Error),
        }
    }
}
