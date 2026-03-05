// guest_net_checksum/src/lib.rs
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

fn checksum(data: &[u8]) -> u32 {
    let mut s = 0u32;
    for chunk in data.chunks(2) {
        let v = if chunk.len()==2 {
            u16::from_be_bytes([chunk[0],chunk[1]]) as u32
        } else {
            chunk[0] as u32
        };
        s = s.wrapping_add(v);
    }
    !s
}

#[no_mangle]
pub extern "C" fn handle(ptr: i32, len: i32) -> i32 {
    let pkt = unsafe { slice::from_raw_parts(ptr as *const u8, len as usize) };
    let c = checksum(pkt);

    unsafe {
        let out = slice::from_raw_parts_mut(ptr as *mut u8, 4);
        out.copy_from_slice(&c.to_be_bytes());
    }
    0
}
