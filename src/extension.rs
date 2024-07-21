use rustsbi::RustSBI;

use crate::board::{board_init_timer, MachineTimer};

#[derive(RustSBI)]
pub struct FixedRustSBI {
    #[rustsbi(timer)]
    pub timer: MachineTimer,
}

lazy_static! {
    pub static ref SBI: FixedRustSBI = {
        FixedRustSBI {
            timer: board_init_timer(),
        }
    };
}
