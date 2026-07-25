use hpm_hal::femc::{Bank2Sel, CasLatency, ColAddrBits, MemorySize, SdramChip, SdramPortSize};

#[cfg(feature = "sdram-rw-test")]
pub const SIZE: usize = 32 * 1024 * 1024;

pub struct SdramInfo {
    pub base_address: usize,
}

pub struct Hpm6360evkSdram;

impl SdramChip for Hpm6360evkSdram {
    fn col_addr_bits(&self) -> ColAddrBits {
        ColAddrBits::_9BIT
    }
    fn cas_latency(&self) -> CasLatency {
        CasLatency::_3
    }
    fn bank_num(&self) -> Bank2Sel {
        Bank2Sel::BANK_NUM_4
    }
    fn size(&self) -> MemorySize {
        MemorySize::_32MB
    }
    fn port_size(&self) -> SdramPortSize {
        SdramPortSize::_16BIT
    }
    fn refresh_count(&self) -> u32 {
        8192
    }
    fn refresh_in_ms(&self) -> u8 {
        64
    }
    fn t_rp(&self) -> u8 {
        18
    }
    fn t_rcd(&self) -> u8 {
        18
    }
    fn t_ras(&self) -> u8 {
        42
    }
    fn t_rc(&self) -> u8 {
        60
    }
    fn t_rrd(&self) -> u8 {
        12
    }
    fn t_wr(&self) -> u8 {
        12
    }
    fn t_xsr(&self) -> u8 {
        72
    }
}

macro_rules! init {
    ($p:ident, $delay:ident) => {{
        let sdram = hpm_hal::femc::Sdram::new_16bit_cs0_a12(
            $p.FEMC,
            // Address pins A0-A12.
            $p.PB18,
            $p.PB19,
            $p.PB20,
            $p.PB21,
            $p.PB31,
            $p.PB30,
            $p.PB29,
            $p.PB28,
            $p.PB27,
            $p.PB26,
            $p.PB17,
            $p.PB25,
            $p.PB24,
            // Bank address BA0-BA1.
            $p.PB15,
            $p.PB16,
            // Data pins DQ0-DQ15.
            $p.PB00,
            $p.PA31,
            $p.PA30,
            $p.PA29,
            $p.PA28,
            $p.PA27,
            $p.PA26,
            $p.PA25,
            $p.PB02,
            $p.PB03,
            $p.PB04,
            $p.PB05,
            $p.PB06,
            $p.PB07,
            $p.PB08,
            $p.PB09,
            // Data mask DM0-DM1.
            $p.PB01,
            $p.PB10,
            // Control: DQS, CLK, CKE, RAS, CAS, WE, CS0.
            $p.PX07,
            $p.PB22,
            $p.PB23,
            $p.PB13,
            $p.PB12,
            $p.PB11,
            $p.PB14,
            $crate::board::board_impl::sdram::Hpm6360evkSdram,
        );

        let base_address = sdram.init(&mut $delay) as usize;

        $crate::board::board_impl::sdram::SdramInfo { base_address }
    }};
}

pub(crate) use init;

#[cfg(feature = "sdram-rw-test")]
pub fn run_rw_test(base_address: usize) {
    crate::println!(
        "[rustsbi pre-init] SDRAM RW test          : full {:#x} bytes",
        SIZE
    );
    unsafe { crate::board::sdram_rw_test::test(base_address, SIZE) };
    crate::println!("[rustsbi pre-init] SDRAM RW test          : passed");
}
