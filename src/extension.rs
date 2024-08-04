use rustsbi::RustSBI;
use spin::Lazy;

use crate::board::{board_init_timer, MachineTimer};

#[derive(RustSBI)]
pub struct FixedRustSBI {
    #[rustsbi(timer)]
    pub timer: MachineTimer,
}

pub static SBI: Lazy<FixedRustSBI> = Lazy::new(|| FixedRustSBI {
    timer: board_init_timer(),
});
