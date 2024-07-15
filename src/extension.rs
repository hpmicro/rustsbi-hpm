use rustsbi::RustSBI;

#[derive(RustSBI)]
pub struct FixedRustSBI {
    // todo: timer: Option<HpmTimer>,
    // todo: reset: Option<HpmReset>,
}

pub static SBI: FixedRustSBI = FixedRustSBI {
    // todo contents
};

// todo: struct HpmTimer
// todo: impl rustsbi::Timer for HpmTimer

// todo: other extensions...
