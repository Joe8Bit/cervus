#![feature(lang_items)]
#![feature(alloc)]
#![feature(global_allocator, allocator_api)]
#![feature(const_fn)]
#![feature(untagged_unions)]
#![feature(nll)]
#![no_std]

extern crate hexagon_e;

#[macro_use]
extern crate alloc;

#[macro_use]
pub mod linux;

pub mod env;
pub mod stub;
pub mod allocator;
pub mod uapi;
pub mod backend;
pub mod global;
pub mod sync;
pub mod mutex;
pub mod error;
pub mod system_service;
pub mod slab;
pub mod resource;
pub mod url;
pub mod api;
pub mod ipc;
pub mod schemes;
pub mod memory_pressure;

use allocator::KernelAllocator;

#[global_allocator]
pub static ALLOCATOR: KernelAllocator = KernelAllocator;

use backend::common::*;

#[lang = "panic_fmt"]
#[no_mangle]
pub extern "C" fn __cv_panic_fmt(_args: core::fmt::Arguments, _file: &'static str, _line: u32) -> ! {
    linux::kernel_panic(_file);
}

#[lang = "oom"]
#[no_mangle]
pub extern "C" fn __cv_oom() -> ! {
    linux::kernel_panic("cervus: Out of memory");
}

fn run_in_usermode_context<B: Backend<Config = G>, G>(
    code: &[u8],
    config: G,
    kctx: *mut u8
) -> BackendResult<()> {
    let mut executor = B::new(config)?;
    let mut context = env::UsermodeContext::new(kctx);
    executor.run(code, &mut context)?;
    Ok(())
}

#[no_mangle]
pub extern "C" fn map_cwa_api(name_base: *const u8, name_len: usize) -> i32 {
    let name = unsafe { ::core::slice::from_raw_parts(name_base, name_len) };
    let name = match ::core::str::from_utf8(name) {
        Ok(v) => v,
        Err(_) => return -1
    };

    env::UsermodeContext::map_cwa_api_to_native_invoke(name)
        .map(|v| v as i32)
        .unwrap_or(-1)
}

#[no_mangle]
pub extern "C" fn run_code_in_hexagon_e(
    code_base: *const u8,
    code_len: usize,
    mem_default_len: usize,
    mem_max_len: usize,
    max_slots: usize,
    stack_len: usize,
    call_stack_len: usize,
    kctx: *mut u8
) -> i32 {
    let code = unsafe { ::core::slice::from_raw_parts(code_base, code_len) };
    let config = backend::hexagon_e::EnvConfig {
        memory_default_len: mem_default_len,
        memory_max_len: mem_max_len,
        max_slots: max_slots,
        stack_len: stack_len,
        call_stack_len: call_stack_len
    };

    println!("loading code with configuration {:?}", config);

    let result = run_in_usermode_context::<backend::hexagon_e::HexagonEBackend, _>(
        code,
        config,
        kctx
    );

    match result {
        Ok(_) => 0,
        Err(e) => {
            println!("execution terminated with error: {:?}", e);
            e.status()
        }
    }
}
