#![warn(unused_extern_crates)]

mod commands_v1;
mod common;
mod v2;

#[macro_use]
extern crate log;

use env_logger::{Builder as logBuilder, Target as logTarget};
use redisql_lib::redis as r;
use redisql_lib::redis::{
    get_path_from_db, is_redisql_database, register_function,
    register_function_with_keys, register_write_function, LoopData,
};
use redisql_lib::redis_type::Context;
use redisql_lib::sqlite as sql;
use std::ffi::CString;
use std::fs::{remove_file, File};
use std::ptr;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::thread;
use uuid::Uuid;

use sync_engine::{register, WriteAOF};

use commands_v1::{
    CreateDB, CreateStatement, DeleteStatement, Exec, ExecStatement,
    GetStatistics, MakeCopy, Query, QueryInto, QueryStatement,
    QueryStatementInto, RediSQLVersion, UpdateStatement,
};
use v2::create_db::CreateDB_v2;
use v2::exec::Exec_v2;
use v2::exec::Query_v2;
use v2::statement::Statement_v2;

#[cfg(not(feature = "pro"))]
extern crate telemetrics;

unsafe extern "C" fn rdb_save(
    rdb: *mut r::rm::ffi::RedisModuleIO,
    value: *mut std::os::raw::c_void,
) {
    let db: *mut r::DBKey =
        Box::into_raw(Box::from_raw(value as *mut r::DBKey));

    let path = format!("rediSQL_rdb_write_{}.sqlite", Uuid::new_v4());

    let db = (*db).loop_data.get_db();
    let conn = &db.lock().unwrap();
    match r::create_backup(conn, &path) {
        Err(e) => println!("{}", e),
        Ok(not_done) if !sql::backup_complete_with_done(not_done) => {
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

// in the RDB file we store a serialized database, nothing else.
// the first step of loading is to write the content of the RDB into a random file
// then we open a connection to the random file, called on_disk
// then we read what should be the path of the database to read.
// It could be either:
// 1) :memory:
// 2) A file that does not exists
// 3) A file that already exists
// Hence, in case of
// 1) we create a new in-memory database and we backup the on_disk database into the new in-memory database
// 2) Similarly we just creare a new database and we backup the content over there
// 3) We try to open the database, if we fail we just exit (like in all other cases) then, we assume that the DB just loaded is more up to date than the one in the RDB thus we don't do any data movement.
// Finally we start the whole threads and bell and whistles!
unsafe extern "C" fn rdb_load(
    rdb: *mut r::rm::ffi::RedisModuleIO,
    _encoding_version: i32,
) -> *mut std::os::raw::c_void {
    let path = format!("rediSQL_rdb_read_{}.sqlite", Uuid::new_v4());

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

    let on_disk = match sql::Connection::open_connection(&path) {
        Err(_) => {
            println!("Error in opening the rdb database");
            return ptr::null_mut();
        }
        Ok(on_disk) => on_disk,
    };

    let on_disk = Arc::new(Mutex::new(on_disk));
    let previous_path = match get_path_from_db(on_disk.clone()) {
        Ok(path) => path,
        Err(e) => {
            println!("Warning trying to load from RDB: {}", e);
            ":memory:".to_string()
        }
    };

    let db = match sql::Connection::open_connection(&previous_path) {
        Err(_) => {
            println!("WARN: Was impossible to open the database {}, using an in-memory database!", previous_path);
            match sql::Connection::open_connection(":memory:") {
                Err(_) => {
                    println!("ERROR: Was impossible to open also an in-memory database, fail!");
                    return ptr::null_mut();
                }
                Ok(in_mem) => in_mem,
            }
        }
        Ok(db) => db,
    };

    let conn = Arc::new(Mutex::new(db));
    if !is_redisql_database(conn.clone()) {
        if let Err(e) = r::make_backup(
            &on_disk.lock().unwrap(),
            &conn.lock().unwrap(),
        ) {
            println!("ERROR: Was impossible to copy the content of the RDB file into a database {}", e);
            return ptr::null_mut();
        }
    }

    let (tx, rx) = channel();
    let db = r::DBKey::new_from_arc(tx, conn);
    let mut loop_data = db.loop_data.clone();

    thread::spawn(move || r::listen_and_execute(&mut loop_data, &rx));

    match remove_file(path) {
        _ => (),
    };

    Box::into_raw(Box::new(db)) as *mut std::os::raw::c_void
}

unsafe extern "C" fn free_db(db_ptr: *mut ::std::os::raw::c_void) {
    let db: Box<r::DBKey> = Box::from_raw(db_ptr as *mut r::DBKey);
    let tx = &db.tx;

    match tx.send(r::Command::Stop) {
        _ => (),
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

    logBuilder::new()
        .filter_level(log::LevelFilter::Debug)
        .target(logTarget::Stdout)
        .init();

    #[cfg(not(feature = "pro"))]
    thread::spawn(telemetrics::start_telemetrics);

    let c_data_type_name = CString::new("rediSQLDB").unwrap();
    let ptr_data_type_name = c_data_type_name.as_ptr();

    let mut types = r::rm::ffi::RedisModuleTypeMethods {
        version: 1,
        rdb_load: Some(rdb_load),
        rdb_save: Some(rdb_save),
        aof_rewrite: Some(WriteAOF),
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
        "REDISQL.V1.CREATE_DB",
        CreateDB,
    ) {
        Ok(()) => (),
        Err(e) => return e,
    }

    match register_write_function(&ctx, "REDISQL.V1.EXEC", Exec) {
        Ok(()) => (),
        Err(e) => return e,
    }

    match register_function(
        &ctx,
        "REDISQL.V1.QUERY",
        "readonly",
        Query,
    ) {
        Ok(()) => (),
        Err(e) => return e,
    }

    match register_function_with_keys(
        &ctx,
        "REDISQL.V1.QUERY.INTO",
        "readonly",
        1,
        2,
        1,
        QueryInto,
    ) {
        Ok(()) => (),
        Err(e) => return e,
    }

    match register_write_function(
        &ctx,
        "REDISQL.V1.CREATE_STATEMENT",
        CreateStatement,
    ) {
        Ok(()) => (),
        Err(e) => return e,
    }

    match register_write_function(
        &ctx,
        "REDISQL.V1.EXEC_STATEMENT",
        ExecStatement,
    ) {
        Ok(()) => (),
        Err(e) => return e,
    }

    match register_write_function(
        &ctx,
        "REDISQL.V1.UPDATE_STATEMENT",
        UpdateStatement,
    ) {
        Ok(()) => (),
        Err(e) => return e,
    }

    match register_write_function(
        &ctx,
        "REDISQL.V1.DELETE_STATEMENT",
        DeleteStatement,
    ) {
        Ok(()) => (),
        Err(e) => return e,
    }

    match register_function(
        &ctx,
        "REDISQL.V1.QUERY_STATEMENT",
        "readonly",
        QueryStatement,
    ) {
        Ok(()) => (),
        Err(e) => return e,
    }

    match register_function_with_keys(
        &ctx,
        "REDISQL.V1.QUERY_STATEMENT.INTO",
        "readonly",
        1,
        2,
        1,
        QueryStatementInto,
    ) {
        Ok(()) => (),
        Err(e) => return e,
    }

    match register_write_function(&ctx, "REDISQL.V1.COPY", MakeCopy) {
        Ok(()) => (),
        Err(e) => return e,
    }

    match register_function(
        &ctx,
        "REDISQL.V1.STATISTICS",
        "readonly",
        GetStatistics,
    ) {
        Ok(()) => (),
        Err(e) => return e,
    }

    match register_function(
        &ctx,
        "REDISQL.V1.VERSION",
        "readonly",
        RediSQLVersion,
    ) {
        Ok(()) => (),
        Err(e) => return e,
    }

    /*
    match register_write_function(
        &ctx,
        "REDISQL.ADD_CONNECTION",
        AddRediSQLConnection,
    ) {
        Ok(()) => (),
        Err(e) => return e,
    }
    */

    match register_write_function(
        &ctx,
        "REDISQL.V2.CREATE_DB",
        CreateDB_v2,
    ) {
        Ok(()) => (),
        Err(e) => return e,
    }

    match register_write_function(
        &ctx,
        "REDISQL.CREATE_DB",
        CreateDB_v2,
    ) {
        Ok(()) => (),
        Err(e) => return e,
    }

    match register_write_function(&ctx, "REDISQL.V2.EXEC", Exec_v2) {
        Ok(()) => (),
        Err(e) => return e,
    }

    match register_write_function(&ctx, "REDISQL.EXEC", Exec_v2) {
        Ok(()) => (),
        Err(e) => return e,
    }

    match register_function(
        &ctx,
        "REDISQL.V2.QUERY",
        "readonly",
        Query_v2,
    ) {
        Ok(()) => (),
        Err(e) => return e,
    }

    match register_function(
        &ctx,
        "REDISQL.QUERY",
        "readonly",
        Query_v2,
    ) {
        Ok(()) => (),
        Err(e) => return e,
    }

    match register_write_function(
        &ctx,
        "REDISQL.V2.STATEMENT",
        Statement_v2,
    ) {
        Ok(()) => (),
        Err(e) => return e,
    }

    match register_write_function(&ctx, "REDISQL.STATEMENT", Exec_v2)
    {
        Ok(()) => (),
        Err(e) => return e,
    }

    match register(ctx) {
        Ok(()) => (),
        Err(e) => return e,
    }

    r::rm::ffi::REDISMODULE_OK
}
