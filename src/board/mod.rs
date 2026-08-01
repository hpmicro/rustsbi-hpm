use hpm_metapac as pac;

mod delay;
mod mchtmr;
#[cfg(feature = "sdram-rw-test")]
mod sdram_rw_test;
pub mod uart;

#[cfg(feature = "hpm6360evk")]
#[path = "hpm6360evk/mod.rs"]
mod board_impl;

#[cfg(feature = "hpm6750evkmini")]
#[path = "hpm6750evkmini/mod.rs"]
mod board_impl;

pub use board_impl::{PLATFORM, board_init};
pub use mchtmr::MachineTimer;
pub use uart::{getchar, putchar};

#[macro_export]
macro_rules! print {
    ($fmt: literal $(, $($args: tt)+)?) => {
        $crate::board::putchar(format_args!($fmt $(, $($args)+)?));
    };
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => {{
        $crate::board::putchar(core::format_args!($($arg)*));
        $crate::println!();
    }}
}

pub fn board_init_timer() -> MachineTimer {
    MachineTimer::new(pac::MCHTMR)
}
