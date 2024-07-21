#![allow(unused)]

use super::pac::sysctl::vals::ClockMux;
use super::pac::{pllctl, resources, sysctl};

pub use super::pac::clocks;

const XTAL24M_FREQ: u32 = 24_000_000;

pub struct ClockConfigurator {
    sysctl: sysctl::Sysctl,
    pllctl: pllctl::Pllctlv2,
}

impl ClockConfigurator {
    pub fn new(sysctl: sysctl::Sysctl, pllctl: pllctl::Pllctlv2) -> Self {
        ClockConfigurator { sysctl, pllctl }
    }

    fn link_to_group(&self, resource: usize) {
        const RESOURCE_START: usize = 256;
        assert!(resource > RESOURCE_START);

        let index = (resource - RESOURCE_START) / 32;
        let offset = (resource - RESOURCE_START) % 32;

        self.sysctl
            .group0(index)
            .set()
            .write(|w| w.set_link(1 << offset))
    }

    pub unsafe fn freeze(self) -> Clocks {
        self.link_to_group(resources::GPIO);
        self.link_to_group(resources::MCT0);
        self.link_to_group(resources::URT0);
        self.link_to_group(resources::FEMC);

        self.sysctl.clock(clocks::URT0).modify(|w| {
            w.set_mux(sysctl::vals::ClockMux::CLK_24M);
            w.set_div(0);
        });

        self.sysctl.clock(clocks::FEMC).modify(|w| {
            w.set_mux(sysctl::vals::ClockMux::PLL0CLK1);
            w.set_div(1);
        });

        Clocks {
            sysctl: self.sysctl,
            pllctl: self.pllctl,
        }
    }
}

pub struct Clocks {
    sysctl: sysctl::Sysctl,
    pllctl: pllctl::Pllctlv2,
}

impl Clocks {
    /// When work in integer mode, the frequency of PLL is:
    ///
    /// $$F_{vco} = F_{ref} \times (MFI + (MFN \div MFD))$$
    pub fn get_pll_freq(&self, pll: usize) -> u32 {
        assert!(pll <= 2);
        let r = self.pllctl.pll(pll);
        let mfi = r.mfi().read().mfi() as f64;
        let mfn = r.mfn().read().mfn() as f64 / 10.0;
        let mfd = r.mfd().read().mfd() as f64 / 10.0;

        let _freq = ((XTAL24M_FREQ as f64) * (mfi + (mfn / mfd))) as u32;
        _freq
    }

    pub fn get_clk_src_freq(&self, src: ClockMux) -> u32 {
        match src {
            ClockMux::CLK_24M => XTAL24M_FREQ,
            (ClockMux::PLL0CLK0 | ClockMux::PLL0CLK1 | ClockMux::PLL0CLK2) => {
                let freq = self.get_pll_freq(0) as f64;
                let div = self
                    .pllctl
                    .pll(0)
                    .div(src as usize - ClockMux::PLL0CLK0 as usize)
                    .read()
                    .div() as f64;
                (freq / (1.0 + 0.2 * div)) as _
            }
            (ClockMux::PLL1CLK0 | ClockMux::PLL1CLK1) => {
                let freq = self.get_pll_freq(0) as f64;
                let div = self
                    .pllctl
                    .pll(1)
                    .div(src as usize - ClockMux::PLL1CLK0 as usize)
                    .read()
                    .div() as f64;
                (freq / (1.0 + 0.2 * div)) as _
            }
            (ClockMux::PLL2CLK0 | ClockMux::PLL2CLK1) => {
                let freq = self.get_pll_freq(0) as f64;
                let div = self
                    .pllctl
                    .pll(2)
                    .div(src as usize - ClockMux::PLL2CLK0 as usize)
                    .read()
                    .div() as f64;
                (freq / (1.0 + 0.2 * div)) as _
            }
            _ => unimplemented!(),
        }
    }

    pub fn get_clk_freq(&self, clock: usize) -> u32 {
        let r = self.sysctl.clock(clock).read();
        let src = r.mux();
        let div = r.div() as u32;
        self.get_clk_src_freq(src) / (div + 1)
    }

    pub fn get_cpu0_clk_freq(&self) -> u32 {
        let r = self.sysctl.clock_cpu(0).read();
        let src = r.mux();
        let div = r.div() as u32;
        self.get_clk_src_freq(src) / (div + 1)
    }
}
