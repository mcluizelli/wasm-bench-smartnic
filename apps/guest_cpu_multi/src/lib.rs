// guest_cpu_matmul/src/lib.rs
use std::slice;

const N: usize = 64;

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
pub extern "C" fn handle(ptr: i32, _len: i32) -> i32 {
    let a = vec![1f32; N*N];
    let b = vec![2f32; N*N];
    let mut c = vec![0f32; N*N];

    for i in 0..N {
        for k in 0..N {
            let aik = a[i*N + k];
            for j in 0..N {
                c[i*N + j] += aik * b[k*N + j];
            }
        }
    }

    unsafe {
        let out = slice::from_raw_parts_mut(ptr as *mut u8, 4);
        out.copy_from_slice(&c[0].to_le_bytes());
    }
    0
}
