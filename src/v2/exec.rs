use parser::common::CommandV2;
use parser::exec::Exec;
use redisql_lib::redis as r;
use redisql_lib::redis::RedisReply;
use redisql_lib::redis_type::BlockedClient;

use crate::common::{free_privdata, reply, timeout};

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
    let command: Exec = match CommandV2::parse(argvector) {
        Ok(comm) => comm,
        Err(mut e) => return e.reply(&context),
    };

    let key = command.key(&context);
    match key.get_channel() {
        Err(mut e) => e.reply(&context),
        Ok(ch) => {
            let blocked_client = BlockedClient::new(
                &context,
                reply,
                timeout,
                free_privdata,
                10_000,
            );
            let timeout = std::time::Instant::now()
                + std::time::Duration::from_secs(10);
            let command =
                command.get_command(timeout, blocked_client);
            match ch.send(command) {
                _ => r::rm::ffi::REDISMODULE_OK,
            }
        }
    }
}
