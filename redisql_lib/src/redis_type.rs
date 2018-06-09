use std::os::raw::{c_char, c_int};

use std::ffi::CString;

#[allow(dead_code)]
#[allow(non_snake_case)]
#[allow(non_camel_case_types)]
#[allow(non_upper_case_globals)]
#[allow(improper_ctypes)]
pub mod ffi {
    #![allow(clippy)]
    include!(concat!(
        env!("OUT_DIR"),
        "/bindings_redis.rs"
    ));
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
    ptr: *mut ffi::RedisModuleString,
    ctx: Context,
}

impl RMString {
    pub fn new(ctx: Context, s: &str) -> RMString {
        let ptr = unsafe {
            ffi::RedisModule_CreateString.unwrap()(
                ctx.as_ptr(),
                s.as_ptr() as *const c_char,
                s.len(),
            )
        };
        RMString { ptr, ctx }
    }
    pub fn as_ptr(&self) -> *mut ffi::RedisModuleString {
        self.ptr
    }
}

impl Drop for RMString {
    fn drop(&mut self) {
        unsafe {
            ffi::RedisModule_FreeString.unwrap()(
                self.ctx.as_ptr(),
                self.as_ptr(),
            );
        }
    }
}

#[allow(non_snake_case)]
pub fn CreateCommand(
    ctx: Context,
    name: String,
    f: ffi::RedisModuleCmdFunc,
    flags: String,
) -> i32 {
    let command_c_name = CString::new(name).unwrap();
    let command_ptr_name = command_c_name.as_ptr();

    let flag_c_name = CString::new(flags).unwrap();
    let flag_ptr_name = flag_c_name.as_ptr();

    unsafe {
        ffi::RedisModule_CreateCommand.unwrap()(
            ctx.as_ptr(),
            command_ptr_name,
            f,
            flag_ptr_name,
            0,
            0,
            0,
        )
    }
}

#[allow(non_snake_case)]
pub fn ReplicateVerbatim(ctx: Context) -> i32 {
    unsafe {
        ffi::RedisModule_ReplicateVerbatim.unwrap()(ctx.as_ptr())
    }
}

#[allow(non_snake_case)]
pub unsafe fn Replicate(
    ctx: Context,
    command: &str,
    argv: *mut *mut ffi::RedisModuleString,
    argc: c_int,
) -> i32 {
    let command = CString::new(command).unwrap();
    let v = CString::new("v").unwrap();
    ffi::RedisModule_Replicate.unwrap()(
        ctx.as_ptr(),
        command.as_ptr(),
        v.as_ptr(),
        argv.offset(1),
        argc - 1,
    )
}

#[allow(non_snake_case)]
pub fn ReplyWithError(ctx: Context, error: &str) -> i32 {
    unsafe {
        ffi::RedisModule_ReplyWithError.unwrap()(
            ctx.as_ptr(),
            error.as_ptr() as *const c_char,
        )
    }
}

#[allow(non_snake_case)]
pub fn OpenKey(
    ctx: Context,
    name: &RMString,
    mode: i32,
) -> *mut ffi::RedisModuleKey {
    unsafe {
        ffi::Export_RedisModule_OpenKey(ctx.as_ptr(), name.ptr, mode)
    }
}

/*
#[allow(non_snake_case)]
pub fn LoadStringBuffer(rdb: *mut rm::ffi::RedisModuleIO,
                        dimension: &mut usize)
                        ->  {
    unsafe { ffi::RedisModule_LoadStringBuffer(rdb, dimension) }
}
*/

#[allow(non_snake_case)]
pub unsafe fn LoadSigned(rdb: *mut ffi::RedisModuleIO) -> i64 {
    ffi::RedisModule_LoadSigned.unwrap()(rdb) as i64
}

#[allow(non_snake_case)]
pub unsafe fn SaveSigned(rdb: *mut ffi::RedisModuleIO, to_save: i64) {
    ffi::RedisModule_SaveSigned.unwrap()(rdb, to_save)
}

#[allow(non_snake_case)]
pub unsafe fn SaveStringBuffer(
    rdb: *mut ffi::RedisModuleIO,
    buffer: &[u8],
) {
    let ptr = buffer.as_ptr() as *const c_char;
    let len = buffer.len();
    ffi::RedisModule_SaveStringBuffer.unwrap()(rdb, ptr, len)
}

#[allow(non_snake_case)]
pub fn ReplyWithNull(ctx: Context) -> i32 {
    unsafe { ffi::RedisModule_ReplyWithNull.unwrap()(ctx.as_ptr()) }
}

#[allow(non_snake_case)]
pub fn ReplyWithLongLong(ctx: Context, ll: i64) -> i32 {
    unsafe {
        ffi::RedisModule_ReplyWithLongLong.unwrap()(ctx.as_ptr(), ll)
    }
}

#[allow(non_snake_case)]
pub fn ReplyWithDouble(ctx: Context, dd: f64) -> i32 {
    unsafe {
        ffi::RedisModule_ReplyWithDouble.unwrap()(ctx.as_ptr(), dd)
    }
}

#[allow(non_snake_case)]
pub fn ReplyWithStringBuffer(ctx: Context, buffer: &[u8]) -> i32 {
    let ptr = buffer.as_ptr() as *const c_char;
    let len = buffer.len();
    unsafe {
        ffi::RedisModule_ReplyWithStringBuffer.unwrap()(
            ctx.as_ptr(),
            ptr,
            len,
        )
    }
}

pub struct AOF {
    aof: *mut ffi::RedisModuleIO,
}

impl AOF {
    pub fn new(aof: *mut ffi::RedisModuleIO) -> AOF {
        AOF { aof }
    }
    pub fn as_ptr(&self) -> *mut ffi::RedisModuleIO {
        self.aof
    }
}

#[allow(non_snake_case)]
pub unsafe fn EmitAOF(
    aof: &AOF,
    command: &str,
    specifier: &str,
    key: *mut ffi::RedisModuleString,
    data: &str,
) {
    ffi::RedisModule_EmitAOF.unwrap()(
        aof.as_ptr(),
        command.as_ptr() as *const c_char,
        specifier.as_ptr() as *const c_char,
        key,
        data.as_ptr(),
    )
}
