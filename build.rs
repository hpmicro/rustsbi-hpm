#![allow(unused)]

use hpm_rt::*;

fn boot_from_flash() {
    let xpi_nor_cfg = XpiNorConfigurationOption::new();

    RuntimeBuilder::load_from_flash(Family::HPM6300, xpi_nor_cfg)
        .xpi0_flash_size(16 * 1024 * 1024)
        .build()
        .unwrap();
}

fn boot_from_ram() {
    RuntimeBuilder::from_ram(Family::HPM6300)
        .stack(MemoryType::Dlm, 8 * 1024)
        .build()
        .unwrap();
}

fn main() {
    #[cfg(feature = "ram")]
    boot_from_ram();

    #[cfg(feature = "flash")]
    boot_from_flash();

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/kernel.bin");
}
