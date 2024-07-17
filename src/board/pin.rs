#![allow(unused)]

use super::pac::gpio::Gpio;
use super::pac::ioc::Ioc;

#[derive(Clone, Copy)]
pub enum PinState {
    Low = 0,
    High,
}

#[derive(PartialEq)]
pub enum Pull {
    PullDown = 0,
    PullUp,
    Floating,
}

pub struct Pin<'a, const PORT: char, const PIN: u8> {
    gpio: &'a Gpio,
    ioc: &'a Ioc,
    pioc: &'a Ioc,
}

impl<'a, const PORT: char, const PIN: u8> Pin<'a, PORT, PIN> {
    fn base_n() -> usize {
        let mut n = (PORT as usize) - ('A' as usize);
        if n > 13 {
            n -= 10;
        }
        n
    }

    // For each pin
    fn new(gpio: &'a Gpio, ioc: &'a Ioc, pioc: &'a Ioc) -> Self {
        Pin { gpio, ioc, pioc }
    }

    pub fn output_enable(&self, enable: bool) -> &Self {
        match enable {
            true => self
                .gpio
                .oe(Self::base_n())
                .set()
                .write(|m| m.set_direction(1 << PIN)),
            false => self
                .gpio
                .oe(Self::base_n())
                .clear()
                .write(|m| m.set_direction(1 << PIN)),
        }
        self
    }

    #[inline(always)]
    pub fn set_high(&self) -> &Self {
        self.gpio
            .do_(Self::base_n())
            .set()
            .write(|w| w.set_output(1 << PIN));
        self
    }

    #[inline(always)]
    pub fn set_low(&self) -> &Self {
        self.gpio
            .do_(Self::base_n())
            .clear()
            .write(|w| w.set_output(1 << PIN));
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
        self.gpio
            .do_(Self::base_n())
            .toggle()
            .write(|w| w.set_output(1 << PIN));
        self
    }

    #[inline(always)]
    pub fn get_state(&self) -> PinState {
        let val = self.gpio.di(Self::base_n()).value().read().input();
        match val >> PIN & 0b1 {
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

    fn set_af(&self, alt: u8) -> &Self {
        assert!(alt < 32);
        let n = Self::base_n() * 32 + PIN as usize;
        self.ioc.pad(n).func_ctl().modify(|m| m.set_alt_select(alt));
        if PORT == 'Y' {
            self.pioc.pad(n).func_ctl().modify(|m| m.set_alt_select(3));
        }
        self
    }

    #[inline(always)]
    fn set_loop_back(&self, on: bool) -> &Self {
        let n = Self::base_n() * 32 + PIN as usize;
        self.ioc
            .pad(n)
            .func_ctl()
            .modify(|m| m.set_loop_back(on.into()));
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
    pub fn set_push_pull(&self) -> &Self {
        let n = Self::base_n() * 32 + PIN as usize;
        self.ioc.pad(n).pad_ctl().modify(|m| m.set_od(false));
        self
    }

    #[inline(always)]
    pub fn set_open_drain(&self) -> &Self {
        let n = Self::base_n() * 32 + PIN as usize;
        self.ioc.pad(n).pad_ctl().modify(|m| m.set_od(true));
        self
    }

    #[inline(always)]
    pub fn set_pull(&self, pull: Pull) -> &Self {
        let n = Self::base_n() * 32 + PIN as usize;
        let r = self.ioc.pad(n).pad_ctl();
        match pull {
            Pull::Floating => r.modify(|m| m.set_pe(false)),
            _ => r.modify(|m| {
                m.set_pe(true);
                m.set_ps(pull == Pull::PullDown)
            }),
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

macro_rules! pin {
    ($PXX:ident: $port:literal, $pin:literal, $AF_MODE:literal) => {
        pub type $PXX<'a> = Pin<'a, $port, $pin>;

        impl<'a> $PXX<'a> {
            #[inline(always)]
            pub fn set_mode_alternate(&self) -> &Self {
                self.set_af($AF_MODE)
            }
        }
    };
}

pub struct PinCtrl {
    gpio: Gpio,
    ioc: Ioc,
    pioc: Ioc,
}

impl PinCtrl {
    pub fn new(gpio: Gpio, ioc: Ioc, pioc: Ioc) -> Self {
        Self { gpio, ioc, pioc }
    }

    pub fn split(&self) -> Pins {
        Pins::new(&self.gpio, &self.ioc, &self.pioc)
    }
}

macro_rules! pins {
    ($(
        $port:literal: [$(($PXX:ident, $pxx:ident, $pin:literal, $AF_MODE:literal),)*]
    ),*) => {
        $(
            $(pin!($PXX: $port, $pin, $AF_MODE);)*
        )*

        pub struct Pins<'a> {
            $(
                $(pub $pxx: $PXX<'a>,)*
            )*
        }

        impl<'a> Pins<'a> {
            pub fn new(gpio: &'a Gpio, ioc: &'a Ioc, pioc: &'a Ioc) -> Self {
                Pins {
                    $(
                        $($pxx: $PXX::new(gpio, ioc, pioc),)*
                    )*
                }
            }
        }
    };
}

pins!(
    'A': [
        (PA07, led,       7,  0),
        (PA25, sdram_0,  25, 12),
        (PA26, sdram_1,  26, 12),
        (PA27, sdram_2,  27, 12),
        (PA28, sdram_3,  28, 12),
        (PA29, sdram_4,  29, 12),
        (PA30, sdram_5,  30, 12),
        (PA31, sdram_6,  31, 12),
    ],
    'B': [
        (PB00, sdram_7,   0, 12),
        (PB01, sdram_8,   1, 12),
        (PB02, sdram_9,   2, 12),
        (PB03, sdram_10,  3, 12),
        (PB04, sdram_11,  4, 12),
        (PB05, sdram_12,  5, 12),
        (PB06, sdram_13,  6, 12),
        (PB07, sdram_14,  7, 12),
        (PB08, sdram_15,  8, 12),
        (PB09, sdram_16,  9, 12),
        (PB10, sdram_17, 10, 12),
        (PB11, sdram_18, 11, 12),
        (PB12, sdram_19, 12, 12),
        (PB13, sdram_20, 13, 12),
        (PB14, sdram_21, 14, 12),
        (PB15, sdram_22, 15, 12),
        (PB16, sdram_23, 16, 12),
        (PB17, sdram_24, 17, 12),
        (PB18, sdram_25, 18, 12),
        (PB19, sdram_26, 19, 12),
        (PB20, sdram_27, 20, 12),
        (PB21, sdram_28, 21, 12),
        (PB22, sdram_29, 22, 12),
        (PB23, sdram_30, 23, 12),
        (PB24, sdram_31, 24, 12),
        (PB25, sdram_32, 25, 12),
        (PB26, sdram_33, 26, 12),
        (PB27, sdram_34, 27, 12),
        (PB28, sdram_35, 28, 12),
        (PB29, sdram_36, 29, 12),
        (PB30, sdram_37, 30, 12),
        (PB31, sdram_38, 31, 12),
    ],
    'Y': [
        (PY06, uart0_tx, 6, 2),
        (PY07, uart0_rx, 7, 2),
    ]
);

impl<'a> Pins<'a> {
    pub fn setup(&self) {
        // Setup LED pinmux
        self.led.output_enable(true).set_open_drain().set_low();
        // Setup UART0 pinmux
        self.uart0_tx.set_mode_alternate();
        self.uart0_rx.set_mode_alternate();
        // Setup SDRAM pinmux
        self.sdram_0.set_mode_alternate();
        self.sdram_1.set_mode_alternate();
        self.sdram_2.set_mode_alternate();
        self.sdram_3.set_mode_alternate();
        self.sdram_4.set_mode_alternate();
        self.sdram_5.set_mode_alternate();
        self.sdram_6.set_mode_alternate();
        self.sdram_7.set_mode_alternate();
        self.sdram_8.set_mode_alternate();
        self.sdram_9.set_mode_alternate();
        self.sdram_10.set_mode_alternate();
        self.sdram_11.set_mode_alternate();
        self.sdram_12.set_mode_alternate();
        self.sdram_13.set_mode_alternate();
        self.sdram_14.set_mode_alternate();
        self.sdram_15.set_mode_alternate();
        self.sdram_16.set_mode_alternate();
        self.sdram_17.set_mode_alternate();
        self.sdram_18.set_mode_alternate();
        self.sdram_19.set_mode_alternate();
        self.sdram_20.set_mode_alternate();
        self.sdram_21.set_mode_alternate();
        self.sdram_22.set_mode_alternate();
        self.sdram_23.set_mode_alternate();
        self.sdram_24.set_mode_alternate();
        self.sdram_25.set_mode_alternate();
        self.sdram_26.set_mode_alternate();
        self.sdram_27.set_mode_alternate();
        self.sdram_28.set_mode_alternate();
        self.sdram_29.set_mode_alternate();
        self.sdram_30.set_mode_alternate();
        self.sdram_31.set_mode_alternate();
        self.sdram_32.set_mode_alternate();
        self.sdram_33.set_mode_alternate();
        self.sdram_34.set_mode_alternate();
        self.sdram_35.set_mode_alternate();
        self.sdram_36.set_mode_alternate();
        self.sdram_37.set_mode_alternate();
        self.sdram_38.set_mode_alternate();
    }
}
