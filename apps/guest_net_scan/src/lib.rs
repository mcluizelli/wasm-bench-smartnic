// guest_net_scan/src/lib.rs
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

    let mut matches = 0u32;
    for w in data.windows(4) {
        if w == b"HTTP" || w == b"GET " || w == b"POST" {
            matches += 1;
        }
    }

    unsafe {
        let out = slice::from_raw_parts_mut(ptr as *mut u8, 4);
        out.copy_from_slice(&matches.to_le_bytes());
    }
    0
}
