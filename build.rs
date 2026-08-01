use hpm_rt::*;

#[cfg(feature = "hpm6360evk")]
const FAMILY: Device = Family::HPM6300;

#[cfg(feature = "hpm6750evkmini")]
const FAMILY: Device = Family::HPM6700_6400;

#[cfg(all(feature = "flash", feature = "hpm6360evk"))]
const XPI0_FLASH_SIZE: u32 = 16 * 1024 * 1024;

#[cfg(all(feature = "flash", feature = "hpm6750evkmini"))]
const XPI0_FLASH_SIZE: u32 = 8 * 1024 * 1024;

#[cfg(feature = "flash")]
fn boot_from_flash() {
    let xpi_nor_cfg = XpiNorConfigurationOption::new();

    RuntimeBuilder::load_from_flash(FAMILY, xpi_nor_cfg)
        .xpi0_flash_size(XPI0_FLASH_SIZE)
        .build()
        .unwrap();
}

#[cfg(feature = "ram")]
fn boot_from_ram() {
    RuntimeBuilder::from_ram(FAMILY)
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
