extern crate libc;
extern crate uuid;

#[macro_use]
extern crate log;
extern crate env_logger;

// use env_logger::{LogBuilder, LogTarget};

use std::ffi::{CString, CStr};
use std::mem;
use std::ptr;
use std::fs::{remove_file, File};

use std::thread;
use std::sync::mpsc::{channel, Sender};

use uuid::Uuid;

mod redisql_error;

pub mod community_statement;

mod sqlite;
use sqlite as sql;

mod redis;
use redis as r;
use redis::RedisReply;

#[cfg(feature = "pro")]
mod replication;


extern "C" fn reply_exec(ctx: *mut r::ffi::RedisModuleCtx,
                         _argv: *mut *mut r::ffi::RedisModuleString,
                         _argc: ::std::os::raw::c_int)
                         -> i32 {
    let result = unsafe {
        r::ffi::RedisModule_GetBlockedClientPrivateData
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
    ctx: *mut r::ffi::RedisModuleCtx,
    _argv: *mut *mut r::ffi::RedisModuleString,
    _argc: ::std::os::raw::c_int,
) -> i32{
    let result = unsafe {
        r::ffi::RedisModule_GetBlockedClientPrivateData
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
    ctx: *mut r::ffi::RedisModuleCtx,
    _argv: *mut *mut r::ffi::RedisModuleString,
    _argc: ::std::os::raw::c_int,
) -> i32{
    let result = unsafe {
        r::ffi::RedisModule_GetBlockedClientPrivateData
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


extern "C" fn timeout(ctx: *mut r::ffi::RedisModuleCtx,
                      _argv: *mut *mut r::ffi::RedisModuleString,
                      _argc: ::std::os::raw::c_int)
                      -> i32 {
    unsafe { r::ffi::RedisModule_ReplyWithNull.unwrap()(ctx) }
}


extern "C" fn free_privdata(_arg: *mut ::std::os::raw::c_void) {}

fn get_db_channel_from_name(ctx: *mut r::ffi::RedisModuleCtx,
                            name: String)
                            -> Result<Sender<r::Command>, i32> {
    let key_name = r::create_rm_string(ctx, name);
    let key = unsafe {
        r::ffi::Export_RedisModule_OpenKey(
            ctx,
            key_name,
            r::ffi::REDISMODULE_WRITE,
        )
    };
    let safe_key = r::RedisKey { key: key };
    let key_type =
        unsafe { r::ffi::RedisModule_KeyType.unwrap()(safe_key.key) };
    if unsafe {
           r::ffi::DBType ==
           r::ffi::RedisModule_ModuleTypeGetType
               .unwrap()(safe_key.key)
       } {
        let db_ptr = unsafe {
            r::ffi::RedisModule_ModuleTypeGetValue
                .unwrap()(safe_key.key) as *mut r::DBKey
        };
        let db: Box<r::DBKey> = unsafe { Box::from_raw(db_ptr) };
        let channel = db.tx.clone();
        std::mem::forget(db);

        Ok(channel)
    } else {
        Err(key_type)
    }
}

fn reply_with_error_from_key_type(ctx: *mut r::ffi::RedisModuleCtx,
                                  key_type: i32)
                                  -> i32 {
    match key_type {
        r::ffi::REDISMODULE_KEYTYPE_EMPTY => {
            let error = CString::new("ERR - Error the key is empty")
                .unwrap();
            unsafe {
                r::ffi::RedisModule_ReplyWithError
                    .unwrap()(ctx, error.as_ptr())
            }
        }
        _ => {
            let error = CStr::from_bytes_with_nul(
                r::ffi::REDISMODULE_ERRORMSG_WRONGTYPE,
            ).unwrap();
            unsafe {
                r::ffi::RedisModule_ReplyWithError
                    .unwrap()(ctx, error.as_ptr())
            }
        }
    }
}

#[allow(non_snake_case)]
extern "C" fn ExecStatement(
    ctx: *mut r::ffi::RedisModuleCtx,
    argv: *mut *mut r::ffi::RedisModuleString,
    argc: ::std::os::raw::c_int,
) -> i32{
    let (_context, argvector) = r::create_argument(ctx, argv, argc);

    match argvector.len() {
        0...2 => {
            let error = CString::new("Wrong number of arguments, it \
                                      needs at least 3")
                    .unwrap();
            unsafe {
                r::ffi::RedisModule_ReplyWithError
                    .unwrap()(ctx, error.as_ptr())
            }
        }
        _ => {
            match get_db_channel_from_name(ctx,
                                           argvector[1].clone()) {
                Err(key_type) => {
                    reply_with_error_from_key_type(ctx, key_type)
                }
                Ok(ch) => {
                    let blocked_client =
                        r::BlockedClient {
                            client: unsafe {
                                r::ffi::RedisModule_BlockClient.unwrap()(ctx,
                                                              Some(reply_exec_statement),
                                                              Some(timeout),
                                                              Some(free_privdata),
                                                              10000)
                            },
                        };

                    let cmd = r::Command::ExecStatement {
                        identifier: argvector[2].clone(),
                        arguments: argvector[3..].to_vec(),
                        client: blocked_client,
                    };

                    match ch.send(cmd) {
                        Ok(()) => r::ffi::REDISMODULE_OK,
                        Err(_) => r::ffi::REDISMODULE_OK,
                    }
                }
            }
        }
    }
}

#[allow(non_snake_case)]
extern "C" fn Exec(ctx: *mut r::ffi::RedisModuleCtx,
                   argv: *mut *mut r::ffi::RedisModuleString,
                   argc: ::std::os::raw::c_int)
                   -> i32 {
    let (_context, argvector) = r::create_argument(ctx, argv, argc);
    match argvector.len() {
        3 => {
            match get_db_channel_from_name(ctx,
                                           argvector[1].clone()) {
                Err(key_type) => {
                    reply_with_error_from_key_type(ctx, key_type)
                }
                Ok(ch) => {
                    let blocked_client = r::BlockedClient {
                        client:
                            unsafe {
                                r::ffi::RedisModule_BlockClient
                                    .unwrap()(ctx,
                                              Some(reply_exec),
                                              Some(timeout),
                                              Some(free_privdata),
                                              10000)
                            },
                    };
                    mem::forget(ctx);
                    let cmd = r::Command::Exec {
                        query: argvector[2].clone(),
                        client: blocked_client,
                    };
                    match ch.send(cmd) {
                        Ok(()) => r::ffi::REDISMODULE_OK,
                        Err(_) => r::ffi::REDISMODULE_OK,
                    }
                }
            }
        }
        _ => {
            let error = CString::new("Wrong number of arguments, it \
                                      accepts 3")
                    .unwrap();
            unsafe {
                r::ffi::RedisModule_ReplyWithError
                    .unwrap()(ctx, error.as_ptr())
            }
        }
    }
}



#[allow(non_snake_case)]
extern "C" fn CreateStatement(
    ctx: *mut r::ffi::RedisModuleCtx,
    argv: *mut *mut r::ffi::RedisModuleString,
    argc: ::std::os::raw::c_int,
) -> i32{
    let (_ctx, argvector) = r::create_argument(ctx, argv, argc);
    match argvector.len() {
        4 => {
            match get_db_channel_from_name(ctx,
                                           argvector[1].clone()) {
                Err(key_type) => {
                    reply_with_error_from_key_type(ctx, key_type)
                }
                Ok(ch) => {
                    let blocked_client =
                        r::BlockedClient {
                            client: unsafe {
                                r::ffi::RedisModule_BlockClient.unwrap()(ctx,
                                                              Some(reply_create_statement),
                                                              Some(timeout),
                                                              Some(free_privdata),
                                                              10000)
                            },
                        };
                    let cmd = r::Command::CompileStatement {
                        identifier: argvector[2].clone(),
                        statement: argvector[3].clone(),
                        client: blocked_client,
                    };

                    match ch.send(cmd) {
                        Ok(()) => r::ffi::REDISMODULE_OK,
                        Err(_) => r::ffi::REDISMODULE_OK,
                    }

                }
            }
        }

        _ => {
            let error = CString::new("Wrong number of arguments, it \
                                      accepts 4")
                    .unwrap();
            unsafe {
                r::ffi::RedisModule_ReplyWithError
                    .unwrap()(ctx, error.as_ptr())
            }
        }
    }
}



#[allow(non_snake_case)]
extern "C" fn UpdateStatement(
    ctx: *mut r::ffi::RedisModuleCtx,
    argv: *mut *mut r::ffi::RedisModuleString,
    argc: ::std::os::raw::c_int,
) -> i32{
    let (_ctx, argvector) = r::create_argument(ctx, argv, argc);
    match argvector.len() {
        4 => {
            match get_db_channel_from_name(ctx,
                                           argvector[1].clone()) {
                Err(key_type) => {
                    reply_with_error_from_key_type(ctx, key_type)
                }
                Ok(ch) => {
                    let blocked_client =
                        r::BlockedClient {
                            client: unsafe {
                                r::ffi::RedisModule_BlockClient.unwrap()(ctx,
                                                              Some(reply_create_statement),
                                                              Some(timeout),
                                                              Some(free_privdata),
                                                              10000)
                            },
                        };

                    let cmd = r::Command::UpdateStatement {
                        identifier: argvector[2].clone(),
                        statement: argvector[3].clone(),
                        client: blocked_client,
                    };

                    match ch.send(cmd) {
                        Ok(()) => r::ffi::REDISMODULE_OK,
                        Err(_) => r::ffi::REDISMODULE_OK,
                    }

                }
            }
        }

        _ => {
            let error = CString::new("Wrong number of arguments, it \
                                      accepts 4")
                    .unwrap();
            unsafe {
                r::ffi::RedisModule_ReplyWithError
                    .unwrap()(ctx, error.as_ptr())
            }
        } 
    }
}

#[allow(non_snake_case)]
extern "C" fn DeleteStatement(
    ctx: *mut r::ffi::RedisModuleCtx,
    argv: *mut *mut r::ffi::RedisModuleString,
    argc: ::std::os::raw::c_int,
) -> i32{
    let (_ctx, argvector) = r::create_argument(ctx, argv, argc);
    match argvector.len() {
        3 => {
            match get_db_channel_from_name(ctx,
                                           argvector[1].clone()) {
                Ok(ch) => {
                    let blocked_client =
                        r::BlockedClient {
                            client: unsafe {

                                r::ffi::RedisModule_BlockClient.unwrap()(ctx,
                                                              Some(reply_create_statement),
                                                              Some(timeout),
                                                              Some(free_privdata),
                                                              10000)
                            },
                        };
                    let cmd = r::Command::DeleteStatement {
                        identifier: argvector[2].clone(),
                        client: blocked_client,
                    };
                    match ch.send(cmd) {
                        Ok(()) => r::ffi::REDISMODULE_OK,
                        Err(_) => r::ffi::REDISMODULE_OK,
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
                r::ffi::RedisModule_ReplyWithError
                    .unwrap()(ctx, error.as_ptr())
            }
        }
    }
}


#[allow(non_snake_case)]
extern "C" fn CreateDB(ctx: *mut r::ffi::RedisModuleCtx,
                       argv: *mut *mut r::ffi::RedisModuleString,
                       argc: ::std::os::raw::c_int)
                       -> i32 {


    let (_context, argvector) = r::create_argument(ctx, argv, argc);

    match argvector.len() {
        2 | 3 => {
            let key_name = r::create_rm_string(ctx,
                                               argvector[1].clone());
            let key = unsafe {
                r::ffi::Export_RedisModule_OpenKey(
                    ctx,
                    key_name,
                    r::ffi::REDISMODULE_WRITE,
                )
            };
            let safe_key = r::RedisKey { key: key };
            match unsafe {
                      r::ffi::RedisModule_KeyType
                          .unwrap()(safe_key.key)
                  } {
                r::ffi::REDISMODULE_KEYTYPE_EMPTY => {
                    let (path, in_memory) = match argvector.len() {
                        3 => {
                            (String::from(argvector[2].clone()),
                             false)
                        }
                        _ => (String::from(":memory:"), true),
                    };
                    match sql::open_connection(path.clone()) {
                        Ok(rc) => {
                            match r::create_metadata_table(&rc)
                                      .and_then(|_| {
                                r::insert_metadata(
                                        &rc,
                                        "setup".to_owned(),
                                        "path".to_owned(),
                                        path,
                                    )
                            }) {
                                Err(e) => e.reply(ctx),
                                Ok(()) => {
                                    let (tx, rx) = channel();
                                    let db = r::DBKey {
                                        tx: tx,
                                        db: rc.clone(),
                                        in_memory: in_memory,
                                        //statements: HashMap::new(),
                                    };
                                    thread::spawn(move || {
                                        r::listen_and_execute(rc, rx);
                                    });
                                    let ptr = Box::into_raw(Box::new(db));
                                    let type_set = unsafe {
                                        r::ffi::RedisModule_ModuleTypeSetValue.unwrap()(safe_key.key, r::ffi::DBType, ptr as *mut std::os::raw::c_void)
                                    };
                                        
                                    match type_set {
                                        r::ffi::REDISMODULE_OK => {
                                            let ok = r::QueryResult::OK {to_replicate: true};
                                            unsafe {
                                                r::ffi::RedisModule_ReplicateVerbatim.unwrap()(ctx);
                                            }
                                            ok.reply(ctx)
                                        }
                                        r::ffi::REDISMODULE_ERR => {
                                            let err = CString::new("ERR - Error in saving the database inside Redis").unwrap();
                                            unsafe { r::ffi::RedisModule_ReplyWithSimpleString.unwrap()(ctx, err.as_ptr()) }
                                        }
                                        _ => {
                                            let err = CString::new("ERR - Error unknow").unwrap();
                                            unsafe { r::ffi::RedisModule_ReplyWithSimpleString.unwrap()(ctx, err.as_ptr()) }
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
                                r::ffi::RedisModule_ReplyWithError.unwrap()(ctx, error.as_ptr())
                            }
                        }
                    }
                }
                _ => {
                    let error = CStr::from_bytes_with_nul(
                        r::ffi::REDISMODULE_ERRORMSG_WRONGTYPE,
                    ).unwrap();
                    unsafe {
                        r::ffi::RedisModule_ReplyWithError
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
                r::ffi::RedisModule_ReplyWithError
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



unsafe extern "C" fn rdb_save(rdb: *mut r::ffi::RedisModuleIO,
                              value: *mut std::os::raw::c_void) {

    let db: *mut r::DBKey =
        Box::into_raw(Box::from_raw(value as *mut r::DBKey));

    if (*db).in_memory {

        let path = format!("rediSQL_rdb_{}.sqlite", Uuid::new_v4());

        match r::create_backup(&(*db).db, path.clone()) {
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

unsafe extern "C" fn rdb_load(rdb: *mut r::ffi::RedisModuleIO,
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
                    match sql::open_connection(":memory:".to_owned()) {
                        Err(_) => {
                            println!("Was impossible to open the in memory db!");
                            ptr::null_mut()
                        },
                        Ok(in_mem) => {
                            match sql::open_connection(path) {
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
                                            let db = r::DBKey {
                                                tx: tx,
                                                db: in_mem.clone(),
                                                in_memory: true,
                                                //statements: HashMap::new(),
                                            };

                                            thread::spawn(move || { 
                                                r::listen_and_execute(in_mem, rx)});
                                        


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


fn register_function(
    ctx: *mut r::ffi::RedisModuleCtx,
    name: String,
    f: extern "C" fn(*mut r::ffi::RedisModuleCtx,
                     *mut *mut r::ffi::RedisModuleString,
                     ::std::os::raw::c_int)
                     -> i32,
) -> Result<(), i32>{

    let create_db: r::ffi::RedisModuleCmdFunc = Some(f);

    let command_c_name = CString::new(name).unwrap();
    let command_ptr_name = command_c_name.as_ptr();

    let flag_c_name = CString::new("write").unwrap();
    let flag_ptr_name = flag_c_name.as_ptr();

    if unsafe {
           r::ffi::RedisModule_CreateCommand
               .unwrap()(ctx,
                         command_ptr_name,
                         create_db,
                         flag_ptr_name,
                         0,
                         0,
                         0)
       } == r::ffi::REDISMODULE_ERR {
        return Err(r::ffi::REDISMODULE_ERR);
    }
    Ok(())
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn RedisModule_OnLoad(
    ctx: *mut r::ffi::RedisModuleCtx,
    _argv: *mut *mut r::ffi::RedisModuleString,
    _argc: i32,
) -> i32{

    sql::disable_global_memory_statistics();

    /*
    LogBuilder::new()
        .filter(None, log::LogLevelFilter::Debug)
        .target(LogTarget::Stdout)
        .init()
        .unwrap();
    */

    let c_data_type_name = CString::new("rediSQLDB").unwrap();
    let ptr_data_type_name = c_data_type_name.as_ptr();

    let mut types = r::ffi::RedisModuleTypeMethods {
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
        r::ffi::Export_RedisModule_Init(
            ctx,
            module_ptr_name,
            1,
            r::ffi::REDISMODULE_APIVER_1,
        )
    } == r::ffi::REDISMODULE_ERR
    {
        return r::ffi::REDISMODULE_ERR;
    }


    unsafe {
        r::ffi::DBType = r::ffi::RedisModule_CreateDataType
            .unwrap()(ctx, ptr_data_type_name, 1, &mut types);
    }


    if unsafe { r::ffi::DBType } == std::ptr::null_mut() {
        return r::ffi::REDISMODULE_ERR;
    }

    match register_function(ctx,
                            String::from("REDISQL.CREATE_DB"),
                            CreateDB) {
        Ok(()) => (),
        Err(e) => return e,
    }

    match register_function(ctx, String::from("REDISQL.EXEC"), Exec) {
        Ok(()) => (),
        Err(e) => return e,
    }

    match register_function(ctx,
                            String::from("REDISQL.CREATE_STATEMENT",),
                            CreateStatement) {
        Ok(()) => (),
        Err(e) => return e,
    }

    match register_function(ctx,
                            String::from("REDISQL.EXEC_STATEMENT"),
                            ExecStatement) {
        Ok(()) => (),
        Err(e) => return e,
    }

    match register_function(ctx,
                            String::from("REDISQL.UPDATE_STATEMENT",),
                            UpdateStatement) {
        Ok(()) => (),
        Err(e) => return e,
    }

    match register_function(ctx,
                            String::from("REDISQL.DELETE_STATEMENT",),
                            DeleteStatement) {
        Ok(()) => (),
        Err(e) => return e,
    }

    r::ffi::REDISMODULE_OK
}
