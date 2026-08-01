#[cfg(feature = "sdram-rw-test")]
pub const SIZE: usize = 16 * 1024 * 1024;

pub struct SdramInfo {
    pub base_address: usize,
}

macro_rules! init {
    ($p:ident, $delay:ident) => {{
        let sdram = hpm_hal::femc::Sdram::new_16bit_cs0(
            $p.FEMC,
            // Address pins A0-A11.
            $p.PC08,
            $p.PC09,
            $p.PC04,
            $p.PC05,
            $p.PC06,
            $p.PC07,
            $p.PC10,
            $p.PC11,
            $p.PC12,
            $p.PC17,
            $p.PC15,
            $p.PC21,
            // Bank address BA0-BA1.
            $p.PC13,
            $p.PC14,
            // Data pins DQ0-DQ15.
            $p.PD08,
            $p.PD05,
            $p.PD00,
            $p.PD01,
            $p.PD02,
            $p.PC27,
            $p.PC28,
            $p.PC29,
            $p.PD04,
            $p.PD03,
            $p.PD07,
            $p.PD06,
            $p.PD10,
            $p.PD09,
            $p.PD13,
            $p.PD12,
            // Data mask DM0-DM1.
            $p.PC30,
            $p.PC31,
            // Control: DQS, CLK, CKE, RAS, CAS, WE, CS0.
            $p.PC16,
            $p.PC26,
            $p.PC25,
            $p.PC18,
            $p.PC23,
            $p.PC24,
            $p.PC19,
            hpm_hal::femc::chips::W9812g6jh6,
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
