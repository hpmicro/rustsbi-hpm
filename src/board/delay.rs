use embedded_hal::delay::DelayNs;

pub struct CycleDelay {
    cycles_per_second: u32,
}

impl CycleDelay {
    pub fn new(cycles_per_second: u32) -> Self {
        Self { cycles_per_second }
    }
}

impl DelayNs for CycleDelay {
    fn delay_ns(&mut self, ns: u32) {
        let cycles =
            (self.cycles_per_second as u64 * ns as u64).saturating_add(999_999_999) / 1_000_000_000;
        let cycles = cycles.max(1);
        let start = riscv::register::mcycle::read64();

        while riscv::register::mcycle::read64().wrapping_sub(start) < cycles {
            core::hint::spin_loop();
        }
    }
}
