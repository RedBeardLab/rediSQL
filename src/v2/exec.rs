use parser;
use parser::CommandV2;

use redisql_lib::redis as r;
use redisql_lib::redis::RedisReply;

#[allow(non_snake_case)]
pub extern "C" fn Exec_v2(
    ctx: *mut r::rm::ffi::RedisModuleCtx,
    argv: *mut *mut r::rm::ffi::RedisModuleString,
    argc: ::std::os::raw::c_int,
) -> i32 {
    let context = r::rm::Context::new(ctx);
    let argvector = match r::create_argument(argv, argc) {
        Ok(argvector) => argvector,
        Err(mut error) => {
            return error.reply(&context);
        }
    };
    let command: parser::Exec =
        match parser::CommandV2::parse(argvector) {
            Ok(comm) => comm,
            Err(mut e) => return e.reply(&context),
        };

    let key = command.key(&context);
    0
}
