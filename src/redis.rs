
extern crate libc;
extern crate uuid;

use std::ffi::{CString, CStr};
use std::string;
use std::fs::File;
use std::io::BufReader;

use std::io::{Read, Write};

use std::sync::mpsc::{Receiver, RecvError, Sender};

use std;

#[allow(dead_code)]
#[allow(non_snake_case)]
#[allow(non_camel_case_types)]
#[allow(non_upper_case_globals)]
#[allow(improper_ctypes)]
pub mod ffi {
    include!(concat!(env!("OUT_DIR"), "/bindings_redis.rs"));
}

use sqlite as sql;

#[allow(dead_code)]
pub struct Context {
    ctx: *mut ffi::RedisModuleCtx,
}

pub trait RedisReply {
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

pub fn create_argument(ctx: *mut ffi::RedisModuleCtx,
                       argv: *mut *mut ffi::RedisModuleString,
                       argc: i32)
                       -> (Context, Vec<String>) {
    let context = Context { ctx: ctx };
    let argvector = parse_args(argv, argc).unwrap();
    (context, argvector)
}

pub fn create_rm_string(ctx: *mut ffi::RedisModuleCtx,
                        s: String)
                        -> *mut ffi::RedisModuleString {
    let l = s.len();
    let cs = CString::new(s).unwrap();

    unsafe { ffi::RedisModule_CreateString.unwrap()(ctx, cs.as_ptr(), l) }
}

#[repr(C)]
pub struct RedisKey {
    pub key: *mut ffi::RedisModuleKey,
}

impl Drop for RedisKey {
    fn drop(&mut self) {
        unsafe {
            ffi::RedisModule_CloseKey.unwrap()(self.key);
        }
    }
}


pub enum Command {
    Stop,
    Exec {
        query: String,
        client: BlockedClient,
    },
}

pub struct BlockedClient {
    pub client: *mut ffi::RedisModuleBlockedClient,
}

unsafe impl Send for BlockedClient {}

pub enum QueryResult {
    OK,
    DONE,
    Array { array: Vec<sql::Row> },
}

fn execute_query(db: &sql::RawConnection,
                 query: String)
                 -> Result<QueryResult, sql::SQLite3Error> {

    let stmt = sql::create_statement(&db, query.clone())?;
    let cursor = sql::execute_statement(stmt)?;
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


pub fn listen_and_execute(db: sql::RawConnection,
                          rx: Receiver<Command>) {

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

pub fn reply_with_ok(ctx: *mut ffi::RedisModuleCtx) -> i32 {
    reply_with_simple_string(ctx, String::from("OK"))
}

pub fn reply_with_done(ctx: *mut ffi::RedisModuleCtx) -> i32 {
    reply_with_simple_string(ctx, String::from("DONE"))
}

pub fn reply_with_array(ctx: *mut ffi::RedisModuleCtx,
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

pub struct DBKey {
    pub tx: Sender<Command>,
    pub db: sql::RawConnection,
    pub in_memory: bool,
}

pub fn create_metadata_table(db: &sql::RawConnection)
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

pub fn insert_metadata(db: &sql::RawConnection,
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

pub fn make_backup(conn1: &sql::RawConnection,
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

pub fn create_backup(conn: &sql::RawConnection,
                     path: String)
                     -> Result<i32, sql::SQLite3Error> {
    match sql::open_connection(path) {
        Err(e) => Err(e),
        Ok(new_db) => make_backup(conn, &new_db),
    }
}

pub fn write_file_to_rdb(f: File,
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
                println!("Number of bytes written: {}", n);
                ffi::RedisModule_SaveStringBuffer.unwrap()(rdb,
                                                           tw.as_slice().as_ptr() as *const i8,
                                                           n)

            },
            Err(e) => return Err(e),
        }
    }

}

// TODO make sure of the deallocation

struct SafeRedisModuleString {
    ptr: *mut std::os::raw::c_char,
}

impl Drop for SafeRedisModuleString {
    fn drop(&mut self) {
        unsafe { ffi::RedisModule_Free.unwrap()(self.ptr as *mut std::os::raw::c_void) }
    }
}

pub fn write_rdb_to_file(f: &mut File,
                         rdb: *mut ffi::RedisModuleIO)
                         -> Result<(), std::io::Error> {

    let blocks =
        unsafe { ffi::RedisModule_LoadSigned.unwrap()(rdb) as i64 };

    for _ in 0..blocks {
        let mut dimension: libc::size_t = 0;
        println!("About to load the string");
        let c_str_ptr = SafeRedisModuleString {
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


/*
 * Create a statement
 *
 * Input: A DB a Statement identifier and a Statement string
 *
 * Check that the DB exist
 * Compile the statement
 * Add the statement to an hash map Identifier => Compiled Statement
 *
 * /







