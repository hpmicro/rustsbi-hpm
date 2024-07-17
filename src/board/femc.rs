use core::mem;
use core::slice;

use super::pac::femc::{vals::SdramCmd, Femc};

pub struct Sdram {
    femc: Femc,
}

pub struct FemcCmd {
    opcode: SdramCmd,
    is_write: bool,
    data: u32,
}

pub struct SdramConfigured {
    _femc: Femc,
    base_address: usize,
}

impl Sdram {
    const BASE_ADDRESS: usize = 0x4000_0000;

    pub fn new(femc: Femc) -> Self {
        let sdram = Self { femc };
        let femc = &sdram.femc;

        // FEMC reset
        femc.br(0).write(|w| w.0 = 0);
        femc.br(1).write(|w| w.0 = 0);
        sdram.reset();
        sdram.disable();

        femc.bmw0().write(|w| w.0 = 0x00030524);
        femc.bmw1().write(|w| w.0 = 0x06030524);

        sdram.enable();

        sdram
    }

    fn reset(&self) {
        self.femc.ctrl().write(|w| w.set_rst(true));
        while self.femc.ctrl().read().rst() {}
    }

    fn enable(&self) {
        self.femc.ctrl().modify(|m| m.set_dis(false));
    }

    fn disable(&self) {
        // Wait while FEMC is busy
        while !self.femc.stat0().read().idle() {}
        // Disable FEMC
        self.femc.ctrl().modify(|m| m.set_dis(true));
    }

    fn config_delay_cell(&self, delay_cell_value: u8) {
        let femc = &self.femc;

        femc.dlycfg().modify(|m| m.set_oe(false));
        femc.dlycfg().modify(|m| {
            m.set_dlysel(delay_cell_value);
            m.set_dlyen(true);
        });
        femc.dlycfg().modify(|m| m.set_oe(true));
    }

    fn issue_ip_cmd(&self, base_addr: u32, cmd: &mut FemcCmd) {
        let femc = &self.femc;

        femc.saddr().write(|w| w.set_sa(base_addr));
        if cmd.is_write {
            femc.iptx().write(|w| w.set_dat(cmd.data));
        }

        femc.ipcmd().write(|w| {
            w.set_cmd(cmd.opcode);
            w.set_key(0xA55A);
        });
        self.wait_ip_cmd_done().unwrap();

        if !cmd.is_write {
            cmd.data = femc.iprx().read().0;
        }
    }

    fn wait_ip_cmd_done(&self) -> Result<(), i32> {
        let femc = &self.femc;
        let mut retry = 5000;
        let mut r = Default::default();

        while retry != 0 {
            retry -= 1;

            r = femc.intr().read();
            if r.ipcmddone() || r.ipcmderr() {
                break;
            }
        }

        // Timeout
        if retry == 0 {
            return Err(-1);
        }

        // Clear status register
        femc.intr().modify(|m| {
            // Write 1 to clear
            m.set_ipcmddone(true);
            m.set_ipcmderr(true);
        });

        if r.ipcmderr() {
            return Err(-2);
        }
        Ok(())
    }

    pub fn config(self) -> SdramConfigured {
        let femc = &self.femc;

        // Set base address: 0x40000000, 32MB, CS0 valid
        femc.br(0)
            .write(|w| w.0 = (Sdram::BASE_ADDRESS | 0x1B) as u32);
        // Update SDRAM control
        femc.sdrctrl0().write(|w| w.0 = 0x00000F31);
        femc.sdrctrl1().write(|w| w.0 = 0x00774B33);
        femc.sdrctrl2().write(|w| w.0 = 0x01020B0B);
        femc.sdrctrl3().write(|w| w.0 = 0x1B1B0300);
        // Config data size
        femc.datsz().write(|w| w.0 = 0);
        femc.bytemsk().write(|w| w.0 = 0);
        // Config delay cell
        self.config_delay_cell(29);

        // Issue IP command
        let mut cmd = FemcCmd {
            opcode: SdramCmd::PRECHARGE_ALL,
            is_write: false,
            data: 0,
        };
        self.issue_ip_cmd(Sdram::BASE_ADDRESS as u32, &mut cmd);

        cmd.opcode = SdramCmd::AUTO_REFRESH;
        self.issue_ip_cmd(Sdram::BASE_ADDRESS as u32, &mut cmd);

        cmd.opcode = SdramCmd::MODE_SET;
        cmd.is_write = true;
        cmd.data = 0x33;
        self.issue_ip_cmd(Sdram::BASE_ADDRESS as u32, &mut cmd);

        // Refresh enable
        femc.sdrctrl3().modify(|m| m.set_ren(true));

        SdramConfigured {
            _femc: self.femc,
            base_address: Sdram::BASE_ADDRESS,
        }
    }
}

impl SdramConfigured {
    pub fn base_address(&self) -> usize {
        self.base_address
    }
}

#[allow(unused)]
pub unsafe fn sdram_rw_test() {
    const TEST_PATTERN: u32 = 0xA55A5AA5;

    let dst = slice::from_raw_parts_mut(
        Sdram::BASE_ADDRESS as *mut u32,
        32 * 1024 * 1024 / mem::size_of::<u32>(),
    );
    let mut start = Sdram::BASE_ADDRESS;
    let end = Sdram::BASE_ADDRESS + 32 * 1024 * 1024;

    dst.fill(TEST_PATTERN);

    assert_eq!(
        dst.iter().try_fold(Sdram::BASE_ADDRESS, |addr, x| {
            if *x == TEST_PATTERN {
                Ok(addr + mem::size_of_val(x))
            } else {
                Err(addr)
            }
        }),
        Ok(end)
    );
}
