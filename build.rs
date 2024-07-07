use hpm_rt::*;

fn main() {
    RuntimeBuilder::from_ram(Family::HPM6700_6400)
        .build()
        .unwrap();

    println!("cargo:rerun-if-changed=build.rs");
}
