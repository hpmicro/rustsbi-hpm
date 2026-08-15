use hpm_hal::femc::{
    CasLatency, ColAddrBits, MemorySize, SdramChip,
    geometry::{Address13, Banks4, Data16},
};

#[cfg(feature = "sdram-rw-test")]
pub const SIZE: usize = 32 * 1024 * 1024;

pub struct SdramInfo {
    pub base_address: usize,
}

pub struct Hpm6360evkSdram;

impl SdramChip for Hpm6360evkSdram {
    type AddressWidth = Address13;
    type DataWidth = Data16;
    type BankCount = Banks4;

    fn col_addr_bits(&self) -> ColAddrBits {
        ColAddrBits::_9BIT
    }
    fn cas_latency(&self) -> CasLatency {
        CasLatency::_3
    }
    fn size(&self) -> MemorySize {
        MemorySize::_32MB
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
        let sdram = hpm_hal::femc::Sdram::new_cs0(
            $p.FEMC,
            hpm_hal::femc::SdramPins {
                address: (
                    $p.PB18, $p.PB19, $p.PB20, $p.PB21, $p.PB31, $p.PB30, $p.PB29, $p.PB28,
                    $p.PB27, $p.PB26, $p.PB17, $p.PB25, $p.PB24,
                ),
                banks: ($p.PB15, $p.PB16),
                data: (
                    $p.PB00, $p.PA31, $p.PA30, $p.PA29, $p.PA28, $p.PA27, $p.PA26, $p.PA25,
                    $p.PB02, $p.PB03, $p.PB04, $p.PB05, $p.PB06, $p.PB07, $p.PB08, $p.PB09,
                ),
                masks: ($p.PB01, $p.PB10),
                control: hpm_hal::femc::ControlPins {
                    dqs: $p.PX07,
                    clk: $p.PB22,
                    cke: $p.PB23,
                    ras: $p.PB13,
                    cas: $p.PB12,
                    we: $p.PB11,
                    cs: $p.PB14,
                },
            },
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
