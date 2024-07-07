#![allow(unused)]

use super::ral::{gpio, ioc};
use super::ral::{modify_reg, read_reg, write_reg};

#[derive(Clone, Copy)]
pub enum PinState {
    Low = 0,
    High,
}

pub enum Pull {
    PullDown = 0,
    PullUp,
    Floating,
}

pub struct Pin<'a, const PORT: char, const PIN: u8> {
    gpio: &'a gpio::GPIO0,
    ioc: &'a ioc::IOC0,
    pioc: &'a ioc::PIOC10,
}

macro_rules! impl_port {
    ($port:literal, $OE_VALUE:ident, $DO_SET:ident, $DO_CLEAR:ident, $DO_TOGGLE:ident, $DI_VALUE:ident) => {
        impl<'a, const PIN: u8> Pin<'a, $port, PIN> {
            #[inline(always)]
            fn output_enable(&self, enable: bool) -> &Self {
                let offset = PIN;
                let mask = 0b1 << offset;
                match enable {
                    true => modify_reg!(gpio, self.gpio, $OE_VALUE, |r| r | mask),
                    false => modify_reg!(gpio, self.gpio, $OE_VALUE, |r| r & !mask),
                }
                self
            }

            #[inline(always)]
            pub fn set_high(&self) -> &Self {
                write_reg!(gpio, self.gpio, $DO_SET, 1 << PIN);
                self
            }

            #[inline(always)]
            pub fn set_low(&self) -> &Self {
                write_reg!(gpio, self.gpio, $DO_CLEAR, 1 << PIN);
                self
            }

            #[inline(always)]
            pub fn set_bool(&self, state: bool) -> &Self {
                match state {
                    false => self.set_low(),
                    true => self.set_high(),
                }
            }

            #[inline(always)]
            pub fn toggle(&self) -> &Self {
                write_reg!(gpio, self.gpio, $DO_TOGGLE, 1 << PIN);
                self
            }

            #[inline(always)]
            pub fn get_state(&self) -> PinState {
                match read_reg!(gpio, self.gpio, $DI_VALUE) >> PIN & 0b1 {
                    0 => PinState::Low,
                    1 => PinState::High,
                    _ => unreachable!(),
                }
            }

            #[inline(always)]
            pub fn is_high(&self) -> bool {
                match self.get_state() {
                    PinState::Low => false,
                    PinState::High => true,
                }
            }

            #[inline(always)]
            pub fn is_low(&self) -> bool {
                match self.get_state() {
                    PinState::Low => true,
                    PinState::High => false,
                }
            }
        }
    };
}

macro_rules! pin {
    ($PXX:ident: $port:literal, $pin:literal, $FUNC_CTL:ident, $PAD_CTL:ident, $AF_MODE:literal) => {
        pub type $PXX<'a> = Pin<'a, $port, $pin>;

        impl<'a> $PXX<'a> {
            // For each pin
            fn new(gpio: &'a gpio::GPIO0, ioc: &'a ioc::IOC0, pioc: &'a ioc::PIOC10) -> Self {
                Pin { gpio, ioc, pioc }
            }

            fn set_af(&self, alt: u32) -> &Self {
                assert!(alt < 32);
                modify_reg!(ioc, self.ioc, $FUNC_CTL, ALT_SELECT: alt);
                if $port == 'Y' {
                    modify_reg!(ioc, self.pioc, $FUNC_CTL, ALT_SELECT: 3);
                }
                self
            }

            #[inline(always)]
            fn set_loop_back(&self, on: bool) -> &Self {
                modify_reg!(ioc, self.ioc, $FUNC_CTL, LOOP_BACK: on as u32);
                self
            }

            #[inline(always)]
            pub fn set_mode_output(&self) -> &Self {
                self.output_enable(true).set_af(0)
            }

            #[inline(always)]
            pub fn set_mode_input(&self) -> &Self {
                self.output_enable(false).set_af(0)
            }

            #[inline(always)]
            pub fn set_mode_alternate(&self) -> &Self {
                self.set_af($AF_MODE)
            }

            #[inline(always)]
            pub fn set_push_pull(&self) -> &Self {
                modify_reg!(ioc, self.ioc, $PAD_CTL, OD: Disable);
                self
            }

            #[inline(always)]
            pub fn set_open_drain(&self) -> &Self {
                modify_reg!(ioc, self.ioc, $PAD_CTL, OD: Enable);
                self
            }

            #[inline(always)]
            pub fn set_pull(&self, pull: Pull) -> &Self {
                match pull {
                    Pull::Floating => modify_reg!(ioc, self.ioc, $PAD_CTL, PE: Disable),
                    _ => modify_reg!(ioc, self.ioc, $PAD_CTL, PE: Enable, PS: pull as u32),
                }
                self
            }

            #[inline(always)]
            pub fn set_pull_down(&self) -> &Self {
                self.set_pull(Pull::PullDown)
            }

            #[inline(always)]
            pub fn set_pull_up(&self) -> &Self {
                self.set_pull(Pull::PullUp)
            }

            #[inline(always)]
            pub fn set_pull_floating(&self) -> &Self {
                self.set_pull(Pull::Floating)
            }
        }
    };
}

macro_rules! pins {
    ($(
        $port:literal: {
            $OE_VALUE:ident,
            $DO_SET:ident,
            $DO_CLEAR:ident,
            $DO_TOGGLE:ident,
            $DI_VALUE:ident,
            [$(($PXX:ident, $pxx:ident, $pin:literal, $FUNC_CTL:ident, $PAD_CTL:ident, $AF_MODE:literal),)*]
        }
    ),*) => {
        $(
            impl_port!($port, $OE_VALUE, $DO_SET, $DO_CLEAR, $DO_TOGGLE, $DI_VALUE);

            $(pin!($PXX: $port, $pin, $FUNC_CTL, $PAD_CTL, $AF_MODE);)*
        )*

        pub struct Pins<'a> {
            $(
                $(pub $pxx: $PXX<'a>,)*
            )*
        }

        impl<'a> Pins<'a> {
            pub fn new(gpio: &'a gpio::GPIO0, ioc: &'a ioc::IOC0, pioc: &'a ioc::PIOC10) -> Self {
                Pins {
                    $(
                        $($pxx: $PXX::new(&gpio, &ioc, &pioc),)*
                    )*
                }
            }
        }
    };
}

pins!(
    'A': {
        OE_GPIOA_VALUE,
        DO_GPIOA_SET, DO_GPIOA_CLEAR, DO_GPIOA_TOGGLE,
        DI_GPIOA_VALUE,
        [
            (PA16, spi1_mosi, 16, PAD_PA16_FUNC_CTL, PAD_PA16_PAD_CTL, 5),
            (PA21, spi1_clk,  21, PAD_PA21_FUNC_CTL, PAD_PA21_PAD_CTL, 5),
            (PA23, spi1_miso, 23, PAD_PA23_FUNC_CTL, PAD_PA23_PAD_CTL, 5),
            (PA26, reset,     26, PAD_PA26_FUNC_CTL, PAD_PA26_PAD_CTL, 0),
            (PA27, spi2_mosi, 27, PAD_PA27_FUNC_CTL, PAD_PA27_PAD_CTL, 5),
            (PA29, uart9_rx,  29, PAD_PA29_FUNC_CTL, PAD_PA29_PAD_CTL, 2),
            (PA30, uart9_tx,  30, PAD_PA30_FUNC_CTL, PAD_PA30_PAD_CTL, 2),
            (PA31, spi2_miso, 31, PAD_PA31_FUNC_CTL, PAD_PA31_PAD_CTL, 5),
        ]
    },
    'B': {
        OE_GPIOB_VALUE,
        DO_GPIOB_SET, DO_GPIOB_CLEAR, DO_GPIOB_TOGGLE,
        DI_GPIOB_VALUE,
        [
            (PB00, spi2_clk,   0, PAD_PB00_FUNC_CTL, PAD_PB00_PAD_CTL, 5),
            (PB14, led_debug, 14, PAD_PB14_FUNC_CTL, PAD_PB14_PAD_CTL, 0),
            (PB18, led_green, 18, PAD_PB18_FUNC_CTL, PAD_PB18_PAD_CTL, 0),
            (PB19, led_red,   19, PAD_PB19_FUNC_CTL, PAD_PB19_PAD_CTL, 0),
            (PB20, led_blue,  20, PAD_PB20_FUNC_CTL, PAD_PB20_PAD_CTL, 0),
        ]
    },
    'D': {
        OE_GPIOD_VALUE,
        DO_GPIOD_SET, DO_GPIOD_CLEAR, DO_GPIOD_TOGGLE,
        DI_GPIOD_VALUE,
        [
            (PD15, led_uart, 15, PAD_PD15_FUNC_CTL, PAD_PD15_PAD_CTL, 0),
            (PD30, xpi0_d2,  30, PAD_PD30_FUNC_CTL, PAD_PD30_PAD_CTL, 0),
            (PD31, xpi0_d0,  31, PAD_PD31_FUNC_CTL, PAD_PD31_PAD_CTL, 0),
        ]
    },
    'E': {
        OE_GPIOE_VALUE,
        DO_GPIOE_SET, DO_GPIOE_CLEAR, DO_GPIOE_TOGGLE,
        DI_GPIOE_VALUE,
        [
            (PE02, xpi0_cs,   2, PAD_PE02_FUNC_CTL, PAD_PE02_PAD_CTL, 0),
            (PE03, xpi0_d3,   3, PAD_PE03_FUNC_CTL, PAD_PE03_PAD_CTL, 0),
            (PE04, xpi0_d1,   4, PAD_PE04_FUNC_CTL, PAD_PE04_PAD_CTL, 0),
            (PE07, xpi0_sclk, 7, PAD_PE07_FUNC_CTL, PAD_PE07_PAD_CTL, 0),
        ]
    },
    'Y': {
        OE_GPIOY_VALUE,
        DO_GPIOY_SET, DO_GPIOY_CLEAR, DO_GPIOY_TOGGLE,
        DI_GPIOY_VALUE,
        [
            (PY06, uart0_tx,  6, PAD_PY06_FUNC_CTL, PAD_PY06_PAD_CTL, 2),
            (PY07, uart0_rx,  7, PAD_PY07_FUNC_CTL, PAD_PY07_PAD_CTL, 2),
        ]
    }
);

pub struct Gpio {
    gpio: gpio::GPIO0,
    ioc: ioc::IOC0,
    pioc: ioc::PIOC10,
}

impl Gpio {
    pub fn new(gpio: gpio::GPIO0, ioc: ioc::IOC0, pioc: ioc::PIOC10) -> Self {
        Self { gpio, ioc, pioc }
    }

    pub fn split(&self) -> Pins {
        Pins::new(&self.gpio, &self.ioc, &self.pioc)
    }
}

impl<'a> Pins<'a> {
    pub fn setup(&self) {
        // Setup RGB LED pinmux
        self.led_red
            .set_mode_output()
            .set_push_pull()
            .set_pull_floating()
            .set_high();
        self.led_green
            .set_mode_output()
            .set_push_pull()
            .set_pull_floating()
            .set_high();
        self.led_blue
            .set_mode_output()
            .set_push_pull()
            .set_pull_floating()
            .set_high();

        // Setup UART0 pinmux
        self.uart0_tx.set_mode_alternate();
        self.uart0_rx.set_mode_alternate();
    }
}
