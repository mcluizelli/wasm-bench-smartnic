// guest_mem_random/src/lib.rs
use std::slice;

const N: usize = 8 * 1024 * 1024;

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
    let mut mem = vec![0u8; N];

    for i in 0..N {
        let idx = (input[i % len as usize] as usize * 2654435761) % N;
        mem[idx] = mem[idx].wrapping_add(1);
    }
    0
}
