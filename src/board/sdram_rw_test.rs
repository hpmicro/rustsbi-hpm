use core::{mem, ptr};

const TEST_PATTERN: u32 = 0xA55A5AA5;

pub unsafe fn test(base_address: usize, size: usize) {
    let words = size / mem::size_of::<u32>();
    let base = base_address as *mut u32;

    for index in 0..words {
        unsafe { ptr::write_volatile(base.add(index), TEST_PATTERN) };
    }

    core::arch::asm!("fence iorw, iorw");

    if hpm_rt::cache::dcache_is_enabled() {
        hpm_rt::cache::dcache_writeback(base_address, size);
        hpm_rt::cache::dcache_invalidate(base_address, size);
    }

    core::arch::asm!("fence iorw, iorw");

    for index in 0..words {
        let addr = base_address + index * mem::size_of::<u32>();
        let actual = unsafe { ptr::read_volatile(base.add(index)) };
        if actual != TEST_PATTERN {
            panic!(
                "SDRAM RW test failed at {:#010x}: expected {:#010x}, actual {:#010x}",
                addr, TEST_PATTERN, actual
            );
        }
    }
}
