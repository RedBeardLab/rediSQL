use redisql_lib::redis as r;

pub extern "C" fn reply(
    ctx: *mut r::rm::ffi::RedisModuleCtx,
    _argv: *mut *mut r::rm::ffi::RedisModuleString,
    _argc: ::std::os::raw::c_int,
) -> i32 {
    let context = r::rm::Context::new(ctx);
    let result = unsafe {
        r::rm::ffi::RedisModule_GetBlockedClientPrivateData.unwrap()(
            context.as_ptr(),
        ) as *mut *mut dyn r::RedisReply
    };
    let result_wrap: Box<*mut dyn r::RedisReply> =
        unsafe { Box::from_raw(result) };
    let mut result: Box<dyn r::RedisReply> =
        unsafe { Box::from_raw(*result_wrap) };
    result.reply(&context)
}

pub extern "C" fn timeout(
    ctx: *mut r::rm::ffi::RedisModuleCtx,
    _argv: *mut *mut r::rm::ffi::RedisModuleString,
    _argc: ::std::os::raw::c_int,
) -> i32 {
    unsafe { r::rm::ffi::RedisModule_ReplyWithNull.unwrap()(ctx) }
}

pub extern "C" fn free_privdata(_arg: *mut ::std::os::raw::c_void) {}
