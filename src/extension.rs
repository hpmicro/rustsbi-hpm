use rustsbi::RustSBI;
use spin::LazyLock;

use crate::board::{MachineTimer, board_init_timer};

#[derive(RustSBI)]
pub struct FixedRustSBI {
    #[rustsbi(timer)]
    pub timer: MachineTimer,
}

pub static SBI: LazyLock<FixedRustSBI> = LazyLock::new(|| FixedRustSBI {
    timer: board_init_timer(),
});
