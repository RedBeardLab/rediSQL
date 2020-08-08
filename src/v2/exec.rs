use std::ffi::CString;

use parser::common::CommandV2;
use parser::exec::Exec;
use parser::exec::ToExecute;

use redisql_lib::redis as r;
use redisql_lib::redis::do_execute;
use redisql_lib::redis::do_query;
use redisql_lib::redis::LoopData;
use redisql_lib::redis::RedisReply;
use redisql_lib::redis::Returner;
use redisql_lib::redis::StatementCache;
use redisql_lib::redis_type::BlockedClient;
use redisql_lib::redis_type::Context;
use redisql_lib::redis_type::ReplicateVerbatim;

use crate::common::{free_privdata, reply_v2, timeout};

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
            return error.reply_v2(&context);
        }
    };
    let command: Exec = match CommandV2::parse(argvector) {
        Ok(comm) => comm,
        Err(mut e) => return e.reply_v2(&context),
    };
    do_exec_v2(command, context)
}

#[allow(non_snake_case)]
pub extern "C" fn Query_v2(
    ctx: *mut r::rm::ffi::RedisModuleCtx,
    argv: *mut *mut r::rm::ffi::RedisModuleString,
    argc: ::std::os::raw::c_int,
) -> i32 {
    let context = r::rm::Context::new(ctx);
    let argvector = match r::create_argument(argv, argc) {
        Ok(argvector) => argvector,
        Err(mut error) => {
            return error.reply_v2(&context);
        }
    };
    let mut command: Exec = match CommandV2::parse(argvector) {
        Ok(comm) => comm,
        Err(mut e) => return e.reply_v2(&context),
    };
    command.make_into_query();
    do_exec_v2(command, context)
}

fn do_exec_v2(command: Exec<'static>, context: Context) -> i32 {
    let t = std::time::Instant::now()
        + std::time::Duration::from_secs(10);
    let key = command.key(&context);
    if !command.is_now() {
        match key.get_channel() {
            Err(mut e) => e.reply_v2(&context),
            Ok(ch) => {
                let blocked_client = BlockedClient::new(
                    &context,
                    reply_v2,
                    timeout,
                    free_privdata,
                    10_000,
                );
                let repl_args = command.replicate_args(&context);
                let comm = command.get_command(t, blocked_client);
                match ch.send(comm) {
                    Err(e) => {
                        dbg!(
                            "Error in sending the command!",
                            e.to_string()
                        );
                        r::rm::ffi::REDISMODULE_OK
                    }
                    Ok(_) => {
                        if let Some(repl_args) = repl_args {
                            let command =
                                CString::new("REDISQL.V2.EXEC")
                                    .unwrap();
                            let format = CString::new("v").unwrap();
                            unsafe {
                                r::rm::ffi::RedisModule_Replicate
                                    .unwrap()(
                                    context.as_ptr(),
                                    command.as_ptr(),
                                    format.as_ptr(),
                                    repl_args.as_ptr(),
                                    repl_args.len(),
                                );
                                for ptr in repl_args {
                                    r::rm::ffi::RedisModule_FreeString.unwrap()(context.as_ptr(), ptr);
                                }
                            }
                        }
                        r::rm::ffi::REDISMODULE_OK
                    }
                }
            }
        }
    } else {
        let db = match key.get_db() {
            Ok(k) => k,
            Err(mut e) => return e.reply_v2(&context),
        };
        let read_only = command.is_read_only();
        let return_method = command.get_return_method();
        let to_execute = command.get_to_execute();
        match to_execute {
            ToExecute::Command { query, args } => {
                let mut res = match read_only {
                    true => match do_query(&db, query, args) {
                        Ok(r) => r.create_data_to_return(
                            &context,
                            &return_method,
                            t,
                        ),
                        Err(e) => e.create_data_to_return(
                            &context,
                            &return_method,
                            t,
                        ),
                    },
                    false => match do_execute(&db, query, args) {
                        Ok(r) => {
                            ReplicateVerbatim(&context);
                            r.create_data_to_return(
                                &context,
                                &return_method,
                                t,
                            )
                        }
                        Err(e) => e.create_data_to_return(
                            &context,
                            &return_method,
                            t,
                        ),
                    },
                };
                res.reply_v2(&context)
            }
            ToExecute::Statement { stmt, args } => {
                let loop_data = match key.get_loop_data() {
                    Ok(k) => k,
                    Err(mut e) => return e.reply_v2(&context),
                };

                let mut result = match read_only {
                    true => {
                        match loop_data
                            .get_replication_book()
                            .query_statement(stmt, args)
                        {
                            Ok(r) => r.create_data_to_return(
                                &context,
                                &return_method,
                                t,
                            ),
                            Err(e) => e.create_data_to_return(
                                &context,
                                &return_method,
                                t,
                            ),
                        }
                    }
                    false => {
                        match loop_data
                            .get_replication_book()
                            .exec_statement(stmt, args)
                        {
                            Ok(r) => r.create_data_to_return(
                                &context,
                                &return_method,
                                t,
                            ),
                            Err(e) => e.create_data_to_return(
                                &context,
                                &return_method,
                                t,
                            ),
                        }
                    }
                };
                result.reply_v2(&context)
            }
        }
    }
}
