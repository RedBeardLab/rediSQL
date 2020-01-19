use parser::common::CommandV2;
use parser::exec::Exec;

use redisql_lib::redis as r;
use redisql_lib::redis::do_execute;
use redisql_lib::redis::LoopData;
use redisql_lib::redis::RedisReply;
use redisql_lib::redis::Returner;
use redisql_lib::redis_type::BlockedClient;
use redisql_lib::redis_type::ReplicateVerbatim;

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
    if !command.is_now() {
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
                    Err(e) => {
                        dbg!(
                            "Error in sending the command!",
                            e.to_string()
                        );
                        r::rm::ffi::REDISMODULE_OK
                    }
                    _ => r::rm::ffi::REDISMODULE_OK,
                }
            }
        }
    } else {
        let dbkey = match key.get_dbkey() {
            Ok(k) => k,
            Err(mut e) => return e.reply(&context),
        };
        let db = dbkey.loop_data.get_db();
        let result = do_execute(
            &db,
            command.get_query().expect("todo if panic"),
        );
        let t = std::time::Instant::now()
            + std::time::Duration::from_secs(10);
        let return_method = command.get_return_method();
        let mut result = match result {
            Ok(r) => {
                ReplicateVerbatim(&context);
                r.create_data_to_return(&context, &return_method, t)
            }
            Err(e) => {
                e.create_data_to_return(&context, &return_method, t)
            }
        };
        result.reply(&context)
    }
}
