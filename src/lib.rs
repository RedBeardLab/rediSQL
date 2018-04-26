extern crate uuid;

extern crate log;
extern crate env_logger;

use env_logger::{LogBuilder, LogTarget};

use std::ffi::{CString, CStr};
use std::mem;
use std::ptr;
use std::fs::{remove_file, File};

use std::thread;
use std::sync::mpsc::{channel, Sender};

use uuid::Uuid;

extern crate redisql_lib;

use redisql_lib::redis_type::{Context, ReplicateVerbatim};

use redisql_lib::sqlite as sql;

use redisql_lib::redis as r;
use redisql_lib::redis::{RedisReply, Loop, LoopData,
                         reply_with_error_from_key_type,
                         get_db_channel_from_name,
                         get_dbkey_from_name, register_function,
                         register_write_function};

#[cfg(feature = "pro")]
extern crate engine_pro;
#[cfg(feature = "pro")]
use engine_pro::{WriteAOF, register};

#[cfg(feature = "pro")]
use engine_pro::replicate;
#[cfg(not(feature = "pro"))]
use redisql_lib::redis::replicate;

extern "C" fn reply_exec(ctx: *mut r::rm::ffi::RedisModuleCtx,
                         _argv: *mut *mut r::rm::ffi::RedisModuleString,
                         _argc: ::std::os::raw::c_int)
-> i32{
    let result = unsafe {
        r::rm::ffi::RedisModule_GetBlockedClientPrivateData
            .unwrap()(ctx) as
        *mut Result<r::QueryResult, sql::SQLite3Error>
    };
    let result: Box<Result<r::QueryResult, sql::SQLite3Error>> =
        unsafe { Box::from_raw(result) };
    match *result {
        Ok(query_result) => query_result.reply(ctx),
        Err(error) => error.reply(ctx),
    }
}

extern "C" fn reply_exec_statement(
    ctx: *mut r::rm::ffi::RedisModuleCtx,
    _argv: *mut *mut r::rm::ffi::RedisModuleString,
    _argc: ::std::os::raw::c_int,
) -> i32{
    let result = unsafe {
        r::rm::ffi::RedisModule_GetBlockedClientPrivateData
            .unwrap()(ctx) as
        *mut Result<r::QueryResult, sql::SQLite3Error>
    };
    let result: Box<Result<r::QueryResult, sql::SQLite3Error>> =
        unsafe { Box::from_raw(result) };
    match *result {
        Ok(query_result) => query_result.reply(ctx),
        Err(error) => error.reply(ctx),
    }
}

extern "C" fn reply_create_statement(
    ctx: *mut r::rm::ffi::RedisModuleCtx,
    _argv: *mut *mut r::rm::ffi::RedisModuleString,
    _argc: ::std::os::raw::c_int,
) -> i32{
    let result = unsafe {
        r::rm::ffi::RedisModule_GetBlockedClientPrivateData
            .unwrap()(ctx) as
        *mut Result<r::QueryResult, sql::SQLite3Error>
    };
    let result: Box<Result<r::QueryResult, sql::SQLite3Error>> =
        unsafe { Box::from_raw(result) };
    match *result {
        Ok(query_result) => query_result.reply(ctx),
        Err(error) => error.reply(ctx),
    }
}


extern "C" fn timeout(ctx: *mut r::rm::ffi::RedisModuleCtx,
                      _argv: *mut *mut r::rm::ffi::RedisModuleString,
                      _argc: ::std::os::raw::c_int)
-> i32{
    unsafe { r::rm::ffi::RedisModule_ReplyWithNull.unwrap()(ctx) }
}


extern "C" fn free_privdata(_arg: *mut ::std::os::raw::c_void) {}

fn get_db_and_loopdata_from_name
    (ctx: *mut r::rm::ffi::RedisModuleCtx,
     name: &str)
     -> Result<(Sender<r::Command>, Loop), i32> {
    let db: Box<r::DBKey> = get_dbkey_from_name(ctx, name)?;
    let channel = db.tx.clone();
    let loopdata = db.loop_data.clone();
    std::mem::forget(db);
    Ok((channel, loopdata))
}


#[allow(non_snake_case)]
extern "C" fn ExecStatement(
    ctx: *mut r::rm::ffi::RedisModuleCtx,
    argv: *mut *mut r::rm::ffi::RedisModuleString,
    argc: ::std::os::raw::c_int,
) -> i32{
    let (_context, argvector) = r::create_argument(ctx, argv, argc);

    match argvector.len() {
        0...2 => {
            let error = CString::new("Wrong number of arguments, it \
                                      needs at least 3")
                    .unwrap();
            unsafe {
                r::rm::ffi::RedisModule_ReplyWithError
                    .unwrap()(ctx, error.as_ptr())
            }
        }
        _ => {
            match get_db_and_loopdata_from_name(ctx, &argvector[1]) {
                Err(key_type) => {
                    reply_with_error_from_key_type(ctx, key_type)
                }
                Ok((ch, _)) => {
                    let blocked_client =
                        r::BlockedClient {
                            client: unsafe {
                                r::rm::ffi::RedisModule_BlockClient.unwrap()(ctx,
                                                              Some(reply_exec_statement),
                                                              Some(timeout),
                                                              Some(free_privdata),
                                                              10000)
                            },
                        };

                    let cmd = r::Command::ExecStatement {
                        identifier: argvector[2],
                        arguments: argvector[3..].to_vec(),
                        client: blocked_client,
                    };

                    match ch.send(cmd) {
                        Ok(()) => {
                            replicate(ctx, String::from("REDISQL.EXEC_STATEMENT.NOW"), argv, argc);
                            r::rm::ffi::REDISMODULE_OK
                        }
                        Err(_) => r::rm::ffi::REDISMODULE_OK,
                    }
                }
            }
        }
    }
}

#[allow(non_snake_case)]
extern "C" fn QueryStatement(
    ctx: *mut r::rm::ffi::RedisModuleCtx,
    argv: *mut *mut r::rm::ffi::RedisModuleString,
    argc: ::std::os::raw::c_int,
) -> i32{
    let (_context, argvector) = r::create_argument(ctx, argv, argc);

    match argvector.len() {
        0...2 => {
            let error = CString::new("Wrong number of arguments, it \
                                      needs at least 3")
                    .unwrap();
            unsafe {
                r::rm::ffi::RedisModule_ReplyWithError
                    .unwrap()(ctx, error.as_ptr())
            }
        }
        _ => {
            match get_db_and_loopdata_from_name(ctx, &argvector[1]) {
                Err(key_type) => {
                    reply_with_error_from_key_type(ctx, key_type)
                }
                Ok((ch, _)) => {
                    let blocked_client =
                        r::BlockedClient {
                            client: unsafe {
                                r::rm::ffi::RedisModule_BlockClient.unwrap()(ctx,
                                                              Some(reply_exec_statement),
                                                              Some(timeout),
                                                              Some(free_privdata),
                                                              10000)
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
            }
        }
    }
}

#[allow(non_snake_case)]
extern "C" fn Exec(ctx: *mut r::rm::ffi::RedisModuleCtx,
                   argv: *mut *mut r::rm::ffi::RedisModuleString,
                   argc: ::std::os::raw::c_int)
                   -> i32 {
    let (_context, argvector) = r::create_argument(ctx, argv, argc);
    match argvector.len() {
        3 => {
            match get_db_channel_from_name(ctx, &argvector[1]) {
                Err(key_type) => {
                    reply_with_error_from_key_type(ctx, key_type)
                }
                Ok(ch) => {
                    let blocked_client = r::BlockedClient {
                        client:
                            unsafe {
                                r::rm::ffi::RedisModule_BlockClient
                                    .unwrap()(ctx,
                                              Some(reply_exec),
                                              Some(timeout),
                                              Some(free_privdata),
                                              10000)
                            },
                    };
                    mem::forget(ctx);
                    let cmd = r::Command::Exec {
                        query: argvector[2],
                        client: blocked_client,
                    };
                    match ch.send(cmd) {
                        Ok(()) => {
                            replicate(ctx, String::from("REDISQL.EXEC.NOW"), argv, argc);
                            r::rm::ffi::REDISMODULE_OK
                        }
                        Err(_) => r::rm::ffi::REDISMODULE_OK,
                    }
                }
            }
        }
        n => {
            let error = CString::new(format!("Wrong number of arguments, it \
                                      accepts 3, you provide {}",
                                             n))
                    .unwrap();
            unsafe {
                r::rm::ffi::RedisModule_ReplyWithError
                    .unwrap()(ctx, error.as_ptr())
            }
        }
    }
}


#[allow(non_snake_case)]
extern "C" fn Query(ctx: *mut r::rm::ffi::RedisModuleCtx,
                    argv: *mut *mut r::rm::ffi::RedisModuleString,
                    argc: ::std::os::raw::c_int)
                    -> i32 {
    let (_context, argvector) = r::create_argument(ctx, argv, argc);
    match argvector.len() {
        3 => {
            match get_db_channel_from_name(ctx, &argvector[1]) {
                Err(key_type) => {
                    reply_with_error_from_key_type(ctx, key_type)
                }
                Ok(ch) => {
                    let blocked_client = r::BlockedClient {
                        client:
                            unsafe {
                                r::rm::ffi::RedisModule_BlockClient
                                    .unwrap()(ctx,
                                              Some(reply_exec),
                                              Some(timeout),
                                              Some(free_privdata),
                                              10000)
                            },
                    };
                    mem::forget(ctx);
                    let cmd = r::Command::Query {
                        query: argvector[2],
                        client: blocked_client,
                    };
                    match ch.send(cmd) {
                        Ok(()) => {
                            r::rm::ffi::REDISMODULE_OK
                        }
                        Err(_) => r::rm::ffi::REDISMODULE_OK,
                    }
                }
            }
        }
        n => {
            let error = CString::new(format!("Wrong number of arguments, it \
                                      accepts 3, you provide {}",
                                             n))
                    .unwrap();
            unsafe {
                r::rm::ffi::RedisModule_ReplyWithError
                    .unwrap()(ctx, error.as_ptr())
            }
        }
    }
}



#[allow(non_snake_case)]
extern "C" fn CreateStatement(
    ctx: *mut r::rm::ffi::RedisModuleCtx,
    argv: *mut *mut r::rm::ffi::RedisModuleString,
    argc: ::std::os::raw::c_int,
) -> i32{
    let (_ctx, argvector) = r::create_argument(ctx, argv, argc);
    match argvector.len() {
        4 => {
            match get_db_channel_from_name(ctx, &argvector[1]) {
                Err(key_type) => {
                    reply_with_error_from_key_type(ctx, key_type)
                }
                Ok(ch) => {
                    let blocked_client =
                        r::BlockedClient {
                            client: unsafe {
                                r::rm::ffi::RedisModule_BlockClient.unwrap()(ctx,
                                                              Some(reply_create_statement),
                                                              Some(timeout),
                                                              Some(free_privdata),
                                                              10000)
                            },
                        };
                    let cmd = r::Command::CompileStatement {
                        identifier: argvector[2],
                        statement: argvector[3],
                        client: blocked_client,
                    };

                    match ch.send(cmd) {
                        Ok(()) => {
                            replicate(ctx, String::from("REDISQL.CREATE_STATEMENT.NOW"), argv, argc);
                            r::rm::ffi::REDISMODULE_OK
                        }
                        Err(_) => r::rm::ffi::REDISMODULE_OK,
                    }

                }
            }
        }

        _ => {
            let error = CString::new("Wrong number of arguments, it \
                                      accepts 4")
                    .unwrap();
            unsafe {
                r::rm::ffi::RedisModule_ReplyWithError
                    .unwrap()(ctx, error.as_ptr())
            }
        }
    }
}



#[allow(non_snake_case)]
extern "C" fn UpdateStatement(
    ctx: *mut r::rm::ffi::RedisModuleCtx,
    argv: *mut *mut r::rm::ffi::RedisModuleString,
    argc: ::std::os::raw::c_int,
) -> i32{
    let (_ctx, argvector) = r::create_argument(ctx, argv, argc);
    match argvector.len() {
        4 => {
            match get_db_channel_from_name(ctx, &argvector[1]) {
                Err(key_type) => {
                    reply_with_error_from_key_type(ctx, key_type)
                }
                Ok(ch) => {
                    let blocked_client =
                        r::BlockedClient {
                            client: unsafe {
                                r::rm::ffi::RedisModule_BlockClient.unwrap()(ctx,
                                                              Some(reply_create_statement),
                                                              Some(timeout),
                                                              Some(free_privdata),
                                                              10000)
                            },
                        };

                    let cmd = r::Command::UpdateStatement {
                        identifier: argvector[2],
                        statement: argvector[3],
                        client: blocked_client,
                    };

                    match ch.send(cmd) {
                        Ok(()) => {
                            replicate(ctx, String::from("REDISQL.UPDATE_STATEMENT.NOW"), argv, argc);
                            r::rm::ffi::REDISMODULE_OK
                        }
                        Err(_) => r::rm::ffi::REDISMODULE_OK,
                    }

                }
            }
        }

        _ => {
            let error = CString::new("Wrong number of arguments, it \
                                      accepts 4")
                    .unwrap();
            unsafe {
                r::rm::ffi::RedisModule_ReplyWithError
                    .unwrap()(ctx, error.as_ptr())
            }
        } 
    }
}

#[allow(non_snake_case)]
extern "C" fn DeleteStatement(
    ctx: *mut r::rm::ffi::RedisModuleCtx,
    argv: *mut *mut r::rm::ffi::RedisModuleString,
    argc: ::std::os::raw::c_int,
) -> i32{
    let (_ctx, argvector) = r::create_argument(ctx, argv, argc);
    match argvector.len() {
        3 => {
            match get_db_channel_from_name(ctx, &argvector[1]) {
                Ok(ch) => {
                    let blocked_client =
                        r::BlockedClient {
                            client: unsafe {

                                r::rm::ffi::RedisModule_BlockClient.unwrap()(ctx,
                                                              Some(reply_create_statement),
                                                              Some(timeout),
                                                              Some(free_privdata),
                                                              10000)
                            },
                        };
                    let cmd = r::Command::DeleteStatement {
                        identifier: argvector[2],
                        client: blocked_client,
                    };
                    match ch.send(cmd) {
                        Ok(()) => {
                            replicate(ctx, String::from("REDISQL.DELETE_STATEMENT.NOW"), argv, argc);
                            r::rm::ffi::REDISMODULE_OK
                        }
                        Err(_) => r::rm::ffi::REDISMODULE_OK,
                    }
                }
                Err(key_type) => {
                    reply_with_error_from_key_type(ctx, key_type)
                }
            }
        }
        _ => {
            let error = CString::new("Wrong number of arguments, it \
                                      accepts 3")
                    .unwrap();
            unsafe {
                r::rm::ffi::RedisModule_ReplyWithError
                    .unwrap()(ctx, error.as_ptr())
            }
        }
    }
}


#[allow(non_snake_case)]
extern "C" fn CreateDB(ctx: *mut r::rm::ffi::RedisModuleCtx,
                       argv: *mut *mut r::rm::ffi::RedisModuleString,
                       argc: ::std::os::raw::c_int)
-> i32{

    let (context, argvector) = r::create_argument(ctx, argv, argc);

    match argvector.len() {
        2 | 3 => {
            let key_name = r::rm::RMString::new(context,
                                                &argvector[1]);
            let key = unsafe {
                r::rm::ffi::Export_RedisModule_OpenKey(
                    ctx,
                    key_name.ptr,
                    r::rm::ffi::REDISMODULE_WRITE,
                )
            };
            let safe_key = r::RedisKey { key: key };
            match unsafe {
                      r::rm::ffi::RedisModule_KeyType
                          .unwrap()(safe_key.key)
                  } {
                r::rm::ffi::REDISMODULE_KEYTYPE_EMPTY => {
                    let (path, in_memory): (&str,
                                            bool) =
                        match argvector.len() {
                            3 => (&argvector[2], false),
                            _ => (":memory:", true),
                        };
                    match sql::get_arc_connection(path) {
                        Ok(rc) => {
                            match r::create_metadata_table(rc.clone())
                                      .and_then(|_| {
                                r::insert_metadata(
                                        rc.clone(),
                                        "setup",
                                        "path",
                                        path,
                                    ).and_then(|_| {
                                r::enable_foreign_key(rc.clone())
                                })
                            }) {
                                Err(e) => e.reply(ctx),
                                Ok(()) => {
                                    let (tx, rx) = channel();
                                    let db = r::DBKey::new_from_arc(tx, rc, in_memory);
                                    let loop_data = db.loop_data.clone();
                                    thread::spawn(move || {
                                        r::listen_and_execute(&loop_data, &rx);
                                    });
                                    let ptr = Box::into_raw(Box::new(db));
                                    let type_set = unsafe {
                                        r::rm::ffi::RedisModule_ModuleTypeSetValue.unwrap()(safe_key.key, r::rm::ffi::DBType, ptr as *mut std::os::raw::c_void)
                                    };
                                        
                                    match type_set {
                                        r::rm::ffi::REDISMODULE_OK => {
                                            let ok = r::QueryResult::OK {to_replicate: true};
                                            ReplicateVerbatim(context);
                                            ok.reply(ctx)
                                        }
                                        r::rm::ffi::REDISMODULE_ERR => {
                                            let err = CString::new("ERR - Error in saving the database inside Redis").unwrap();
                                            unsafe { r::rm::ffi::RedisModule_ReplyWithSimpleString.unwrap()(ctx, err.as_ptr()) }
                                        }
                                        _ => {
                                            let err = CString::new("ERR - Error unknow").unwrap();
                                            unsafe { r::rm::ffi::RedisModule_ReplyWithSimpleString.unwrap()(ctx, err.as_ptr()) }
                                        }
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            let error = CString::new("Err - Error \
                                                      opening the in \
                                                      memory databade")
                                .unwrap();
                            unsafe {
                                r::rm::ffi::RedisModule_ReplyWithError.unwrap()(ctx, error.as_ptr())
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
                            .unwrap()(ctx, error.as_ptr())
                    }
                }
            }
        }
        _ => {
            let error = CString::new("Wrong number of arguments, it \
                                      accepts 2 or 3")
                    .unwrap();
            unsafe {
                r::rm::ffi::RedisModule_ReplyWithError
                    .unwrap()(ctx, error.as_ptr())
            }
        }
    }
}

unsafe extern "C" fn free_db(db_ptr: *mut ::std::os::raw::c_void) {

    let db: Box<r::DBKey> = Box::from_raw(db_ptr as *mut r::DBKey);
    let tx = db.tx.clone();

    match tx.send(r::Command::Stop) {
        _ => (),
    }
}



unsafe extern "C" fn rdb_save(rdb: *mut r::rm::ffi::RedisModuleIO,
                              value: *mut std::os::raw::c_void) {

    let db: *mut r::DBKey =
        Box::into_raw(Box::from_raw(value as *mut r::DBKey));

    if (*db).in_memory {

        let path = format!("rediSQL_rdb_{}.sqlite", Uuid::new_v4());

        let db = (*db).loop_data.get_db();
        let conn = &db.lock().unwrap();
        match r::create_backup(conn, &path) {
            Err(e) => println!("{}", e),
            Ok(not_done)
                if !sql::backup_complete_with_done(not_done) => {
                println!("Return NOT DONE: {}", not_done)
            }
            Ok(_) => {
                match File::open(path.clone()) {
                    Err(e) => println!("{}", e),
                    Ok(f) => {
                        match r::write_file_to_rdb(f, rdb) {
                            Ok(()) => {
                            match remove_file(path) {
                                _ => ()
                            }
                        }
                            Err(_) => {
                                println!("Impossible to write the file \
                                          in the rdb file");
                            }
                        }
                    }
                }
            }
        }
    }
}

unsafe extern "C" fn rdb_load(rdb: *mut r::rm::ffi::RedisModuleIO,
                              _encoding_version: i32)
                              -> *mut std::os::raw::c_void {

    let path = format!("rediSQL_rdb_{}.sqlite", Uuid::new_v4());
    match File::create(path.clone()) {
        Err(_) => {
            println!("Was impossible to create a file!");
            ptr::null_mut()
        }
        Ok(ref mut f) => {
            match r::write_rdb_to_file(f, rdb) {
                Err(_) => {
                    println!("Was impossible to write the rdb file!");
                    ptr::null_mut()
                }
                Ok(()) => {
                    match sql::open_connection(":memory:") {
                        Err(_) => {
                            println!("Was impossible to open the in memory db!");
                            ptr::null_mut()
                        },
                        Ok(in_mem) => {
                            match sql::open_connection(&path) {
                                Err(_) => {
                                    println!("Error in opening the rdb database");
                                    ptr::null_mut()
                                }
                                Ok(on_disk) => {
                                    match r::make_backup(&on_disk, &in_mem) {
                                        Err(e) => {
                                            println!("{}", e);
                                            ptr::null_mut()
                                        }
                                        Ok(_) => {
                                            let (tx, rx) = channel();
                                            let db = r::DBKey::new(tx, in_mem, true);
                                            let loop_data = db.loop_data.clone();

                                            thread::spawn(move || { 
                                                r::listen_and_execute(&loop_data, &rx)});
                                        


                                            Box::into_raw(Box::new(db)) as *mut std::os::raw::c_void
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}


#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn RedisModule_OnLoad(
    ctx: *mut r::rm::ffi::RedisModuleCtx,
    _argv: *mut *mut r::rm::ffi::RedisModuleString,
    _argc: i32,
) -> i32{

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
            .unwrap()(ctx.as_ptr(),
                              ptr_data_type_name,
                              1,
                              &mut types);
    }


    if unsafe { r::rm::ffi::DBType } == std::ptr::null_mut() {
        return r::rm::ffi::REDISMODULE_ERR;
    }

    match register_write_function(ctx,
                                  String::from("REDISQL.CREATE_DB"),
                                  CreateDB) {
        Ok(()) => (),
        Err(e) => return e,
    }

    match register_write_function(ctx,
                                  String::from("REDISQL.EXEC"),
                                  Exec) {
        Ok(()) => (),
        Err(e) => return e,
    }

    match register_function(ctx,
                            String::from("REDISQL.QUERY"),
                            String::from("readonly"),
                            Query) {
        Ok(()) => (),
        Err(e) => return e,
    }

    match register_write_function(ctx,
                                  String::from("REDISQL.CREATE_STATEMENT",),
                                  CreateStatement) {
        Ok(()) => (),
        Err(e) => return e,
    }

    match register_write_function(ctx,
                                  String::from("REDISQL.EXEC_STATEMENT",),
                                  ExecStatement) {
        Ok(()) => (),
        Err(e) => return e,
    }

    match register_write_function(ctx,
                                  String::from("REDISQL.UPDATE_STATEMENT",),
                                  UpdateStatement) {
        Ok(()) => (),
        Err(e) => return e,
    }

    match register_write_function(ctx,
                                  String::from("REDISQL.DELETE_STATEMENT",),
                                  DeleteStatement) {
        Ok(()) => (),
        Err(e) => return e,
    }

    match register_function(ctx,
                            String::from("REDISQL.QUERY_STATEMENT"),
                            String::from("readonly"),
                            QueryStatement) {
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
