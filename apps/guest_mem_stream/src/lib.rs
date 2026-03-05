// guest_mem_stream/src/lib.rs
use std::slice;

const SIZE: usize = 16 * 1024 * 1024;

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
    let input = unsafe { slice::from_raw_parts(ptr as *const u8, len as usize) };
    let mut buf = vec![0u8; SIZE];

    for i in 0..SIZE {
        buf[i] = input[i % len as usize];
    }

    let sum: u64 = buf.iter().map(|&b| b as u64).sum();

    unsafe {
        let out = slice::from_raw_parts_mut(ptr as *mut u8, 8);
        out.copy_from_slice(&sum.to_le_bytes());
    }
    0
}
