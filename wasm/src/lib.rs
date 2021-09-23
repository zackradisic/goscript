use std::{ffi::CStr, os::raw::c_char};

use goscript_engine::{Config, Engine};

#[no_mangle]
pub extern "C" fn _start() {
    println!("WASI Module instance started");
}

#[no_mangle]
pub extern "C" fn run_go(path_ptr: *const c_char) -> usize {
    let path: &str = unsafe {
        assert!(!path_ptr.is_null());
        CStr::from_ptr(path_ptr).to_str().unwrap()
    };

    let cfg = Config {
        work_dir: Some("/".to_string()),
        base_path: Some("/std".to_string()),
        trace_parser: false,
        trace_checker: false,
        trace_vm: false,
    };

    let engine = Engine::new(cfg);
    engine.run(path)
}

#[no_mangle]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    let mut buf = Vec::with_capacity(len);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

#[no_mangle]
pub extern "C" fn dealloc(ptr: *mut u8, len: usize) {
    let data = unsafe { Vec::from_raw_parts(ptr, len, len) };

    std::mem::drop(data);
}
