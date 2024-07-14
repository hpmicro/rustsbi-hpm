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
        (PA07, led, 7, 0),
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
    }
}
