extern crate libc;
extern crate uuid;
extern crate fnv;

use std::mem;
use std::ffi::{CString, CStr};
use std::string;
use std::fs::File;
use std::io::BufReader;

use std::os::raw::c_char;
use std::os::raw::c_long;

use std::io::{Read, Write};

use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Receiver, RecvError, Sender};

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use self::fnv::FnvHashMap;

use std;
use std::fmt;
use std::error;

use redisql_error as err;

use Loop;
use LoopData;

use sqlite::StatementTrait;

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

impl Drop for Context {
    fn drop(&mut self) {
        mem::forget(self.ctx);
    }
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
                                                                int as
                                                                i64)
                }
                sql::Entity::Float { float } => {
                    ffi::RedisModule_ReplyWithDouble.unwrap()(ctx,
                                                              float)
                }
                sql::Entity::Text { ref text } => {
                    let text_c = CString::new(text.clone()).unwrap();
                    ffi::RedisModule_ReplyWithStringBuffer
                        .unwrap()(ctx, text_c.as_ptr(), text.len())
                }
                sql::Entity::Blob { ref blob } => {
                    let blob_c = CString::new(blob.clone()).unwrap();
                    ffi::RedisModule_ReplyWithStringBuffer
                        .unwrap()(ctx, blob_c.as_ptr(), blob.len())
                }
                sql::Entity::Null => {
                    ffi::RedisModule_ReplyWithNull.unwrap()(ctx)
                }
                sql::Entity::OK { to_replicate } => {
                    QueryResult::OK { to_replicate: to_replicate }
                        .reply(ctx)
                }                
                sql::Entity::DONE {
                    modified_rows,
                    to_replicate,
                } => {
                    QueryResult::DONE {
                            modified_rows: modified_rows,
                            to_replicate: to_replicate,
                        }
                        .reply(ctx)
                }
            }
        }
    }
}

fn reply_with_string(ctx: *mut ffi::RedisModuleCtx,
                     s: String)
                     -> i32 {
    let len = s.len();
    let s = CString::new(s).unwrap();
    unsafe {
        ffi::RedisModule_ReplyWithStringBuffer.unwrap()(ctx,
                                                        s.as_ptr(),
                                                        len)
    }
}

pub fn reply_with_simple_string(ctx: *mut ffi::RedisModuleCtx,
                                s: String)
                                -> i32 {
    let s = CString::new(s).unwrap();
    unsafe {
        ffi::RedisModule_ReplyWithSimpleString.unwrap()(ctx,
                                                        s.as_ptr())
    }
}

pub fn reply_with_ok(ctx: *mut ffi::RedisModuleCtx) -> i32 {
    reply_with_simple_string(ctx, String::from("OK"))
}

pub fn reply_with_done(ctx: *mut ffi::RedisModuleCtx,
                       modified_rows: i32)
                       -> i32 {
    unsafe {
        ffi::RedisModule_ReplyWithArray.unwrap()(ctx, 2);
    }
    reply_with_simple_string(ctx, String::from("DONE"));
    unsafe {
        ffi::RedisModule_ReplyWithLongLong.unwrap()(ctx,
                                                    modified_rows as
                                                    i64);
    }
    ffi::REDISMODULE_OK
}

pub fn reply_with_array(ctx: *mut ffi::RedisModuleCtx,
                        array: Vec<sql::Row>)
                        -> i32 {
    let len = array.len() as c_long;
    unsafe {
        ffi::RedisModule_ReplyWithArray.unwrap()(ctx, len);
    }
    for row in array {
        unsafe {
            ffi::RedisModule_ReplyWithArray.unwrap()(ctx,
                                                     row.len() as
                                                     c_long);
        }
        for entity in row {
            entity.reply(ctx);
        }
    }
    ffi::REDISMODULE_OK
}


impl RedisReply for sql::SQLite3Error {
    fn reply(&self, ctx: *mut ffi::RedisModuleCtx) -> i32 {
        let error = format!("{}", self);
        reply_with_error(ctx, error)
    }
}

fn reply_with_error(ctx: *mut ffi::RedisModuleCtx, s: String) -> i32 {
    let s = CString::new(s).unwrap();
    unsafe {
        ffi::RedisModule_ReplyWithError.unwrap()(ctx, s.as_ptr())
    }
}

fn parse_args(argv: *mut *mut ffi::RedisModuleString,
              argc: i32)
              -> Result<Vec<String>, string::FromUtf8Error> {
    mem::forget(argv);
    mem::forget(argc);
    let mut args: Vec<String> = Vec::with_capacity(argc as usize);
    for i in 0..argc {
        let redis_str = unsafe { *argv.offset(i as isize) };
        let mut arg = string_ptr_len(redis_str);
        arg = arg.replace("\\", "");
        args.push(arg);
    }
    Ok(args)
}

pub fn create_argument(ctx: *mut ffi::RedisModuleCtx,
                       argv: *mut *mut ffi::RedisModuleString,
                       argc: i32)
                       -> (Context, Vec<String>) {
    mem::forget(argv);
    mem::forget(argc);
    mem::forget(ctx);
    let context = Context { ctx: ctx };
    let argvector = parse_args(argv, argc).unwrap();
    (context, argvector)
}

pub fn create_rm_string(ctx: *mut ffi::RedisModuleCtx,
                        s: String)
                        -> *mut ffi::RedisModuleString {
    let l = s.len();
    let cs = CString::new(s).unwrap();

    unsafe {
        ffi::RedisModule_CreateString.unwrap()(ctx, cs.as_ptr(), l)
    }
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
    CompileStatement {
        identifier: String,
        statement: String,
        client: BlockedClient,
    },
    ExecStatement {
        identifier: String,
        arguments: Vec<String>,
        client: BlockedClient,
    },
    UpdateStatement {
        identifier: String,
        statement: String,
        client: BlockedClient,
    },
    DeleteStatement {
        identifier: String,
        client: BlockedClient,
    },
}

pub struct BlockedClient {
    pub client: *mut ffi::RedisModuleBlockedClient,
}

unsafe impl Send for BlockedClient {}

pub enum QueryResult {
    OK { to_replicate: bool },
    DONE {
        modified_rows: i32,
        to_replicate: bool,
    },
    Array {
        array: Vec<sql::Row>,
        to_replicate: bool,
    },
}

#[cfg(feature = "community")]
impl QueryResult {
    pub fn reply(self, ctx: *mut ffi::RedisModuleCtx) -> i32 {
        match self {
            QueryResult::OK { .. } => reply_with_ok(ctx),
            QueryResult::DONE { modified_rows, .. } => {
                reply_with_done(ctx, modified_rows)
            }
            QueryResult::Array { array, .. } => {
                reply_with_array(ctx, array)
            }
        }
    }
    pub fn to_replicate(&self) -> bool {
        false
    }
}

fn cursor_to_query_result(cursor: sql::Cursor) -> QueryResult {
    match cursor {
        sql::Cursor::OKCursor { to_replicate } => {
            QueryResult::OK { to_replicate }
        }
        sql::Cursor::DONECursor {
            modified_rows,
            to_replicate,
        } => {
            QueryResult::DONE {
                modified_rows,
                to_replicate,
            }
        }
        sql::Cursor::RowsCursor { to_replicate, .. } => {
            let y = QueryResult::Array {
                array: cursor.collect::<Vec<sql::Row>>(),
                to_replicate: to_replicate,
            };
            return y;
        }
    }
}

fn execute_query(db: &Arc<Mutex<sql::RawConnection>>,
                 query: String)
                 -> Result<QueryResult, err::RediSQLError> {

    let stmt = sql::MultiStatement::new(db.clone(), query)?;
    let cursor = stmt.execute()?;
    Ok(cursor_to_query_result(cursor))
}

fn bind_statement<'a>
    (stmt: &'a sql::MultiStatement,
     arguments: Vec<String>)
     -> Result<&'a sql::MultiStatement, sql::SQLite3Error> {

    match stmt.bind_texts(arguments) {
        Err(e) => Err(e),
        Ok(_) => Ok(stmt),
    }
}

pub struct RedisError {
    pub msg: String,
}

impl fmt::Display for RedisError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ERR - {}", self.msg)
    }
}

impl fmt::Debug for RedisError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl error::Error for RedisError {
    fn description(&self) -> &str {
        self.msg.as_str()
    }
}

fn restore_previous_statements<'a>(loopdata: Loop, mut statements_cache: &mut HashMap<String, sql::MultiStatement, std::hash::BuildHasherDefault<fnv::FnvHasher
                               >> )
-> (){
    let saved_statements =
        get_statement_metadata(loopdata.db.clone());
    match saved_statements {
        Ok(QueryResult::Array { array, .. }) => {
            for row in array {
                let identifier = match row[1] {
                    sql::Entity::Text { ref text } => text.clone(),
                    _ => continue,
                };
                let statement = match row[2] {
                    sql::Entity::Text { ref text } => text.clone(),
                    _ => continue,
                };
                match compile_and_insert_statement(
                    identifier,
                    statement,
                    loopdata.clone(),
                    &mut statements_cache,
                ) {
                    Err(e) => println!("Error: {}", e),
                    _ => (),
                }
            }
        }
        Err(e) => println!("Error: {}", e),
        _ => (),
    }
}

fn return_value(client: BlockedClient,
                result: Result<QueryResult, err::RediSQLError>) {
    unsafe {
        ffi::RedisModule_UnblockClient
            .unwrap()(client.client,
                      Box::into_raw(Box::new(result)) as
                      *mut std::os::raw::c_void);

    }
}

pub fn listen_and_execute(loopdata: Loop, rx: Receiver<Command>) {
    println!("Start thread execution");
    let mut statements_cache = FnvHashMap::default();
    restore_previous_statements(loopdata.clone(),
                                &mut statements_cache);
    loop {
        println!("Loop iteration");
        match rx.recv() {
            Ok(Command::Exec { query, client }) => {
                debug!("Exec | Query = {:?}", query);
                let result = execute_query(&loopdata.db, query);
                return_value(client, result);
            }
            Ok(Command::UpdateStatement {
                   identifier,
                   statement,
                   client,
               }) => {
                debug!("UpdateStatement | Identifier = {:?} Statement = {:?}",
                       identifier,
                       statement);
                let result = match statements_cache
                          .entry(identifier.clone()) {
                    Entry::Vacant(_) => {
                        let err = RedisError {
                            msg: String::from("Statement does not \
                                                   exists yet, \
                                                   impossible to \
                                                   update."),
                        };
                        Err(err::RediSQLError::from(err))
                    }
                    Entry::Occupied(mut o) => {
                        match update_statement(&loopdata.db,
                                               identifier.clone(),
                                               statement) {
                            Ok(stmt) => {
                                o.insert(stmt);
                                Ok(QueryResult::OK {
                                       to_replicate: true,
                                   })
                            }
                            Err(e) => Err(e),
                        }
                    }
                };

                return_value(client, result)
            }
            Ok(Command::DeleteStatement { identifier, client }) => {
                debug!("DeleteStatement | Identifier = {:?}",
                       identifier);
                let result = match statements_cache
                          .entry(identifier.clone()) {
                    Entry::Vacant(_) => {
                        let err = RedisError {
                            msg: String::from("Statement does not \
                                                   exists yet, \
                                                   impossible to \
                                                   delete."),
                        };
                        Err(err::RediSQLError::from(err))
                    }
                    Entry::Occupied(o) => {
                        match remove_statement(&loopdata.db,
                                               identifier) {
                            Ok(()) => {
                                o.remove_entry();
                                Ok(QueryResult::OK {to_replicate: true})
                            }
                            Err(e) => Err(err::RediSQLError::from(e)),
                        }
                    }
                };
                return_value(client, result);
            }
            Ok(Command::CompileStatement {
                   identifier,
                   statement,
                   client,
               }) => {
                debug!("CompileStatement | Identifier = {:?} Statement = {:?}",
                       identifier,
                       statement);
                let result = compile_and_insert_statement(
                    identifier,
                    statement,
                    loopdata.clone(),
                    &mut statements_cache,
                );
                return_value(client, result);
            }

            Ok(Command::ExecStatement {
                   identifier,
                   arguments,
                   client,
               }) => {
                debug!("ExecStatement | Identifier = {:?} Arguments = {:?}",
                       identifier,
                       arguments);
                let result = match statements_cache
                          .get(&identifier) {
                    None => {
                        let debug = String::from("No statement \
                                                      found");
                        let description = String::from("The statement is not \
                                              present in the \
                                              database");
                        Err(err::RediSQLError::new(debug,
                                                   description))
                    }
                    Some(stmt) => {
                        stmt.reset();
                        let bind = bind_statement(stmt, arguments);
                        match bind {
                            Ok(stmt) => {
                                let cursor = stmt.execute();
                                match cursor {
                                    Err(e) => Err(
                                        err::RediSQLError::from(e),
                                    ),
                                    Ok(cursor) => {
                                        Ok(
                                            cursor_to_query_result(cursor),
                                        )
                                    }
                                }
                            }
                            Err(e) => Err(err::RediSQLError::from(e)),
                        }
                    }
                };
                return_value(client, result);
            }
            Ok(Command::Stop) => return,
            Err(RecvError) => return,
        }
    }
}


fn compile_and_insert_statement<'a>(identifier: String,
                                statement: String,
                                loop_data: Loop,
                                statements_cache: &mut HashMap<String, sql::MultiStatement, std::hash::BuildHasherDefault<fnv::FnvHasher>>)
-> Result<QueryResult, err::RediSQLError>{
    match statements_cache.entry(identifier.clone()) {
        Entry::Vacant(v) => {
            let db = loop_data.db;
            match create_statement(db,
                                   identifier.clone(),
                                   statement) {
                Ok(stmt) => {
                    v.insert(stmt);
                    Ok(QueryResult::OK { to_replicate: true })
                }
                Err(e) => Err(e),
            }
        }
        Entry::Occupied(_) => {
            let err = RedisError {
                msg: String::from("Statement already existsm, \
                                   impossible to overwrite it with \
                                   this command, try with \
                                   UPDATE_STATEMENT"),
            };
            Err(err::RediSQLError::from(err))
        }
    }
}

pub struct DBKey {
    pub tx: Sender<Command>,
    pub in_memory: bool,
    pub loop_data: Loop,
}

impl DBKey {
    pub fn new(tx: Sender<Command>,
               db: sql::RawConnection,
               in_memory: bool)
               -> DBKey {
        let loop_data = Loop::new(db);
        DBKey {
            tx,
            in_memory,
            loop_data,
        }
    }
    pub fn new_from_arc(tx: Sender<Command>,
                        db: Arc<Mutex<sql::RawConnection>>,
                        in_memory: bool)
                        -> DBKey {
        let loop_data = Loop::new_from_arc(db);
        DBKey {
            tx,
            in_memory,
            loop_data,
        }
    }
}

pub fn create_metadata_table(db: Arc<Mutex<sql::RawConnection>>)
                             -> Result<(), sql::SQLite3Error> {
    let statement = String::from("CREATE TABLE \
                                  RediSQLMetadata(data_type TEXT, \
                                  key TEXT, value TEXT);");

    match sql::MultiStatement::new(db, statement) {
        Err(e) => Err(e),
        Ok(stmt) => {
            match stmt.execute() {
                Ok(_) => Ok(()),
                Err(e) => Err(e),
            }
        }
    }
}

pub fn insert_metadata(db: Arc<Mutex<sql::RawConnection>>,
                       data_type: String,
                       key: String,
                       value: String)
                       -> Result<(), sql::SQLite3Error> {
    let statement = String::from("INSERT INTO RediSQLMetadata \
                                  VALUES(?1, ?2, ?3);");

    match sql::MultiStatement::new(db, statement) {
        Err(e) => Err(e),
        Ok(stmt) => {
            match stmt.bind_index(1, &data_type) {
                Err(e) => Err(e),
                Ok(sql::SQLiteOK::OK) => {
                    match stmt.bind_index(2, &key) {
                        Err(e) => Err(e),
                        Ok(sql::SQLiteOK::OK) => {
                            match stmt.bind_index(3, &value) {
                                Err(e) => Err(e),
                                Ok(sql::SQLiteOK::OK) => {
                                    match stmt.execute() {
                                        Ok(_) => Ok(()),
                                        Err(e) => Err(e),
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

pub fn enable_foreign_key(db: Arc<Mutex<sql::RawConnection>>)
                          -> Result<(), sql::SQLite3Error> {
    let enable_foreign_key = String::from("PRAGMA foreign_keys = ON;",);
    match sql::MultiStatement::new(db, enable_foreign_key) {
        Err(e) => Err(e),
        Ok(stmt) => {
            match stmt.execute() {
                Err(e) => Err(e),
                Ok(_) => Ok(()),
            }
        }
    }
}

pub fn update_statement_metadata(db: Arc<Mutex<sql::RawConnection>>,
                                 key: String,
                                 value: String)
                                 -> Result<(), sql::SQLite3Error> {
    let statement = String::from("UPDATE RediSQLMetadata SET value \
                                  = ?1 WHERE data_type = \
                                  'statement' AND key = ?2");

    let stmt = sql::MultiStatement::new(db, statement)?;
    stmt.bind_index(1, &value)?;
    stmt.bind_index(2, &key)?;
    stmt.execute()?;
    Ok(())
}

pub fn remove_statement_metadata(db: Arc<Mutex<sql::RawConnection>>,
                                 key: String)
                                 -> Result<(), sql::SQLite3Error> {
    let statement = String::from("DELETE FROM RediSQLMetadata \
                                  WHERE data_type = 'statement' \
                                  AND key = ?1");

    let stmt = sql::MultiStatement::new(db, statement)?;
    stmt.bind_index(1, &key)?;
    stmt.execute()?;
    Ok(())
}

pub fn get_statement_metadata
    (db: Arc<Mutex<sql::RawConnection>>)
     -> Result<QueryResult, sql::SQLite3Error> {

    let statement = String::from("SELECT * FROM RediSQLMetadata \
                                  WHERE data_type = 'statement';");

    let stmt = sql::MultiStatement::new(db, statement)?;
    let cursor = stmt.execute()?;
    Ok(cursor_to_query_result(cursor))
}

pub fn string_ptr_len(str: *mut ffi::RedisModuleString) -> String {
    unsafe {
        CStr::from_ptr(ffi::RedisModule_StringPtrLen
                           .unwrap()(str, std::ptr::null_mut()))
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
                ffi::RedisModule_SaveStringBuffer
                    .unwrap()(rdb,
                              tw.as_slice().as_ptr() as *const c_char,
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
        unsafe {
            ffi::RedisModule_Free.unwrap()(self.ptr as
                                           *mut std::os::raw::c_void)
        }
    }
}

pub fn write_rdb_to_file(f: &mut File,
                         rdb: *mut ffi::RedisModuleIO)
                         -> Result<(), std::io::Error> {

    let blocks =
        unsafe { ffi::RedisModule_LoadSigned.unwrap()(rdb) as i64 };

    for _ in 0..blocks {
        let mut dimension: libc::size_t = 0;
        let c_str_ptr = SafeRedisModuleString {
            ptr: unsafe {
                ffi::RedisModule_LoadStringBuffer
                    .unwrap()(rdb, &mut dimension)
            },
        };

        if dimension == 0 {
            break;
        }
        let buffer: Vec<u8> = unsafe {
            Vec::from_raw_parts(c_str_ptr.ptr as *mut u8,
                                dimension,
                                dimension)
        };
        let y = f.write_all(buffer.as_slice());
        ::mem::forget(buffer);
        match y {
            Err(e) => return Err(e),
            _ => (),
        }
    }
    Ok(())
}

fn create_statement
    (db: Arc<Mutex<sql::RawConnection>>,
     identifier: String,
     statement: String)
     -> Result<sql::MultiStatement, err::RediSQLError> {

    let stmt = sql::MultiStatement::new(Arc::clone(&db),
                                        statement.clone())?;
    insert_metadata(db,
                    String::from("statement"),
                    identifier.clone(),
                    statement)?;
    Ok(stmt)
}

fn update_statement
    (db: &Arc<Mutex<sql::RawConnection>>,
     identifier: String,
     statement: String)
     -> Result<sql::MultiStatement, err::RediSQLError> {

    let stmt = sql::MultiStatement::new(Arc::clone(db),
                                        statement.clone())?;
    update_statement_metadata(Arc::clone(db),
                              identifier.clone(),
                              statement)?;
    Ok(stmt)
}

fn remove_statement(db: &Arc<Mutex<sql::RawConnection>>,
                    identifier: String)
                    -> Result<(), err::RediSQLError> {
    remove_statement_metadata(Arc::clone(db), identifier.clone())?;
    Ok(())
}
