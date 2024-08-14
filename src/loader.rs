use crate::{DTB_LOAD_ADDRESS, SUPERVISOR_ENTRY};

#[derive(PartialEq)]
enum BlobType {
    Kernel,
    Dtb,
}
struct BlobInfo {
    type_: BlobType,
    start: usize,
    length: usize,
}
/// # Blob Info Table
///
/// | Name    | Begin      | Length |
/// |---------|------------|--------|
/// | RustSBI | 0x80000000 | 64 KB  |
/// | Kernel  | 0x80010000 | 3 MB   |
/// | DTB     | 0x80310000 | 16 KB  |
///
const BLOB_TABLE: &'static [BlobInfo] = &[
    BlobInfo {
        type_: BlobType::Kernel,
        start: 0x80010000,
        length: 3 * 1024 * 1024,
    },
    BlobInfo {
        type_: BlobType::Dtb,
        start: 0x80310000,
        length: 16 * 1024,
    },
];

impl BlobInfo {
    unsafe fn load(&self, load_address: *mut u8) {
        let src: &[u8] = core::slice::from_raw_parts(self.start as *mut _, self.length);
        let dst: &mut [u8] = core::slice::from_raw_parts_mut(load_address, self.length);
        dst.copy_from_slice(src);
    }

    #[allow(unused)]
    unsafe fn compare(&self, load_address: *const u8) {
        let src: &[u8] = core::slice::from_raw_parts(self.start as *const _, self.length);
        let dst: &[u8] = core::slice::from_raw_parts(load_address, self.length);

        assert!(src.eq(dst))
    }
}

pub unsafe fn load_kernel() {
    let info: &BlobInfo = &BLOB_TABLE[0];
    assert!(info.type_ == BlobType::Kernel);
    assert!(info.start + info.length <= BLOB_TABLE[1].start);

    info.load(SUPERVISOR_ENTRY as *mut _);
}

pub unsafe fn load_dtb() {
    let info: &BlobInfo = &BLOB_TABLE[1];
    assert!(info.type_ == BlobType::Dtb);

    info.load(DTB_LOAD_ADDRESS as *mut _);
}
