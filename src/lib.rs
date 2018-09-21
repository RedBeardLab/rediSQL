extern crate env_logger;
#[macro_use]
extern crate log;
extern crate redisql_lib;
extern crate uuid;

use env_logger::{LogBuilder, LogTarget};
use redisql_lib::redis as r;
use redisql_lib::redis::{
    get_dbkey_from_name, register_function, register_write_function,
    reply_with_error_from_key_type, with_ch_and_loopdata, LoopData,
    RedisReply,
};
use redisql_lib::redis_type::{Context, ReplicateVerbatim};
use redisql_lib::sqlite as sql;
use redisql_lib::virtual_tables as vtab;
use std::ffi::{CStr, CString};
use std::fs::{remove_file, File};
use std::ptr;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::thread;
use uuid::Uuid;

#[cfg(feature = "pro")]
extern crate engine_pro;
#[cfg(feature = "pro")]
use engine_pro::{register, WriteAOF};

#[cfg(feature = "pro")]
use engine_pro::Replicate;
#[cfg(not(feature = "pro"))]
use redisql_lib::redis::Replicate;

extern "C" fn reply_exec(
    ctx: *mut r::rm::ffi::RedisModuleCtx,
    _argv: *mut *mut r::rm::ffi::RedisModuleString,
    _argc: ::std::os::raw::c_int,
) -> i32 {
    let context = r::rm::Context::new(ctx);
    let result = unsafe {
        r::rm::ffi::RedisModule_GetBlockedClientPrivateData.unwrap()(
            context.as_ptr(),
        ) as *mut Result<r::QueryResult, sql::SQLite3Error>
    };
    let result: Box<Result<r::QueryResult, sql::SQLite3Error>> =
        unsafe { Box::from_raw(result) };
    match *result {
        Ok(query_result) => query_result.reply(&context),
        Err(error) => error.reply(&context),
    }
}

extern "C" fn reply_exec_statement(
    ctx: *mut r::rm::ffi::RedisModuleCtx,
    _argv: *mut *mut r::rm::ffi::RedisModuleString,
    _argc: ::std::os::raw::c_int,
) -> i32 {
    let context = r::rm::Context::new(ctx);
    let result = unsafe {
        r::rm::ffi::RedisModule_GetBlockedClientPrivateData.unwrap()(
            context.as_ptr(),
        ) as *mut Result<r::QueryResult, sql::SQLite3Error>
    };
    let result: Box<Result<r::QueryResult, sql::SQLite3Error>> =
        unsafe { Box::from_raw(result) };
    match *result {
        Ok(query_result) => query_result.reply(&context),
        Err(error) => error.reply(&context),
    }
}

extern "C" fn reply_create_statement(
    ctx: *mut r::rm::ffi::RedisModuleCtx,
    _argv: *mut *mut r::rm::ffi::RedisModuleString,
    _argc: ::std::os::raw::c_int,
) -> i32 {
    let context = r::rm::Context::new(ctx);
    let result = unsafe {
        r::rm::ffi::RedisModule_GetBlockedClientPrivateData.unwrap()(
            context.as_ptr(),
        ) as *mut Result<r::QueryResult, sql::SQLite3Error>
    };
    let result: Box<Result<r::QueryResult, sql::SQLite3Error>> =
        unsafe { Box::from_raw(result) };
    match *result {
        Ok(query_result) => query_result.reply(&context),
        Err(error) => error.reply(&context),
    }
}

extern "C" fn timeout(
    ctx: *mut r::rm::ffi::RedisModuleCtx,
    _argv: *mut *mut r::rm::ffi::RedisModuleString,
    _argc: ::std::os::raw::c_int,
) -> i32 {
    unsafe { r::rm::ffi::RedisModule_ReplyWithNull.unwrap()(ctx) }
}

extern "C" fn free_privdata(_arg: *mut ::std::os::raw::c_void) {}

#[allow(non_snake_case)]
extern "C" fn ExecStatement(
    ctx: *mut r::rm::ffi::RedisModuleCtx,
    argv: *mut *mut r::rm::ffi::RedisModuleString,
    argc: ::std::os::raw::c_int,
) -> i32 {
    let (context, argvector) = r::create_argument(ctx, argv, argc);

    match argvector.len() {
        0...2 => {
            let error = CString::new(
                "Wrong number of arguments, it \
                 needs at least 3",
            ).unwrap();
            unsafe {
                r::rm::ffi::RedisModule_ReplyWithError.unwrap()(
                    context.as_ptr(),
                    error.as_ptr(),
                )
            }
        }
        _ => with_ch_and_loopdata(
            context.as_ptr(),
            argvector[1],
            |ch_loopdata| match ch_loopdata {
                Err(key_type) => reply_with_error_from_key_type(
                    context.as_ptr(),
                    key_type,
                ),
                Ok((ch, _loopdata)) => {
                    let blocked_client = r::rm::BlockedClient {
                        client: unsafe {
                            r::rm::ffi::RedisModule_BlockClient
                                .unwrap()(
                                context.as_ptr(),
                                Some(reply_exec_statement),
                                Some(timeout),
                                Some(free_privdata),
                                10000,
                            )
                        },
                    };

                    let cmd = r::Command::ExecStatement {
                        identifier: argvector[2],
                        arguments: argvector[3..].to_vec(),
                        client: blocked_client,
                    };
                    match ch.send(cmd) {
                        Ok(()) => {
                            unsafe {
                                Replicate(
                                    &context,
                                    "REDISQL.EXEC_STATEMENT.NOW",
                                    argv,
                                    argc,
                                );
                            }
                            r::rm::ffi::REDISMODULE_OK
                        }
                        Err(_) => r::rm::ffi::REDISMODULE_OK,
                    }
                }
            },
        ),
    }
}

#[allow(non_snake_case)]
extern "C" fn QueryStatement(
    ctx: *mut r::rm::ffi::RedisModuleCtx,
    argv: *mut *mut r::rm::ffi::RedisModuleString,
    argc: ::std::os::raw::c_int,
) -> i32 {
    let (context, argvector) = r::create_argument(ctx, argv, argc);

    match argvector.len() {
        0...2 => {
            let error = CString::new(
                "Wrong number of arguments, it \
                 needs at least 3",
            ).unwrap();
            unsafe {
                r::rm::ffi::RedisModule_ReplyWithError.unwrap()(
                    context.as_ptr(),
                    error.as_ptr(),
                )
            }
        }
        _ => with_ch_and_loopdata(
            context.as_ptr(),
            argvector[1],
            |ch_loopdata| match ch_loopdata {
                Err(key_type) => reply_with_error_from_key_type(
                    context.as_ptr(),
                    key_type,
                ),
                Ok((ch, _loopdata)) => {
                    let blocked_client = r::rm::BlockedClient {
                        client: unsafe {
                            r::rm::ffi::RedisModule_BlockClient
                                .unwrap()(
                                context.as_ptr(),
                                Some(reply_exec_statement),
                                Some(timeout),
                                Some(free_privdata),
                                10000,
                            )
                        },
                    };

                    let cmd = r::Command::ExecStatement {
                        identifier: argvector[2],
                        arguments: argvector[3..].to_vec(),
                        client: blocked_client,
                    };

                    match ch.send(cmd) {
                        Ok(()) => r::rm::ffi::REDISMODULE_OK,
                        Err(_) => r::rm::ffi::REDISMODULE_OK,
                    }
                }
            },
        ),
    }
}

#[allow(non_snake_case)]
extern "C" fn Exec(
    ctx: *mut r::rm::ffi::RedisModuleCtx,
    argv: *mut *mut r::rm::ffi::RedisModuleString,
    argc: ::std::os::raw::c_int,
) -> i32 {
    let (context, argvector) = r::create_argument(ctx, argv, argc);
    match argvector.len() {
        3 => with_ch_and_loopdata(
            context.as_ptr(),
            argvector[1],
            |leaky_db| match leaky_db {
                Err(key_type) => reply_with_error_from_key_type(
                    context.as_ptr(),
                    key_type,
                ),
                Ok((ch, _loopdata)) => {
                    debug!("Exec | GotDB");
                    let blocked_client = r::rm::BlockedClient {
                        client: unsafe {
                            r::rm::ffi::RedisModule_BlockClient
                                .unwrap()(
                                context.as_ptr(),
                                Some(reply_exec),
                                Some(timeout),
                                Some(free_privdata),
                                10000,
                            )
                        },
                    };
                    debug!("Exec | BlockedClient");

                    let cmd = r::Command::Exec {
                        query: argvector[2],
                        client: blocked_client,
                    };
                    debug!("Exec | Create Command");
                    match ch.send(cmd) {
                        Ok(()) => {
                            unsafe {
                                Replicate(
                                    &context,
                                    "REDISQL.EXEC.NOW",
                                    argv,
                                    argc,
                                );
                            }
                            r::rm::ffi::REDISMODULE_OK
                        }
                        Err(_) => r::rm::ffi::REDISMODULE_OK,
                    }
                }
            },
        ),
        n => {
            let error = CString::new(format!(
                "Wrong number of arguments, it \
                 accepts 3, you provide {}",
                n
            )).unwrap();
            unsafe {
                r::rm::ffi::RedisModule_ReplyWithError.unwrap()(
                    context.as_ptr(),
                    error.as_ptr(),
                )
            }
        }
    }
}

#[allow(non_snake_case)]
extern "C" fn Query(
    ctx: *mut r::rm::ffi::RedisModuleCtx,
    argv: *mut *mut r::rm::ffi::RedisModuleString,
    argc: ::std::os::raw::c_int,
) -> i32 {
    let (context, argvector) = r::create_argument(ctx, argv, argc);
    match argvector.len() {
        3 => with_ch_and_loopdata(
            context.as_ptr(),
            argvector[1],
            |ch_loopdata| match ch_loopdata {
                Err(key_type) => reply_with_error_from_key_type(
                    context.as_ptr(),
                    key_type,
                ),
                Ok((ch, _loopdata)) => {
                    let blocked_client = r::rm::BlockedClient {
                        client: unsafe {
                            r::rm::ffi::RedisModule_BlockClient
                                .unwrap()(
                                context.as_ptr(),
                                Some(reply_exec),
                                Some(timeout),
                                Some(free_privdata),
                                10000,
                            )
                        },
                    };

                    let cmd = r::Command::Query {
                        query: argvector[2],
                        client: blocked_client,
                    };
                    match ch.send(cmd) {
                        Ok(()) => r::rm::ffi::REDISMODULE_OK,
                        Err(_) => r::rm::ffi::REDISMODULE_OK,
                    }
                }
            },
        ),
        n => {
            let error = CString::new(format!(
                "Wrong number of arguments, it \
                 accepts 3, you provide {}",
                n
            )).unwrap();
            unsafe {
                r::rm::ffi::RedisModule_ReplyWithError.unwrap()(
                    context.as_ptr(),
                    error.as_ptr(),
                )
            }
        }
    }
}

#[allow(non_snake_case)]
extern "C" fn CreateStatement(
    ctx: *mut r::rm::ffi::RedisModuleCtx,
    argv: *mut *mut r::rm::ffi::RedisModuleString,
    argc: ::std::os::raw::c_int,
) -> i32 {
    let (context, argvector) = r::create_argument(ctx, argv, argc);
    match argvector.len() {
        4 => {
            with_ch_and_loopdata(
                context.as_ptr(),
                argvector[1],
                |ch_loopdata| match ch_loopdata {
                    Err(key_type) => reply_with_error_from_key_type(
                        context.as_ptr(),
                        key_type,
                    ),
                    Ok((ch, _loopdata)) => {
                        let blocked_client = r::rm::BlockedClient {
                            client: unsafe {
                                r::rm::ffi::RedisModule_BlockClient
                                    .unwrap()(
                                    context.as_ptr(),
                                    Some(reply_create_statement),
                                    Some(timeout),
                                    Some(free_privdata),
                                    10000,
                                )
                            },
                        };
                        let cmd = r::Command::CompileStatement {
                            identifier: argvector[2],
                            statement: argvector[3],
                            client: blocked_client,
                        };
                        match ch.send(cmd) {
                            Ok(()) => {
                                unsafe {
                                    Replicate(&context, "REDISQL.CREATE_STATEMENT.NOW", argv, argc);
                                }
                                r::rm::ffi::REDISMODULE_OK
                            }
                            Err(_) => r::rm::ffi::REDISMODULE_OK,
                        }
                    }
                },
            )
        }

        _ => {
            let error = CString::new(
                "Wrong number of arguments, it \
                 accepts 4",
            ).unwrap();
            unsafe {
                r::rm::ffi::RedisModule_ReplyWithError.unwrap()(
                    context.as_ptr(),
                    error.as_ptr(),
                )
            }
        }
    }
}

#[allow(non_snake_case)]
extern "C" fn UpdateStatement(
    ctx: *mut r::rm::ffi::RedisModuleCtx,
    argv: *mut *mut r::rm::ffi::RedisModuleString,
    argc: ::std::os::raw::c_int,
) -> i32 {
    let (context, argvector) = r::create_argument(ctx, argv, argc);
    match argvector.len() {
        4 => {
            with_ch_and_loopdata(
                context.as_ptr(),
                argvector[1],
                |ch_loopdata| match ch_loopdata {
                    Err(key_type) => reply_with_error_from_key_type(
                        context.as_ptr(),
                        key_type,
                    ),
                    Ok((ch, _loopdata)) => {
                        let blocked_client = r::rm::BlockedClient {
                            client: unsafe {
                                r::rm::ffi::RedisModule_BlockClient
                                    .unwrap()(
                                    context.as_ptr(),
                                    Some(reply_create_statement),
                                    Some(timeout),
                                    Some(free_privdata),
                                    10000,
                                )
                            },
                        };

                        let cmd = r::Command::UpdateStatement {
                            identifier: argvector[2],
                            statement: argvector[3],
                            client: blocked_client,
                        };

                        match ch.send(cmd) {
                            Ok(()) => {
                                unsafe {
                                    Replicate(&context, "REDISQL.UPDATE_STATEMENT.NOW", argv, argc);
                                }
                                r::rm::ffi::REDISMODULE_OK
                            }
                            Err(_) => r::rm::ffi::REDISMODULE_OK,
                        }
                    }
                },
            )
        }

        _ => {
            let error = CString::new(
                "Wrong number of arguments, it \
                 accepts 4",
            ).unwrap();
            unsafe {
                r::rm::ffi::RedisModule_ReplyWithError.unwrap()(
                    context.as_ptr(),
                    error.as_ptr(),
                )
            }
        }
    }
}

#[allow(non_snake_case)]
extern "C" fn DeleteStatement(
    ctx: *mut r::rm::ffi::RedisModuleCtx,
    argv: *mut *mut r::rm::ffi::RedisModuleString,
    argc: ::std::os::raw::c_int,
) -> i32 {
    let (context, argvector) = r::create_argument(ctx, argv, argc);
    match argvector.len() {
        3 => with_ch_and_loopdata(
            context.as_ptr(),
            argvector[1],
            |ch_loopdata| {
                match ch_loopdata {
                    Err(key_type) => reply_with_error_from_key_type(
                        context.as_ptr(),
                        key_type,
                    ),
                    Ok((ch, _loopdata)) => {
                        //let ch = &db.tx;
                        //let _loopdata = &db.loop_data;
                        let blocked_client = r::rm::BlockedClient {
                            client: unsafe {
                                r::rm::ffi::RedisModule_BlockClient
                                    .unwrap()(
                                    context.as_ptr(),
                                    Some(reply_create_statement),
                                    Some(timeout),
                                    Some(free_privdata),
                                    10000,
                                )
                            },
                        };

                        let cmd = r::Command::DeleteStatement {
                            identifier: argvector[2],
                            client: blocked_client,
                        };
                        match ch.send(cmd) {
                            Ok(()) => {
                                unsafe {
                                    Replicate(&context, "REDISQL.DELETE_STATEMENT.NOW", argv, argc);
                                }
                                r::rm::ffi::REDISMODULE_OK
                            }
                            Err(_) => r::rm::ffi::REDISMODULE_OK,
                        }
                    }
                }
            },
        ),
        _ => {
            let error = CString::new(
                "Wrong number of arguments, it \
                 accepts 3",
            ).unwrap();
            unsafe {
                r::rm::ffi::RedisModule_ReplyWithError.unwrap()(
                    context.as_ptr(),
                    error.as_ptr(),
                )
            }
        }
    }
}

#[allow(non_snake_case)]
extern "C" fn CreateDB(
    ctx: *mut r::rm::ffi::RedisModuleCtx,
    argv: *mut *mut r::rm::ffi::RedisModuleString,
    argc: ::std::os::raw::c_int,
) -> i32 {
    let (context, argvector) = r::create_argument(ctx, argv, argc);

    match argvector.len() {
        2 | 3 => {
            let key_name =
                r::rm::RMString::new(&context, argvector[1]);
            let key = unsafe {
                r::rm::ffi::Export_RedisModule_OpenKey(
                    context.as_ptr(),
                    key_name.as_ptr(),
                    r::rm::ffi::REDISMODULE_WRITE,
                )
            };
            let safe_key = r::RedisKey { key };
            match unsafe {
                r::rm::ffi::RedisModule_KeyType.unwrap()(safe_key.key)
            } {
                r::rm::ffi::REDISMODULE_KEYTYPE_EMPTY => {
                    let (path, in_memory): (
                        &str,
                        bool,
                    ) = match argvector.len() {
                        3 => (argvector[2], false),
                        _ => (":memory:", true),
                    };
                    match sql::get_arc_connection(path) {
                        Ok(rc) => {
                            match r::create_metadata_table(rc.clone())
                                .and_then(|_| {
                                    r::enable_foreign_key(rc.clone())
                                }).and_then(|_| {
                                    vtab::register_modules(&rc)
                                }) {
                                Err(e) => e.reply(&context),
                                Ok(mut vtab_context) => {
                                    let (tx, rx) = channel();
                                    let db = r::DBKey::new_from_arc(
                                        tx,
                                        rc,
                                        in_memory,
                                        vtab_context,
                                    );
                                    let mut loop_data =
                                        db.loop_data.clone();
                                    thread::spawn(move || {
                                        r::listen_and_execute(
                                            &mut loop_data,
                                            &rx,
                                        );
                                    });
                                    let ptr =
                                        Box::into_raw(Box::new(db));
                                    let type_set = unsafe {
                                        r::rm::ffi::RedisModule_ModuleTypeSetValue.unwrap()(
                                        safe_key.key,
                                        r::rm::ffi::DBType,
                                        ptr as *mut std::os::raw::c_void,
                                    )
                                    };

                                    match type_set {
                                    r::rm::ffi::REDISMODULE_OK => {
                                        let ok =
                                            r::QueryResult::OK {};
                                        ReplicateVerbatim(&context);
                                        ok.reply(&context)
                                    }
                                    r::rm::ffi::REDISMODULE_ERR => {
                                        let err = CString::new(
                                            "ERR - Error in saving the database inside Redis",
                                        ).unwrap();
                                        unsafe {
                                            r::rm::ffi::RedisModule_ReplyWithSimpleString.unwrap()(
                                                context.as_ptr(),
                                                err.as_ptr(),
                                            )
                                        }
                                    }
                                    _ => {
                                        let err = CString::new("ERR - Error unknow").unwrap();
                                        unsafe {
                                            r::rm::ffi::RedisModule_ReplyWithSimpleString.unwrap()(
                                                context.as_ptr(),
                                                err.as_ptr(),
                                            )
                                        }
                                    }
                                }
                                }
                            }
                        }
                        Err(_) => {
                            let error = CString::new(
                                "Err - Error \
                                 opening the in \
                                 memory databade",
                            ).unwrap();
                            unsafe {
                                r::rm::ffi::RedisModule_ReplyWithError
                                    .unwrap()(
                                    context.as_ptr(),
                                    error.as_ptr(),
                                )
                            }
                        }
                    }
                }
                _ => {
                    let error = CStr::from_bytes_with_nul(
                        r::rm::ffi::REDISMODULE_ERRORMSG_WRONGTYPE,
                    ).unwrap();
                    unsafe {
                        r::rm::ffi::RedisModule_ReplyWithError
                            .unwrap()(
                            context.as_ptr(),
                            error.as_ptr(),
                        )
                    }
                }
            }
        }
        _ => {
            let error = CString::new(
                "Wrong number of arguments, it \
                 accepts 2 or 3",
            ).unwrap();
            unsafe {
                r::rm::ffi::RedisModule_ReplyWithError.unwrap()(
                    context.as_ptr(),
                    error.as_ptr(),
                )
            }
        }
    }
}

unsafe extern "C" fn free_db(db_ptr: *mut ::std::os::raw::c_void) {
    let db: Box<r::DBKey> = Box::from_raw(db_ptr as *mut r::DBKey);
    let tx = &db.tx;

    match tx.send(r::Command::Stop) {
        _ => (),
    }
}

unsafe extern "C" fn rdb_save(
    rdb: *mut r::rm::ffi::RedisModuleIO,
    value: *mut std::os::raw::c_void,
) {
    let db: *mut r::DBKey =
        Box::into_raw(Box::from_raw(value as *mut r::DBKey));

    if (*db).in_memory {
        let path = format!("rediSQL_rdb_{}.sqlite", Uuid::new_v4());

        let db = (*db).loop_data.get_db();
        let conn = &db.lock().unwrap();
        match r::create_backup(conn, &path) {
            Err(e) => println!("{}", e),
            Ok(not_done)
                if !sql::backup_complete_with_done(not_done) =>
            {
                println!("Return NOT DONE: {}", not_done)
            }
            Ok(_) => match File::open(path.clone()) {
                Err(e) => println!("{}", e),
                Ok(f) => match r::write_file_to_rdb(f, rdb) {
                    Ok(()) => match remove_file(path) {
                        _ => (),
                    },
                    Err(_) => {
                        println!(
                            "Impossible to write the file \
                             in the rdb file"
                        );
                    }
                },
            },
        }
    }
}

unsafe extern "C" fn rdb_load(
    rdb: *mut r::rm::ffi::RedisModuleIO,
    _encoding_version: i32,
) -> *mut std::os::raw::c_void {
    let path = format!("rediSQL_rdb_{}.sqlite", Uuid::new_v4());

    let mut file = match File::create(path.clone()) {
        Err(_) => {
            println!("Was impossible to create a file!");
            return ptr::null_mut();
        }
        Ok(f) => f,
    };

    if r::write_rdb_to_file(&mut file, rdb).is_err() {
        println!("Was impossible to write the rdb file!");
        return ptr::null_mut();
    }

    let in_mem = match sql::RawConnection::open_connection(":memory:")
    {
        Err(_) => {
            println!("Was impossible to open the in memory db!");
            return ptr::null_mut();
        }
        Ok(in_mem) => in_mem,
    };

    let on_disk = match sql::RawConnection::open_connection(&path) {
        Err(_) => {
            println!("Error in opening the rdb database");
            return ptr::null_mut();
        }
        Ok(on_disk) => on_disk,
    };

    match r::make_backup(&on_disk, &in_mem) {
        Err(e) => {
            println!("{}", e);
            ptr::null_mut()
        }
        Ok(_) => {
            let (tx, rx) = channel();
            let conn = Arc::new(Mutex::new(in_mem));
            let redis_context = match vtab::register_modules(&conn) {
                Err(e) => {
                    println!("{}", e);
                    return ptr::null_mut();
                }
                Ok(redis_context) => redis_context,
            };
            let db =
                r::DBKey::new_from_arc(tx, conn, true, redis_context);
            let mut loop_data = db.loop_data.clone();

            thread::spawn(move || {
                r::listen_and_execute(&mut loop_data, &rx)
            });

            Box::into_raw(Box::new(db)) as *mut std::os::raw::c_void
        }
    }
}

#[allow(non_snake_case)]
extern "C" fn MakeCopy(
    ctx: *mut r::rm::ffi::RedisModuleCtx,
    argv: *mut *mut r::rm::ffi::RedisModuleString,
    argc: ::std::os::raw::c_int,
) -> i32 {
    debug!("MakeCopy | Start");
    let (context, argvector) = r::create_argument(ctx, argv, argc);

    if argvector.len() != 3 {
        let error = CString::new(
            "Wrong number of arguments, it accepts exactly 3",
        ).unwrap();
        return unsafe {
            r::rm::ffi::RedisModule_ReplyWithError.unwrap()(
                context.as_ptr(),
                error.as_ptr(),
            )
        };
    }

    let source_db =
        get_dbkey_from_name(context.as_ptr(), argvector[1]);
    if source_db.is_err() {
        let error =
            CString::new("Error in opening the SOURCE database")
                .unwrap();
        return unsafe {
            r::rm::ffi::RedisModule_ReplyWithError.unwrap()(
                context.as_ptr(),
                error.as_ptr(),
            )
        };
    }
    let source_db = source_db.unwrap();

    let dest_db = get_dbkey_from_name(context.as_ptr(), argvector[2]);
    if dest_db.is_err() {
        let error =
            CString::new("Error in opening the DESTINATION database")
                .unwrap();
        return unsafe {
            r::rm::ffi::RedisModule_ReplyWithError.unwrap()(
                context.as_ptr(),
                error.as_ptr(),
            )
        };
    }
    let dest_db = dest_db.unwrap();

    let blocked_client = r::rm::BlockedClient {
        client: unsafe {
            r::rm::ffi::RedisModule_BlockClient.unwrap()(
                context.as_ptr(),
                Some(reply_create_statement),
                Some(timeout),
                Some(free_privdata),
                10000,
            )
        },
    };

    /*
    let source_connection = source_db.loop_data.get_db();
    let dest_connection = dest_db.loop_data.get_db();
    */
    let ch = &source_db.tx.clone();

    let cmd = r::Command::MakeCopy {
        source: source_db,
        destination: dest_db,
        client: blocked_client,
    };

    /*
    std::mem::forget(source_db);
    std::mem::forget(dest_db);
    */

    debug!("MakeCopy | End");
    match ch.send(cmd) {
        Ok(()) => {
            debug!("MakeCopy | Successfully send command");
            r::rm::ffi::REDISMODULE_OK
        }
        Err(_) => r::rm::ffi::REDISMODULE_OK,
    }
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn RedisModule_OnLoad(
    ctx: *mut r::rm::ffi::RedisModuleCtx,
    _argv: *mut *mut r::rm::ffi::RedisModuleString,
    _argc: i32,
) -> i32 {
    let ctx = Context::new(ctx);

    sql::disable_global_memory_statistics();

    LogBuilder::new()
        .filter(None, log::LogLevelFilter::Debug)
        .target(LogTarget::Stdout)
        .init()
        .unwrap();

    let c_data_type_name = CString::new("rediSQLDB").unwrap();
    let ptr_data_type_name = c_data_type_name.as_ptr();

    #[cfg(feature = "pro")]
    let mut types = r::rm::ffi::RedisModuleTypeMethods {
        version: 1,
        rdb_load: Some(rdb_load),
        rdb_save: Some(rdb_save),
        aof_rewrite: Some(WriteAOF),
        mem_usage: None,
        digest: None,
        free: Some(free_db),
    };

    #[cfg(not(feature = "pro"))]
    let mut types = r::rm::ffi::RedisModuleTypeMethods {
        version: 1,
        rdb_load: Some(rdb_load),
        rdb_save: Some(rdb_save),
        aof_rewrite: None,
        mem_usage: None,
        digest: None,
        free: Some(free_db),
    };

    let module_c_name = CString::new("rediSQL").unwrap();
    let module_ptr_name = module_c_name.as_ptr();
    if unsafe {
        r::rm::ffi::Export_RedisModule_Init(
            ctx.as_ptr(),
            module_ptr_name,
            1,
            r::rm::ffi::REDISMODULE_APIVER_1,
        )
    } == r::rm::ffi::REDISMODULE_ERR
    {
        return r::rm::ffi::REDISMODULE_ERR;
    }

    unsafe {
        r::rm::ffi::DBType = r::rm::ffi::RedisModule_CreateDataType
            .unwrap()(
            ctx.as_ptr(),
            ptr_data_type_name,
            1,
            &mut types,
        );
    }

    if unsafe { r::rm::ffi::DBType.is_null() } {
        return r::rm::ffi::REDISMODULE_ERR;
    }

    match register_write_function(
        &ctx,
        String::from("REDISQL.CREATE_DB"),
        CreateDB,
    ) {
        Ok(()) => (),
        Err(e) => return e,
    }

    match register_write_function(
        &ctx,
        String::from("REDISQL.EXEC"),
        Exec,
    ) {
        Ok(()) => (),
        Err(e) => return e,
    }

    match register_function(
        &ctx,
        String::from("REDISQL.QUERY"),
        String::from("readonly"),
        Query,
    ) {
        Ok(()) => (),
        Err(e) => return e,
    }

    match register_write_function(
        &ctx,
        String::from("REDISQL.CREATE_STATEMENT"),
        CreateStatement,
    ) {
        Ok(()) => (),
        Err(e) => return e,
    }

    match register_write_function(
        &ctx,
        String::from("REDISQL.EXEC_STATEMENT"),
        ExecStatement,
    ) {
        Ok(()) => (),
        Err(e) => return e,
    }

    match register_write_function(
        &ctx,
        String::from("REDISQL.UPDATE_STATEMENT"),
        UpdateStatement,
    ) {
        Ok(()) => (),
        Err(e) => return e,
    }

    match register_write_function(
        &ctx,
        String::from("REDISQL.DELETE_STATEMENT"),
        DeleteStatement,
    ) {
        Ok(()) => (),
        Err(e) => return e,
    }

    match register_function(
        &ctx,
        String::from("REDISQL.QUERY_STATEMENT"),
        String::from("readonly"),
        QueryStatement,
    ) {
        Ok(()) => (),
        Err(e) => return e,
    }

    match register_function(
        &ctx,
        String::from("REDISQL.COPY"),
        String::from("readonly"),
        MakeCopy,
    ) {
        Ok(()) => (),
        Err(e) => return e,
    }

    #[cfg(feature = "pro")]
    match register(ctx) {
        Ok(()) => (),
        Err(e) => return e,
    }

    r::rm::ffi::REDISMODULE_OK
}
