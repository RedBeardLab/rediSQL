use std::os::raw::c_char;

use std::ffi::CString;

#[allow(dead_code)]
#[allow(non_snake_case)]
#[allow(non_camel_case_types)]
#[allow(non_upper_case_globals)]
#[allow(improper_ctypes)]
pub mod ffi {
    include!(concat!(env!("OUT_DIR"), "/bindings_redis.rs"));
}


#[allow(dead_code)]
#[derive(Copy, Clone)]
pub struct Context {
    ctx: *mut ffi::RedisModuleCtx,
}

impl Context {
    pub fn new(ctx: *mut ffi::RedisModuleCtx) -> Context {
        Context { ctx }
    }
    pub fn as_ptr(self) -> *mut ffi::RedisModuleCtx {
        self.ctx
    }
}

impl From<Context> for *mut ffi::RedisModuleCtx {
    fn from(c: Context) -> *mut ffi::RedisModuleCtx {
        c.ctx
    }
}

pub struct RMString {
    pub ptr: *mut ffi::RedisModuleString,
    ctx: *mut ffi::RedisModuleCtx,
}

impl RMString {
    pub fn new(ctx: *mut ffi::RedisModuleCtx, s: &str) -> RMString {
        let ptr = unsafe {
            ffi::RedisModule_CreateString.unwrap()(ctx,
                                                   s.as_ptr() as
                                                   *const c_char,
                                                   s.len())
        };
        RMString { ptr, ctx }
    }
}

impl Drop for RMString {
    fn drop(&mut self) {
        unsafe {
            ffi::RedisModule_FreeString.unwrap()(self.ctx, self.ptr);
        }
    }
}

#[allow(non_snake_case)]
pub fn CreateCommand(ctx: Context,
                     name: String,
                     f: ffi::RedisModuleCmdFunc,
                     flags: String)
                     -> i32 {
    let command_c_name = CString::new(name).unwrap();
    let command_ptr_name = command_c_name.as_ptr();

    let flag_c_name = CString::new(flags).unwrap();
    let flag_ptr_name = flag_c_name.as_ptr();

    unsafe {
        ffi::RedisModule_CreateCommand.unwrap()(ctx.as_ptr(),
                                                command_ptr_name,
                                                f,
                                                flag_ptr_name,
                                                0,
                                                0,
                                                0)

    }
}
