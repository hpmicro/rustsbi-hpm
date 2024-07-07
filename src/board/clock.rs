#![allow(unused)]

use hpm_ral::{modify_reg, read_reg, write_reg};
use hpm_ral::{pllctl, sysctl};

const XTAL24M_FREQ: u32 = 24_000_000;

pub struct ClockConfigurator {
    sysctl: sysctl::SYSCTL,
    pllctl: pllctl::PLLCTL,
}

impl ClockConfigurator {
    pub fn new(sysctl: sysctl::SYSCTL, pllctl: pllctl::PLLCTL) -> Self {
        ClockConfigurator { sysctl, pllctl }
    }

    pub unsafe fn freeze(self) -> Clocks {
        // Enable peripheral clocks
        modify_reg!(
            sysctl,
            self.sysctl,
            GROUP0_0_VALUE,
            GPIO0_1: Linked,
            MCHTMR0: Linked,
        );
        modify_reg!(
            sysctl,
            self.sysctl,
            GROUP0_1_VALUE,
            UARTO: Linked,
        );
        // Set AHB clock source to PLL1 clock 1 and divider to 2 (200 MHz)
        modify_reg!(sysctl, self.sysctl, CLOCK_CLK_TOP_AHB, MUX: 3, DIV: 2);
        // Set UART0 clock source to osc24 and divider to 1 (24 MHz)
        modify_reg!(sysctl, self.sysctl, CLOCK_CLK_TOP_UART0, MUX: 0, DIV: 0);

        Clocks {
            sysctl: self.sysctl,
            pllctl: self.pllctl,
        }
    }
}

pub enum Pll {
    Pll0,
    Pll1,
    Pll2,
    Pll3,
    Pll4,
}

pub enum ClockSource {
    Osc0Clock0,
    Pll0Clock0,
    Pll1Clock0,
    Pll1Clock1,
    Pll2Clock0,
    Pll2Clock1,
    Pll3Clock0,
    Pll4Clock0,
}

#[derive(Clone, Copy)]
pub enum ClockName {
    CPU0,
    MCHTMR0,
    UART0,
    UART9,
    SPI1,
    SPI2,
}

pub struct Clocks {
    sysctl: sysctl::SYSCTL,
    pllctl: pllctl::PLLCTL,
}

macro_rules! pll_int_freq {
    ($PLLCTL:expr, $PLLx_CFG0:ident, $PLLx_CFG2:ident) => {{
        let refdiv = read_reg!(pllctl, $PLLCTL, $PLLx_CFG0, REFDIV);
        let fbdiv_int = read_reg!(pllctl, $PLLCTL, $PLLx_CFG2, FBDIV_INT);
        let postdiv = read_reg!(pllctl, $PLLCTL, $PLLx_CFG0, POSTDIV1);
        XTAL24M_FREQ / refdiv * fbdiv_int / postdiv
    }};
}

impl Clocks {
    /// When work in integer mode, the frequency of PLL is:
    ///
    /// $$F_{OUT} = F_{REF} \div REFDIV \times FBDIV\_INT \div POSDIV$$
    pub fn get_pll_freq(&self, pll: Pll) -> u32 {
        match pll {
            Pll::Pll0 => pll_int_freq!(self.pllctl, PLL_PLL0_CFG0, PLL_PLL0_CFG2),
            Pll::Pll1 => pll_int_freq!(self.pllctl, PLL_PLL1_CFG0, PLL_PLL1_CFG2),
            Pll::Pll2 => pll_int_freq!(self.pllctl, PLL_PLL2_CFG0, PLL_PLL2_CFG2),
            Pll::Pll3 => pll_int_freq!(self.pllctl, PLL_PLL3_CFG0, PLL_PLL3_CFG2),
            Pll::Pll4 => pll_int_freq!(self.pllctl, PLL_PLL4_CFG0, PLL_PLL4_CFG2),
        }
    }

    pub fn get_clk_src_freq(&self, src: ClockSource) -> u32 {
        match src {
            ClockSource::Osc0Clock0 => XTAL24M_FREQ,
            ClockSource::Pll0Clock0 => self.get_pll_freq(Pll::Pll0),
            ClockSource::Pll1Clock0 => self.get_pll_freq(Pll::Pll1) / 3,
            ClockSource::Pll1Clock1 => self.get_pll_freq(Pll::Pll1) / 2,
            ClockSource::Pll2Clock0 => self.get_pll_freq(Pll::Pll2) / 3,
            ClockSource::Pll2Clock1 => self.get_pll_freq(Pll::Pll2) / 4,
            ClockSource::Pll3Clock0 => self.get_pll_freq(Pll::Pll3),
            ClockSource::Pll4Clock0 => self.get_pll_freq(Pll::Pll4),
        }
    }

    pub fn get_clk_src(&self, name: ClockName) -> ClockSource {
        let mux = match name {
            ClockName::CPU0 => read_reg!(sysctl, self.sysctl, CLOCK_CLK_TOP_CPU0, MUX),
            ClockName::MCHTMR0 => read_reg!(sysctl, self.sysctl, CLOCK_CLK_TOP_MCHTMR0, MUX),
            ClockName::UART0 => read_reg!(sysctl, self.sysctl, CLOCK_CLK_TOP_UART0, MUX),
            ClockName::UART9 => read_reg!(sysctl, self.sysctl, CLOCK_CLK_TOP_UART9, MUX),
            ClockName::SPI1 => read_reg!(sysctl, self.sysctl, CLOCK_CLK_TOP_SPI1, MUX),
            ClockName::SPI2 => read_reg!(sysctl, self.sysctl, CLOCK_CLK_TOP_SPI2, MUX),
        };
        unsafe { core::mem::transmute(mux as u8) }
    }

    pub fn get_clk_div(&self, name: ClockName) -> u32 {
        match name {
            ClockName::CPU0 => read_reg!(sysctl, self.sysctl, CLOCK_CLK_TOP_CPU0, DIV),
            ClockName::MCHTMR0 => read_reg!(sysctl, self.sysctl, CLOCK_CLK_TOP_MCHTMR0, DIV),
            ClockName::UART0 => read_reg!(sysctl, self.sysctl, CLOCK_CLK_TOP_UART0, DIV),
            ClockName::UART9 => read_reg!(sysctl, self.sysctl, CLOCK_CLK_TOP_UART9, DIV),
            ClockName::SPI1 => read_reg!(sysctl, self.sysctl, CLOCK_CLK_TOP_SPI1, DIV),
            ClockName::SPI2 => read_reg!(sysctl, self.sysctl, CLOCK_CLK_TOP_SPI2, DIV),
        }
    }

    pub fn get_clk_freq(&self, name: ClockName) -> u32 {
        let src = self.get_clk_src(name);
        let div = self.get_clk_div(name);
        self.get_clk_src_freq(src) / (div + 1)
    }

    pub fn get_clk_cpu0_freq(&self) -> u32 {
        self.get_clk_freq(ClockName::CPU0)
    }

    pub fn get_clk_mchtmr0_freq(&self) -> u32 {
        self.get_clk_freq(ClockName::MCHTMR0)
    }

    pub fn get_clk_uart0_freq(&self) -> u32 {
        self.get_clk_freq(ClockName::UART0)
    }

    pub fn get_clk_uart9_freq(&self) -> u32 {
        self.get_clk_freq(ClockName::UART9)
    }

    pub fn get_clk_spi1_freq(&self) -> u32 {
        self.get_clk_freq(ClockName::SPI1)
    }

    pub fn get_clk_spi2_freq(&self) -> u32 {
        self.get_clk_freq(ClockName::SPI2)
    }
}
