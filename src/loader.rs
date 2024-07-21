use crate::{DTB_LOAD_ADDRESS, SUPERVISOR_ENTRY};

#[derive(PartialEq)]
enum BlobType {
    Kernel,
    Dts,
}
struct BlobInfo {
    type_: BlobType,
    start: usize,
    length: usize,
}
/// # Blob Info Table
///
/// | Name   | Begin      | Length |
/// |--------|------------|--------|
/// | Kernel | 0x80040000 | 2 MB   |
/// | DTS    | 0x80240000 | 16 KB  |
///
const BLOB_TABLE: &'static [BlobInfo] = &[
    BlobInfo {
        type_: BlobType::Kernel,
        // Keep 256 KB for SBI firmware.
        start: 0x80040000,
        length: 2 * 1024 * 1024,
    },
    BlobInfo {
        type_: BlobType::Dts,
        start: 0x80240000,
        length: 16 * 1024,
    },
];

impl BlobInfo {
    unsafe fn load(&self, load_address: *mut u8) {
        let src: &[u8] = core::slice::from_raw_parts(self.start as *mut _, self.length);
        let dst: &mut [u8] = core::slice::from_raw_parts_mut(load_address, self.length);
        dst.copy_from_slice(src);
    }
}

pub unsafe fn load_test_kernel() {
    let info: &BlobInfo = &BLOB_TABLE[0];
    assert!(info.type_ == BlobType::Kernel);
    assert!(info.start + info.length <= BLOB_TABLE[1].start);

    info.load(SUPERVISOR_ENTRY as *mut u8);
}

pub unsafe fn load_dtb() {
    let info: &BlobInfo = &BLOB_TABLE[1];
    assert!(info.type_ == BlobType::Dts);

    info.load(DTB_LOAD_ADDRESS as *mut u8);
}
