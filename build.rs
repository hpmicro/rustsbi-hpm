use hpm_rt::*;

fn main() {
    let xpi_nor_cfg = XpiNorConfigurationOption::new();

    RuntimeBuilder::from_flash(Family::HPM6700_6400, xpi_nor_cfg)
        .xpi0_flash_size(8 * 1024 * 1024)
        .build()
        .unwrap();

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/kernel.bin");
}
