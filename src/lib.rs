extern crate libc;
extern crate uuid;

use std::ffi::{CString, CStr};
use std::string;
use std::mem;
use std::ptr;
use std::fs::{remove_file, File};
use std::io::BufReader;

use std::io::{Read, Write};

use std::thread;
use std::sync::mpsc::{Receiver, RecvError, channel, Sender};

use uuid::Uuid;

#[allow(dead_code)]
#[allow(non_snake_case)]
#[allow(non_camel_case_types)]
#[allow(non_upper_case_globals)]
#[allow(improper_ctypes)]
mod ffi {
    include!(concat!(env!("OUT_DIR"), "/bindings_redis.rs"));
}

mod sqlite;
use sqlite as sql;


trait RedisReply {
    fn reply(&self, ctx: *mut ffi::RedisModuleCtx) -> i32;
}

impl RedisReply for sql::Entity {
    fn reply(&self, ctx: *mut ffi::RedisModuleCtx) -> i32 {
        unsafe {
            match *self {
                sql::Entity::Integer { int } => {
                    ffi::RedisModule_ReplyWithLongLong.unwrap()(ctx,
                                                                int as i64)
                }
                sql::Entity::Float { float } => {
                    ffi::RedisModule_ReplyWithDouble.unwrap()(ctx, float)
                }
                sql::Entity::Text { ref text } => {
                    let text_c = CString::new(text.clone()).unwrap();
                    ffi::RedisModule_ReplyWithStringBuffer.unwrap()(ctx, text_c.as_ptr(), text.len())
                }
                sql::Entity::Blob { ref blob } => {
                    let blob_c = CString::new(blob.clone()).unwrap();
                    ffi::RedisModule_ReplyWithStringBuffer.unwrap()(ctx, blob_c.as_ptr(), blob.len())
                }
                sql::Entity::Null => {
                    ffi::RedisModule_ReplyWithNull.unwrap()(ctx)
                }
                sql::Entity::OK => {
                    let ok = String::from("OK");
                    let ok_c = CString::new(ok.clone()).unwrap();
                    ffi::RedisModule_ReplyWithStringBuffer.unwrap()(ctx, ok_c.as_ptr(), ok.len())
                }                
                sql::Entity::DONE => {
                    let done = String::from("DONE");
                    let done_c = CString::new(done.clone()).unwrap();
                    ffi::RedisModule_ReplyWithStringBuffer.unwrap()(ctx,
                                                                    done_c.as_ptr(),
                                                                    done.len())
                }
            }
        }
    }
}

fn reply_with_string(ctx: *mut ffi::RedisModuleCtx, s: String) -> i32 {
    let len = s.len();
    let s = CString::new(s).unwrap();
    unsafe {
        ffi::RedisModule_ReplyWithStringBuffer.unwrap()(ctx,
                                                        s.as_ptr(),
                                                        len)
    }
}

impl RedisReply for sql::SQLite3Error {
    fn reply(&self, ctx: *mut ffi::RedisModuleCtx) -> i32 {
        let error = format!("{}", self);
        reply_with_string(ctx, error)
    }
}


#[allow(dead_code)]
struct Context {
    ctx: *mut ffi::RedisModuleCtx,
}

fn create_argument(ctx: *mut ffi::RedisModuleCtx,
                   argv: *mut *mut ffi::RedisModuleString,
                   argc: i32)
                   -> (Context, Vec<String>) {
    let context = Context { ctx: ctx };
    let argvector = parse_args(argv, argc).unwrap();
    (context, argvector)
}

struct RedisModuleString {
    rm_string: *mut ffi::RedisModuleString,
}

fn create_rm_string(ctx: *mut ffi::RedisModuleCtx,
                    s: String)
                    -> RedisModuleString {
    let l = s.len();
    let cs = CString::new(s).unwrap();


    RedisModuleString {
        rm_string: unsafe {
            ffi::RedisModule_CreateString.unwrap()(ctx, cs.as_ptr(), l)
        },
    }
}

#[repr(C)]
struct RedisKey {
    key: *mut ffi::RedisModuleKey,
}

impl Drop for RedisKey {
    fn drop(&mut self) {
        unsafe {
            ffi::RedisModule_CloseKey.unwrap()(self.key);
        }
    }
}

enum Command {
    Stop,
    Exec {
        query: String,
        client: BlockedClient,
    },
}

struct BlockedClient {
    client: *mut ffi::RedisModuleBlockedClient,
}

unsafe impl Send for BlockedClient {}

enum QueryResult {
    OK,
    DONE,
    Array { array: Vec<sql::Row> },
}

fn execute_query(db: &sqlite::RawConnection,
                 query: String)
                 -> Result<QueryResult, sql::SQLite3Error> {
    match sql::create_statement(&db, query) {
        Ok(stmt) => {
            match sql::execute_statement(stmt) {
                Ok(cursor) => {
                    match cursor {
                        sql::Cursor::OKCursor => Ok(QueryResult::OK),
                        sql::Cursor::DONECursor => Ok(QueryResult::DONE),

                        sql::Cursor::RowsCursor { .. } => {
                            Ok(QueryResult::Array {
                                array: cursor.collect::<Vec<sql::Row>>(),
                            })
                        }
                    }
                }
                Err(e) => Err(e),
            }
        }
        Err(e) => Err(e),
    }
}


fn listen_and_execute(db: sqlite::RawConnection, rx: Receiver<Command>) {

    loop {
        match rx.recv() {
            Ok(Command::Exec { query, client }) => {
                let result = Box::new(execute_query(&db, query));

                unsafe {
                    ffi::RedisModule_UnblockClient.unwrap()(client.client,
                                                       Box::into_raw(result) as *mut std::os::raw::c_void)
                };

            }
            Ok(Command::Stop) => return,
            Err(RecvError) => return,
        }
    }
}

fn reply_with_simple_string(ctx: *mut ffi::RedisModuleCtx,
                            s: String)
                            -> i32 {
    let s = CString::new(s).unwrap();
    unsafe {
        ffi::RedisModule_ReplyWithSimpleString.unwrap()(ctx, s.as_ptr())
    }
}

fn reply_with_ok(ctx: *mut ffi::RedisModuleCtx) -> i32 {
    reply_with_simple_string(ctx, String::from("OK"))
}

fn reply_with_done(ctx: *mut ffi::RedisModuleCtx) -> i32 {
    reply_with_simple_string(ctx, String::from("DONE"))
}

fn reply_with_array(ctx: *mut ffi::RedisModuleCtx,
                    array: Vec<sql::Row>)
                    -> i32 {
    let len = array.len() as i64;
    unsafe {
        ffi::RedisModule_ReplyWithArray.unwrap()(ctx, len);
    }
    for row in array {
        unsafe {
            ffi::RedisModule_ReplyWithArray.unwrap()(ctx,
                                                     row.len() as i64);
        }
        for entity in row {
            entity.reply(ctx);
        }
    }
    ffi::REDISMODULE_OK
}

extern "C" fn reply_exec(ctx: *mut ffi::RedisModuleCtx,
                         _argv: *mut *mut ffi::RedisModuleString,
                         _argc: ::std::os::raw::c_int)
                         -> i32 {
    let result =
        unsafe { ffi::RedisModule_GetBlockedClientPrivateData.unwrap()(ctx) as *mut Result<QueryResult, sql::SQLite3Error>};
    let result: Box<Result<QueryResult, sql::SQLite3Error>> =
        unsafe { Box::from_raw(result) };
    match *result {
        Ok(QueryResult::OK) => reply_with_ok(ctx),
        Ok(QueryResult::DONE) => reply_with_done(ctx),
        Ok(QueryResult::Array { array }) => reply_with_array(ctx, array),
        Err(error) => error.reply(ctx),
    }
}

extern "C" fn timeout(ctx: *mut ffi::RedisModuleCtx,
                      _argv: *mut *mut ffi::RedisModuleString,
                      _argc: ::std::os::raw::c_int)
                      -> i32 {
    unsafe { ffi::RedisModule_ReplyWithNull.unwrap()(ctx) }
}


extern "C" fn free_privdata(_arg: *mut ::std::os::raw::c_void) {}


#[allow(non_snake_case)]
extern "C" fn Exec(ctx: *mut ffi::RedisModuleCtx,
                   argv: *mut *mut ffi::RedisModuleString,
                   argc: ::std::os::raw::c_int)
                   -> i32 {
    let (_context, argvector) = create_argument(ctx, argv, argc);

    match argvector.len() {
        3 => {
            let key_name = create_rm_string(ctx, argvector[1].clone());
            let key =
                unsafe {
                    ffi::Export_RedisModule_OpenKey(ctx,
                                                key_name.rm_string,
                                                ffi::REDISMODULE_WRITE)
                };
            let safe_key = RedisKey { key: key };
            let key_type = unsafe {
                ffi::RedisModule_KeyType.unwrap()(safe_key.key)
            };
            if unsafe {
                ffi::DBType ==
                ffi::RedisModule_ModuleTypeGetType.unwrap()(safe_key.key)
            } {

                let db_ptr = unsafe {
                    ffi::RedisModule_ModuleTypeGetValue.unwrap()(safe_key.key) as *mut DBKey
                };


                let db: Box<DBKey> = unsafe { Box::from_raw(db_ptr) };

                let ch = db.tx.clone();

                std::mem::forget(db);

                let blocked_client = BlockedClient {
                    client:
                        unsafe {
                        ffi::RedisModule_BlockClient.unwrap()(ctx,
                                                              Some(reply_exec),
                                                              Some(timeout),
                                                              Some(free_privdata),
                                                              10000)
                    },
                };

                let cmd = Command::Exec {
                    query: argvector[2].clone(),
                    client: blocked_client,
                };

                match ch.send(cmd) {
                    Ok(()) => ffi::REDISMODULE_OK,
                    Err(_) => ffi::REDISMODULE_OK,
                }

            } else {
                match key_type {
                    ffi::REDISMODULE_KEYTYPE_EMPTY => {
                        let error = CString::new("ERR - Error the key \
                                                  is empty")
                            .unwrap();
                        unsafe {
                        ffi::RedisModule_ReplyWithError.unwrap()(ctx, error.as_ptr())
                    }
                    }
                    _ => {
                        let error = CStr::from_bytes_with_nul(ffi::REDISMODULE_ERRORMSG_WRONGTYPE).unwrap();
                        unsafe {
                        ffi::RedisModule_ReplyWithError.unwrap()(ctx, error.as_ptr())
                    }
                    }
                }

            }
        }
        _ => {
            let error = CString::new("Wrong number of arguments, it \
                                      accepts 3")
                .unwrap();
            unsafe {
                ffi::RedisModule_ReplyWithError.unwrap()(ctx,
                                                         error.as_ptr())
            }
        }
    }
}

struct DBKey {
    tx: Sender<Command>,
    db: sqlite::RawConnection,
    in_memory: bool,
}

fn create_metadata_table(db: &sql::RawConnection)
                         -> Result<(), sql::SQLite3Error> {
    let statement = String::from("CREATE TABLE \
                                 RediSQLMetadata(data_type TEXT, key \
                                 TEXT, value TEXT);");

    match sql::create_statement(&db, statement) {
        Err(e) => Err(e),
        Ok(stmt) => {
            match sql::execute_statement(stmt) {
                Ok(_) => Ok(()),
                Err(e) => Err(e),
            }
        }
    }
}

fn insert_metadata(db: &sql::RawConnection,
                   data_type: String,
                   key: String,
                   value: String)
                   -> Result<(), sql::SQLite3Error> {
    let statement = String::from("INSERT INTO RediSQLMetadata \
                                  VALUES(?, ?, ?);");

    match sql::create_statement(&db, statement) {
        Err(e) => Err(e),
        Ok(stmt) => {
            match sql::bind_text(&db, &stmt, 1, data_type) {
                Err(e) => Err(e),
                Ok(()) => match sql::bind_text(&db, &stmt, 2, key) {
                    Err(e) => Err(e),
                    Ok(()) => match sql::bind_text(&db, &stmt, 3, value) {
                        Err(e) => Err(e),
                        Ok(()) => {
                            match sql::execute_statement(stmt) {
                                Ok(_) => {
                                    Ok(())
                                },
                                Err(e) => Err(e),
                            }
                        }
                    }    
                },
            }
        }
    }
}

#[allow(non_snake_case)]
extern "C" fn CreateDB(ctx: *mut ffi::RedisModuleCtx,
                       argv: *mut *mut ffi::RedisModuleString,
                       argc: ::std::os::raw::c_int)
                       -> i32 {


    let (_context, argvector) = create_argument(ctx, argv, argc);

    match argvector.len() {
        2 | 3 => {
            let key_name = create_rm_string(ctx, argvector[1].clone());
            let key =
                unsafe {
                    ffi::Export_RedisModule_OpenKey(ctx, key_name.rm_string, ffi::REDISMODULE_WRITE)
                };
            let safe_key = RedisKey { key: key };
            match unsafe {
                ffi::RedisModule_KeyType.unwrap()(safe_key.key)
            } {
                ffi::REDISMODULE_KEYTYPE_EMPTY => {
                    let (path, in_memory) = match argvector.len() {
                        3 => (String::from(argvector[2].clone()), false),
                        _ => (String::from(":memory:"), true),
                    };
                    match sql::open_connection(path.clone()) {
                        Ok(rc) => {
                            match create_metadata_table(&rc)
                                .and_then(|_| {
                                    insert_metadata(&rc,
                                                    "setup".to_owned(),
                                                    "path".to_owned(),
                                                    path)
                                }) {
                                Err(e) => e.reply(ctx),
                                Ok(()) => {
                                    let (tx, rx) = channel();
                                    let db = DBKey {
                                        tx: tx,
                                        db: rc.clone(),
                                        in_memory: in_memory,
                                    };
                                    thread::spawn(move || {
                                        listen_and_execute(rc, rx);
                                    });
                                    let ptr = Box::into_raw(Box::new(db));
                                    let type_set = unsafe {
                                        ffi::RedisModule_ModuleTypeSetValue.unwrap()(safe_key.key, ffi::DBType, ptr as *mut std::os::raw::c_void)
                                    };
                                        
                                    match type_set {
                                        ffi::REDISMODULE_OK => {
                                            let ok = CString::new("OK").unwrap();
                                            unsafe { ffi::RedisModule_ReplyWithSimpleString.unwrap()(ctx, ok.as_ptr()) }
                                        }
                                        ffi::REDISMODULE_ERR => {
                                            let err = CString::new("ERR - Error in saving the database inside Redis").unwrap();
                                            unsafe { ffi::RedisModule_ReplyWithSimpleString.unwrap()(ctx, err.as_ptr()) }
                                        }
                                        _ => {
                                            let err = CString::new("ERR - Error unknow").unwrap();
                                            unsafe { ffi::RedisModule_ReplyWithSimpleString.unwrap()(ctx, err.as_ptr()) }
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
                            unsafe { ffi::RedisModule_ReplyWithError.unwrap()(ctx, error.as_ptr()) }
                        }
                    }
                }
                _ => {
                    let error = CStr::from_bytes_with_nul(ffi::REDISMODULE_ERRORMSG_WRONGTYPE).unwrap();
                    unsafe { ffi::RedisModule_ReplyWithError.unwrap()(ctx, error.as_ptr()) }
                }
            }
        }
        _ => {
            let error = CString::new("Wrong number of arguments, it \
                                      accepts 2 or 3")
                .unwrap();
            unsafe {
                ffi::RedisModule_ReplyWithError.unwrap()(ctx,
                                                         error.as_ptr())
            }
        }
    }
}

fn parse_args(argv: *mut *mut ffi::RedisModuleString,
              argc: i32)
              -> Result<Vec<String>, string::FromUtf8Error> {
    let mut args: Vec<String> = Vec::with_capacity(argc as usize);
    for i in 0..argc {
        let redis_str = unsafe { *argv.offset(i as isize) };
        args.push(string_ptr_len(redis_str));
    }
    Ok(args)
}

pub fn string_ptr_len(str: *mut ffi::RedisModuleString) -> String {
    unsafe {
        CStr::from_ptr(ffi::RedisModule_StringPtrLen.unwrap()(str, std::ptr::null_mut()))
            .to_string_lossy()
            .into_owned()
    }
}

unsafe extern "C" fn free_db(db_ptr: *mut ::std::os::raw::c_void) {

    let db: Box<DBKey> = Box::from_raw(db_ptr as *mut DBKey);
    let tx = db.tx.clone();

    match tx.send(Command::Stop) {
        _ => (),
    }
}

fn make_backup(conn1: &sql::RawConnection,
               conn2: &sql::RawConnection)
               -> Result<i32, sql::SQLite3Error> {
    match sql::create_backup(conn1, conn2) {
        Err(e) => Err(e),
        Ok(bk) => {
            let mut result = sql::backup_step(bk, 1);
            while sql::backup_should_step_again(result) {
                result = sql::backup_step(bk, 1);
            }
            sql::backup_finish(bk);
            Ok(result)
        }
    }
}

fn create_backup(conn: &sql::RawConnection,
                 path: String)
                 -> Result<i32, sql::SQLite3Error> {
    match sql::open_connection(path) {
        Err(e) => Err(e),
        Ok(new_db) => make_backup(conn, &new_db),
    }
}

fn write_file_to_rdb(f: File,
                     rdb: *mut ffi::RedisModuleIO)
                     -> Result<(), std::io::Error> {

    let block_size = 1024 * 4 as i64;
    let lenght = f.metadata().unwrap().len();
    let blocks = lenght / block_size as u64;

    println!("Dimension file: {}\n Blocks: {}", lenght, blocks);

    unsafe {
        ffi::RedisModule_SaveSigned.unwrap()(rdb, blocks as i64);
    }

    let to_write: Vec<u8> = vec![0; block_size as usize];
    let mut buffer = BufReader::with_capacity(block_size as usize, f);
    loop {
        let mut tw = to_write.clone();
        match buffer.read(tw.as_mut_slice()) {
            Ok(0) => {
                return Ok(());
            }
            Ok(n) => unsafe {
                // let cs = CString::from_vec_unchecked(tw);
                println!("Number of bytes written: {}", n);
                ffi::RedisModule_SaveStringBuffer.unwrap()(rdb,
                                                           tw.as_slice().as_ptr() as *const i8,
                                                           n)

            },
            Err(e) => return Err(e),
        }
    }

}

unsafe extern "C" fn rdb_save(rdb: *mut ffi::RedisModuleIO,
                              value: *mut std::os::raw::c_void) {

    let db: *mut DBKey =
        unsafe { Box::into_raw(Box::from_raw(value as *mut DBKey)) };

    if (*db).in_memory {

        let path = format!("rediSQL_rdb_{}.sqlite", Uuid::new_v4());

        match create_backup(&(*db).db, path.clone()) {
            Err(e) => println!("{}", e),
            Ok(not_done) if !sql::backup_complete_with_done(not_done) => {
                println!("Return NOT DONE: {}", not_done)
            }
            Ok(_) => {
                match File::open(path.clone()) {
                    Err(e) => println!("{}", e),
                    Ok(f) => {
                        match write_file_to_rdb(f, rdb) {
                            Ok(()) => {
                            remove_file(path);
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

// TODO make sure of the deallocation

struct Save_RedisModule_String {
    ptr: *mut std::os::raw::c_char,
}

impl Drop for Save_RedisModule_String {
    fn drop(&mut self) {
        unsafe { ffi::RedisModule_Free.unwrap()(self.ptr as *mut std::os::raw::c_void) }
    }
}

fn write_rdb_to_file(f: &mut File,
                     rdb: *mut ffi::RedisModuleIO)
                     -> Result<(), std::io::Error> {

    let blocks =
        unsafe { ffi::RedisModule_LoadSigned.unwrap()(rdb) as i64 };

    for _ in 0..blocks {
        let mut dimension: libc::size_t = 0;
        println!("About to load the string");
        let c_str_ptr = Save_RedisModule_String {
            ptr:
                unsafe {
                ffi::RedisModule_LoadStringBuffer.unwrap()(rdb,
                                                           &mut dimension)
            },
        };

        println!("Dimension: {}", dimension);
        if dimension == 0 {
            break;
        }
        let buffer: Vec<u8> =
            unsafe {
                Vec::from_raw_parts(c_str_ptr.ptr as *mut u8,
                                    dimension,
                                    dimension)
            };
        println!("Buffer dimension: {}, {:?}",
                 buffer.len(),
                 c_str_ptr.ptr);

        let y = f.write_all(buffer.as_slice());
        ::mem::forget(buffer);
        match y {
            Err(e) => return Err(e),
            _ => (),
        }
    }
    Ok(())
}

unsafe extern "C" fn rdb_load(rdb: *mut ffi::RedisModuleIO,
                              _encoding_version: i32)
                              -> *mut std::os::raw::c_void {

    let path = format!("rediSQL_rdb_{}.sqlite", Uuid::new_v4());
    match File::create(path.clone()) {
        Err(_) => {
            println!("Was impossible to create a file!");
            ptr::null_mut()
        }
        Ok(ref mut f) => {
            match write_rdb_to_file(f, rdb) {
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
                                    match make_backup(&on_disk, &in_mem) {
                                        Err(e) => {
                                            println!("{}", e);
                                            ptr::null_mut()
                                        }
                                        Ok(_) => {
                                            let (tx, rx) = channel();
                                            let db = DBKey {
                                                tx: tx,
                                                db: in_mem.clone(),
                                                in_memory: true,
                                            };

                                            thread::spawn(move || { 
                                                listen_and_execute(in_mem, rx)});

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
pub extern "C" fn RedisModule_OnLoad(ctx: *mut ffi::RedisModuleCtx,
                                     _argv: *mut *mut ffi::RedisModuleString,
                                     _argc: i32)
                                     -> i32 {



    let c_data_type_name = CString::new("rediSQLDB").unwrap();
    let ptr_data_type_name = c_data_type_name.as_ptr();

    let mut types = ffi::RedisModuleTypeMethods {
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
        ffi::Export_RedisModule_Init(ctx,
                                     module_ptr_name,
                                     1,
                                     ffi::REDISMODULE_APIVER_1)
    } == ffi::REDISMODULE_ERR {
        return ffi::REDISMODULE_ERR;
    }


    unsafe {
        ffi::DBType =
            ffi::RedisModule_CreateDataType.unwrap()(ctx,
                                                     ptr_data_type_name,
                                                     1,
                                                     &mut types);
    }


    if unsafe { ffi::DBType } == std::ptr::null_mut() {
        return ffi::REDISMODULE_ERR;
    }

    let create_db: ffi::RedisModuleCmdFunc = Some(CreateDB);

    let command_c_name = CString::new("REDISQL.CREATE_DB").unwrap();
    let command_ptr_name = command_c_name.as_ptr();

    let flag_c_name = CString::new("write").unwrap();
    let flag_ptr_name = flag_c_name.as_ptr();

    if unsafe {
        ffi::RedisModule_CreateCommand.unwrap()(ctx,
                                                command_ptr_name,
                                                create_db,
                                                flag_ptr_name,
                                                0,
                                                0,
                                                0)
    } == ffi::REDISMODULE_ERR {
        return ffi::REDISMODULE_ERR;
    }

    let exec: ffi::RedisModuleCmdFunc = Some(Exec);

    let command_c_name = CString::new("REDISQL.EXEC").unwrap();
    let command_ptr_name = command_c_name.as_ptr();

    let flag_c_name = CString::new("write").unwrap();
    let flag_ptr_name = flag_c_name.as_ptr();

    if unsafe {
        ffi::RedisModule_CreateCommand.unwrap()(ctx,
                                                command_ptr_name,
                                                exec,
                                                flag_ptr_name,
                                                0,
                                                0,
                                                0)
    } == ffi::REDISMODULE_ERR {
        return ffi::REDISMODULE_ERR;
    }
    ffi::REDISMODULE_OK
}
