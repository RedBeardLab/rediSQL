extern crate redisql_lib;

use std::collections::vec_deque::VecDeque;
use std::ffi::CString;
use std::mem::zeroed;
use std::mem::ManuallyDrop;
use std::os::raw;
use std::sync::{Arc, Mutex};

use redisql_lib::sqlite::ffi;
use redisql_lib::sqlite::Connection;
use redisql_lib::sqlite::SQLiteConnection;

use redisql_lib::redis as r;
use redisql_lib::redis::{
    do_execute, do_query, get_dbkey_from_name, register_function,
    register_function_with_keys, register_write_function,
    reply_with_error_from_key_type, LoopData, RedisReply,
    ReturnMethod, Returner, StatementCache,
};
use redisql_lib::redis_type::ffi::{
    RedisModuleIO, RedisModuleString,
};
use redisql_lib::redis_type::{Context, ReplicateVerbatim};

struct DumpIterator {
    fd: raw::c_int,
    buffer: [u8; 4096],
    iterator: VecDeque<String>,
    first_chunk: String,
}

impl<'b> DumpIterator {
    fn new(conn: &Arc<Mutex<Connection>>) -> DumpIterator {
        let db = conn.lock().unwrap();
        let buffer: [u8; 4096] = unsafe { zeroed() };
        let fd = unsafe { ffi::start((*db).get_db()) };
        let iterator = VecDeque::new();
        let first_chunk = String::from("");
        DumpIterator {
            fd,
            buffer,
            iterator,
            first_chunk,
        }
    }
}

impl<'b> Drop for DumpIterator {
    fn drop(&mut self) {
        unsafe {
            ffi::close_read_pipe(self.fd);
        }
    }
}

impl Iterator for DumpIterator {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        match self.iterator.len() {
            n if n > 0 => {
                Some(self.iterator.pop_front().unwrap_or_default())
            }
            _ => {
                let read_bytes = unsafe {
                    ffi::read_from_pipe(
                        self.fd,
                        self.buffer.as_mut_ptr() as *mut raw::c_void,
                        4096,
                    )
                };
                match read_bytes {
                    n if n > 0 => {
                        let data = self
                            .buffer
                            .split_at(n as usize)
                            .0
                            .to_vec();
                        let whole_string = unsafe {
                            String::from_utf8_unchecked(data)
                        };
                        self.iterator = whole_string
                            .split('\n')
                            .map(String::from)
                            .collect();

                        let result = match self.iterator.pop_front() {
                            None => None,
                            Some(s) => {
                                Some(self.first_chunk.clone() + &s)
                            }
                        };
                        self.first_chunk = self
                            .iterator
                            .pop_back()
                            .unwrap_or_else(|| String::from(""));
                        result
                    }
                    _ => None,
                }
            }
        }
    }
}

#[allow(non_snake_case, clippy::missing_safety_doc)]
pub unsafe extern "C" fn WriteAOF(
    aof: *mut RedisModuleIO,
    key: *mut RedisModuleString,
    value: *mut raw::c_void,
) {
    let aof = r::rm::AOF::new(aof);
    let dbkey: Box<r::DBKey> = Box::from_raw(value as *mut r::DBKey);

    let db = dbkey.loop_data.get_db();

    r::rm::EmitAOF(&aof, "REDISQL.V1.CREATE_DB", "s", key, "");

    let iter = DumpIterator::new(&db);
    for s in iter {
        for line in s.split('\n').filter(|l| !l.is_empty()) {
            r::rm::EmitAOF(
                &aof,
                "REDISQL.V1.EXEC.NOW",
                "sc",
                key,
                line,
            );
        }
    }
}

fn check_args(
    args: Vec<&str>,
    lenght: usize,
) -> Result<Vec<&str>, CString> {
    if args.len() == lenght {
        Ok(args)
    } else {
        let str_error = format!("Wrong number of arguments, it accepts {}, you provide {}",
                                lenght,
                                args.len());
        let error = CString::new(str_error).unwrap();
        Err(error)
    }
}

#[allow(non_snake_case)]
pub extern "C" fn ExecNow(
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

    match check_args(argvector, 3) {
        Err(e) => unsafe {
            r::rm::ffi::RedisModule_ReplyWithError.unwrap()(
                context.as_ptr(),
                e.as_ptr(),
            )
        },
        Ok(args) => {
            match get_dbkey_from_name(context.as_ptr(), args[1]) {
                Err(key_type) => reply_with_error_from_key_type(
                    context.as_ptr(),
                    key_type,
                ),
                Ok(dbkey) => {
                    let dbkey = ManuallyDrop::new(dbkey);
                    let db = dbkey.loop_data.get_db();
                    let result =
                        do_execute(&db, args[2], &Vec::new());
                    let t = std::time::Instant::now()
                        + std::time::Duration::from_secs(10);
                    let mut result = match result {
                        Ok(r) => {
                            ReplicateVerbatim(&context);
                            r.create_data_to_return(
                                &context,
                                &ReturnMethod::Reply,
                                t,
                            )
                        }
                        Err(r) => r.create_data_to_return(
                            &context,
                            &ReturnMethod::Reply,
                            t,
                        ),
                    };

                    result.reply(&context)
                }
            }
        }
    }
}

#[allow(non_snake_case)]
pub extern "C" fn QueryNow(
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

    match check_args(argvector, 3) {
        Err(e) => unsafe {
            r::rm::ffi::RedisModule_ReplyWithError.unwrap()(
                context.as_ptr(),
                e.as_ptr(),
            )
        },
        Ok(args) => {
            match get_dbkey_from_name(context.as_ptr(), args[1]) {
                Err(key_type) => reply_with_error_from_key_type(
                    context.as_ptr(),
                    key_type,
                ),
                Ok(dbkey) => {
                    let dbkey = ManuallyDrop::new(dbkey);
                    let db = dbkey.loop_data.get_db();
                    let result = do_query(&db, args[2], &Vec::new());
                    let t = std::time::Instant::now()
                        + std::time::Duration::from_secs(10);
                    let mut result = match result {
                        Ok(r) => r.create_data_to_return(
                            &context,
                            &ReturnMethod::Reply,
                            t,
                        ),
                        Err(r) => r.create_data_to_return(
                            &context,
                            &ReturnMethod::Reply,
                            t,
                        ),
                    };
                    result.reply(&context)
                }
            }
        }
    }
}

#[allow(non_snake_case)]
pub extern "C" fn QueryNowInto(
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

    match check_args(argvector, 4) {
        Err(e) => unsafe {
            r::rm::ffi::RedisModule_ReplyWithError.unwrap()(
                context.as_ptr(),
                e.as_ptr(),
            )
        },
        Ok(args) => {
            match get_dbkey_from_name(context.as_ptr(), args[2]) {
                Err(key_type) => reply_with_error_from_key_type(
                    context.as_ptr(),
                    key_type,
                ),
                Ok(dbkey) => {
                    let dbkey = ManuallyDrop::new(dbkey);
                    let db = dbkey.loop_data.get_db();
                    let result = do_query(&db, args[3], &Vec::new());
                    let return_method =
                        ReturnMethod::Stream { name: args[1] };
                    let t = std::time::Instant::now()
                        + std::time::Duration::from_secs(10);
                    let mut result = match result {
                        Ok(r) => r.create_data_to_return(
                            &context,
                            &return_method,
                            t,
                        ),
                        Err(r) => r.create_data_to_return(
                            &context,
                            &return_method,
                            t,
                        ),
                    };
                    result.reply(&context)
                }
            }
        }
    }
}

#[allow(non_snake_case)]
pub extern "C" fn ExecStatementNow(
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

    match argvector.len() {
        0..=2 => {
            let str_error = format!("Wrong number of arguments, it needs at least more than 2, you provide only {}",
                                    argvector.len());
            r::rm::ReplyWithError(&context, &str_error)
        }
        _ => {
            match get_dbkey_from_name(context.as_ptr(), argvector[1])
            {
                Err(key_type) => reply_with_error_from_key_type(
                    context.as_ptr(),
                    key_type,
                ),
                Ok(dbkey) => {
                    let dbkey = ManuallyDrop::new(dbkey);
                    // _rc must be
                    // 1. Define befor the call to exec_statement() and .reply(&context)
                    // 2. Dropped before we forget the `dbkey`
                    let t = std::time::Instant::now()
                        + std::time::Duration::from_secs(10);
                    let result = dbkey
                        .loop_data
                        .get_replication_book()
                        .exec_statement(
                            argvector[2],
                            &argvector[3..],
                        );
                    match result {
                        Ok(res) => {
                            ReplicateVerbatim(&context);
                            let mut r = res.create_data_to_return(
                                &context,
                                &ReturnMethod::Reply,
                                t,
                            );
                            r.reply(&context)
                        }
                        Err(mut err) => err.reply(&context),
                    }
                }
            }
        }
    }
}

#[allow(non_snake_case)]
pub extern "C" fn CreateStatementNow(
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

    match check_args(argvector, 4) {
        Err(e) => unsafe {
            r::rm::ffi::RedisModule_ReplyWithError.unwrap()(
                context.as_ptr(),
                e.as_ptr(),
            )
        },
        Ok(args) => {
            match get_dbkey_from_name(context.as_ptr(), args[1]) {
                Err(key_type) => reply_with_error_from_key_type(
                    context.as_ptr(),
                    key_type,
                ),
                Ok(dbkey) => {
                    let dbkey = ManuallyDrop::new(dbkey);
                    let result = dbkey
                        .loop_data
                        .get_replication_book()
                        .insert_new_statement(
                            args[2], args[3], false,
                        );
                    match result {
                        Ok(mut res) => {
                            ReplicateVerbatim(&context);
                            res.reply(&context)
                        }
                        Err(mut e) => e.reply(&context),
                    }
                }
            }
        }
    }
}

#[allow(non_snake_case)]
pub extern "C" fn UpdateStatementNow(
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

    match check_args(argvector, 4) {
        Err(e) => unsafe {
            r::rm::ffi::RedisModule_ReplyWithError.unwrap()(
                context.as_ptr(),
                e.as_ptr(),
            )
        },

        Ok(args) => {
            match get_dbkey_from_name(context.as_ptr(), args[1]) {
                Err(key_type) => reply_with_error_from_key_type(
                    context.as_ptr(),
                    key_type,
                ),
                Ok(dbkey) => {
                    let dbkey = ManuallyDrop::new(dbkey);
                    let result = dbkey
                        .loop_data
                        .get_replication_book()
                        .update_statement(args[2], args[3], false);
                    match result {
                        Ok(mut res) => {
                            ReplicateVerbatim(&context);
                            res.reply(&context)
                        }
                        Err(mut e) => e.reply(&context),
                    }
                }
            }
        }
    }
}

#[allow(non_snake_case)]
pub extern "C" fn DeleteStatementNow(
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

    match check_args(argvector, 3) {
        Err(e) => unsafe {
            r::rm::ffi::RedisModule_ReplyWithError.unwrap()(
                context.as_ptr(),
                e.as_ptr(),
            )
        },

        Ok(args) => {
            match get_dbkey_from_name(context.as_ptr(), args[1]) {
                Err(key_type) => reply_with_error_from_key_type(
                    context.as_ptr(),
                    key_type,
                ),
                Ok(dbkey) => {
                    let dbkey = ManuallyDrop::new(dbkey);
                    let result = dbkey
                        .loop_data
                        .get_replication_book()
                        .delete_statement(args[2]);
                    match result {
                        Ok(mut res) => {
                            ReplicateVerbatim(&context);
                            res.reply(&context)
                        }
                        Err(mut e) => e.reply(&context),
                    }
                }
            }
        }
    }
}

#[allow(non_snake_case)]
pub extern "C" fn QueryStatementNow(
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

    match argvector.len() {
        0..=2 => {
            let str_error = format!("Wrong number of arguments, it needs at least more than 2, you provide only {}",
                                    argvector.len());
            r::rm::ReplyWithError(&context, &str_error)
        }
        _ => {
            match get_dbkey_from_name(context.as_ptr(), argvector[1])
            {
                Err(key_type) => reply_with_error_from_key_type(
                    context.as_ptr(),
                    key_type,
                ),
                Ok(dbkey) => {
                    let dbkey = ManuallyDrop::new(dbkey);
                    let t = std::time::Instant::now()
                        + std::time::Duration::from_secs(10);
                    let result = dbkey
                        .loop_data
                        .get_replication_book()
                        .query_statement(
                            argvector[2],
                            &argvector[3..],
                        );
                    match result {
                        Ok(res) => {
                            ReplicateVerbatim(&context);
                            let mut r = res.create_data_to_return(
                                &context,
                                &ReturnMethod::Reply,
                                t,
                            );
                            r.reply(&context)
                        }
                        Err(mut err) => err.reply(&context),
                    }
                }
            }
        }
    }
}

#[allow(non_snake_case)]
pub extern "C" fn QueryStatementNowInto(
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

    match check_args(argvector, 4) {
        Err(e) => unsafe {
            r::rm::ffi::RedisModule_ReplyWithError.unwrap()(
                context.as_ptr(),
                e.as_ptr(),
            )
        },
        Ok(args) => {
            match get_dbkey_from_name(context.as_ptr(), args[2]) {
                Err(key_type) => reply_with_error_from_key_type(
                    context.as_ptr(),
                    key_type,
                ),
                Ok(dbkey) => {
                    let dbkey = ManuallyDrop::new(dbkey);
                    let result = dbkey
                        .loop_data
                        .get_replication_book()
                        .query_statement(args[3], &args[4..]);
                    let t = std::time::Instant::now()
                        + std::time::Duration::from_secs(10);
                    match result {
                        Ok(result) => {
                            let mut to_return = result
                                .create_data_to_return(
                                    &context,
                                    &ReturnMethod::Stream {
                                        name: args[1],
                                    },
                                    t,
                                );
                            to_return.reply(&context)
                        }
                        Err(mut err) => err.reply(&context),
                    }
                }
            }
        }
    }
}

#[allow(non_snake_case)]
pub extern "C" fn MakeCopyNow(
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

    match check_args(argvector, 3) {
        Err(e) => unsafe {
            r::rm::ffi::RedisModule_ReplyWithError.unwrap()(
                context.as_ptr(),
                e.as_ptr(),
            )
        },
        Ok(argvector) => {
            let source_db =
                get_dbkey_from_name(context.as_ptr(), argvector[1]);
            if source_db.is_err() {
                let error = CString::new(
                    "Error in opening the SOURCE database",
                )
                .unwrap();
                return unsafe {
                    r::rm::ffi::RedisModule_ReplyWithError.unwrap()(
                        context.as_ptr(),
                        error.as_ptr(),
                    )
                };
            }
            let source_db = source_db.unwrap();
            let source_db = ManuallyDrop::new(source_db);

            let dest_db =
                get_dbkey_from_name(context.as_ptr(), argvector[2]);
            if dest_db.is_err() {
                let error = CString::new(
                    "Error in opening the DESTINATION database",
                )
                .unwrap();
                return unsafe {
                    r::rm::ffi::RedisModule_ReplyWithError.unwrap()(
                        context.as_ptr(),
                        error.as_ptr(),
                    )
                };
            }

            let dest_db = dest_db.unwrap();
            let dest_db = ManuallyDrop::new(dest_db);

            let dest_loopdata = &dest_db.loop_data;
            let source_loopdata = &source_db.loop_data;
            let t = std::time::Instant::now()
                + std::time::Duration::from_secs(10);
            let mut result = match r::do_copy(
                &source_loopdata.get_db(),
                dest_loopdata,
            ) {
                Ok(r) => {
                    ReplicateVerbatim(&context);
                    r.create_data_to_return(
                        &context,
                        &ReturnMethod::Reply,
                        t,
                    )
                }
                Err(r) => r.create_data_to_return(
                    &context,
                    &ReturnMethod::Reply,
                    t,
                ),
            };

            result.reply(&context)
        }
    }
}

#[allow(non_snake_case, clippy::missing_safety_doc)]
pub unsafe fn Replicate(
    ctx: &Context,
    command: &str,
    argv: *mut *mut r::rm::ffi::RedisModuleString,
    argc: std::os::raw::c_int,
) -> i32 {
    let command = CString::new(command).unwrap();
    let v = CString::new("v").unwrap();
    r::rm::ffi::RedisModule_Replicate.unwrap()(
        ctx.as_ptr(),
        command.as_ptr(),
        v.as_ptr(),
        argv.offset(1),
        argc - 1,
    )
}

pub fn register(ctx: Context) -> Result<(), i32> {
    #[cfg(feature = "trial")]
    std::thread::spawn(|| {
        println!("# Attention ====================================================================== #");
        println!("# Attention, TRIAL version, do NOT use in production, it will shutdown in ~2 hours #");
        println!("# Attention, TRIAL version, do NOT use in production, it will shutdown in ~2 hours #");
        println!("# Attention, TRIAL version, do NOT use in production, it will shutdown in ~2 hours #");
        println!("# Attention ====================================================================== #");
        let alive_time = 60 * 60 * 2; // 2 hours in seconds
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let random_increment = 60 * (now % 60); // between 0 and 1 hour in second
        let total_wait = alive_time + random_increment;
        let duration = std::time::Duration::from_secs(total_wait);
        std::thread::sleep(duration);
        std::process::exit(1);
    });

    register_write_function(&ctx, "REDISQL.V1.EXEC.NOW", ExecNow)
        .and_then(|_| {
            register_function(
                &ctx,
                "REDISQL.V1.QUERY.NOW",
                "readonly",
                QueryNow,
            )
        })
        .and_then(|_| {
            register_write_function(
                &ctx,
                "REDISQL.V1.CREATE_STATEMENT.NOW",
                CreateStatementNow,
            )
        })
        .and_then(|_| {
            register_write_function(
                &ctx,
                "REDISQL.V1.EXEC_STATEMENT.NOW",
                ExecStatementNow,
            )
        })
        .and_then(|_| {
            register_write_function(
                &ctx,
                "REDISQL.V1.UPDATE_STATEMENT.NOW",
                UpdateStatementNow,
            )
        })
        .and_then(|_| {
            register_write_function(
                &ctx,
                "REDISQL.V1.DELETE_STATEMENT.NOW",
                DeleteStatementNow,
            )
        })
        .and_then(|_| {
            register_function(
                &ctx,
                "REDISQL.V1.QUERY_STATEMENT.NOW",
                "readonly",
                QueryStatementNow,
            )
        })
        .and_then(|_| {
            register_function_with_keys(
                &ctx,
                "REDISQL.V1.QUERY.INTO.NOW",
                "readonly",
                1,
                2,
                1,
                QueryNowInto,
            )
        })
        .and_then(|_| {
            register_function_with_keys(
                &ctx,
                "REDISQL.V1.QUERY_STATEMENT.INTO.NOW",
                "readonly",
                1,
                2,
                1,
                QueryStatementNowInto,
            )
        })
        .and_then(|_| {
            register_write_function(
                &ctx,
                "REDISQL.V1.COPY.NOW",
                MakeCopyNow,
            )
        })
}
