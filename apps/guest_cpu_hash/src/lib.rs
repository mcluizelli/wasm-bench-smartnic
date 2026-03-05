// guest_cpu_hash/src/lib.rs
use std::slice;

#[no_mangle]
pub extern "C" fn alloc(len: i32) -> i32 {
    let mut v = Vec::<u8>::with_capacity(len as usize);
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p as i32
}

#[no_mangle]
pub extern "C" fn dealloc(ptr: i32, len: i32) {
    unsafe { Vec::from_raw_parts(ptr as *mut u8, 0, len as usize); }
}

#[no_mangle]
pub extern "C" fn handle(ptr: i32, len: i32) -> i32 {
    let data = unsafe { slice::from_raw_parts(ptr as *const u8, len as usize) };

    let mut h: u64 = 0xcbf29ce484222325;
    for _ in 0..8000 {
        for &b in data {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
    }

    unsafe {
        let out = slice::from_raw_parts_mut(ptr as *mut u8, 8);
        out.copy_from_slice(&h.to_le_bytes());
    }
    0
}
