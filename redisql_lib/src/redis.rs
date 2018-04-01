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

use std::clone::Clone;

use std::sync::{Arc, Mutex, RwLock};
use std::sync::mpsc::{Receiver, RecvError, Sender};

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use self::fnv::FnvHashMap;

use std;
use std::fmt;
use std::error;

use redisql_error as err;
use redisql_error::RediSQLError;

use sqlite::StatementTrait;

use community_statement::MultiStatement;

#[allow(dead_code)]
#[allow(non_snake_case)]
#[allow(non_camel_case_types)]
#[allow(non_upper_case_globals)]
#[allow(improper_ctypes)]
pub mod ffi {
    include!(concat!(env!("OUT_DIR"), "/bindings_redis.rs"));
}

use sqlite as sql;

#[derive(Clone)]
pub struct ReplicationBook {
    data: Arc<RwLock<HashMap<String, (MultiStatement, bool)>>>,
    db: Arc<Mutex<sql::RawConnection>>,
}

trait ReplicationData {
    fn to_replicate(&self) -> bool;
}

pub trait StatementCache {
    fn new(&Arc<Mutex<sql::RawConnection>>) -> Self;
    fn is_statement_present(&self, String) -> bool;
    fn insert_new_statement(&mut self,
                            identifier: String,
                            statement: String)
                            -> Result<QueryResult, RediSQLError>;
    fn delete_statement(&mut self,
                        String)
                        -> Result<QueryResult, RediSQLError>;
    fn update_statement(&mut self,
                        identifier: String,
                        statement: String)
                        -> Result<QueryResult, RediSQLError>;
    fn exec_statement(&self,
                      String,
                      Vec<String>)
                      -> Result<QueryResult, RediSQLError>;
    fn query_statement(&self,
                       String,
                       Vec<String>)
                       -> Result<QueryResult, RediSQLError>;
    fn to_replicate(&self, String) -> bool;
}

impl StatementCache for ReplicationBook {
    fn new(db: &Arc<Mutex<sql::RawConnection>>) -> Self {
        ReplicationBook {
            data: Arc::new(RwLock::new(HashMap::new())),
            db: Arc::clone(db),
        }
    }

    fn is_statement_present(&self, identifier: String) -> bool {
        self.data.read().unwrap().contains_key(&identifier)
    }

    fn insert_new_statement(&mut self,
                            identifier: String,
                            statement: String)
                            -> Result<QueryResult, RediSQLError> {

        let db = self.db.clone();
        let mut map = self.data.write().unwrap();
        match map.entry(identifier.clone()) {
            Entry::Vacant(v) => {
                let stmt =
                    create_statement(db, identifier, statement)?;
                let read_only = stmt.is_read_only();
                v.insert((stmt, read_only));
                Ok(QueryResult::OK { to_replicate: true })
            }
            Entry::Occupied(_) => {
                let debug = String::from("Statement already present");
                let description = String::from("The statement is already present in the database, try with UPDATE_STATEMENT",);
                Err(RediSQLError::new(debug, description))
            }
        }
    }

    fn delete_statement(&mut self,
                        identifier: String)
                        -> Result<QueryResult, RediSQLError> {
        let db = self.db.clone();
        let mut map = self.data.write().unwrap();
        match map.entry(identifier.clone()) {
            Entry::Vacant(_) => {
                let debug = String::from("Statement not present.");
                let description = String::from("The statement is not present in the database, impossible to delete it.",);
                Err(RediSQLError::new(debug, description))
            }
            Entry::Occupied(o) => {
                remove_statement(&db, identifier)?;
                o.remove_entry();
                Ok(QueryResult::OK { to_replicate: true })
            }
        }
    }

    fn update_statement(&mut self,
                        identifier: String,
                        statement: String)
                        -> Result<QueryResult, RediSQLError> {
        let db = self.db.clone();
        let mut map = self.data.write().unwrap();
        match map.entry(identifier.clone()) {
            Entry::Vacant(_) => {
                let debug = String::from("Statement not present.");
                let description = String::from("The statement is not present in the database, impossible to update it.",);
                Err(RediSQLError::new(debug, description))
            }
            Entry::Occupied(mut o) => {
                let stmt =
                    update_statement(&db, identifier, statement)?;
                let read_only = stmt.is_read_only();
                o.insert((stmt, read_only));
                Ok(QueryResult::OK { to_replicate: true })
            }
        }
    }

    fn exec_statement(&self,
                      identifier: String,
                      args: Vec<String>)
                      -> Result<QueryResult, RediSQLError> {
        let map = self.data.read().unwrap();
        match map.get(&identifier) {
            None => {
                let debug = String::from("No statement found");
                let description = String::from("The statement is not present in the database",);
                Err(RediSQLError::new(debug, description))
            }
            Some(&(ref stmt, _)) => {
                stmt.reset();
                let stmt = bind_statement(&stmt, args)?;
                let cursor = stmt.execute()?;
                Ok(cursor_to_query_result(cursor))
            }
        }
    }

    fn query_statement(&self,
                       identifier: String,
                       args: Vec<String>)
                       -> Result<QueryResult, RediSQLError> {
        let map = self.data.read().unwrap();
        match map.get(&identifier) {
            None => {
                let debug = String::from("No statement found");
                let description = String::from("The statement is not present in the database",);
                Err(RediSQLError::new(debug, description))
            }
            Some(&(ref stmt, true)) => {
                stmt.reset();
                let stmt = bind_statement(&stmt, args)?;
                let cursor = stmt.execute()?;
                Ok(cursor_to_query_result(cursor))
            }
            Some(&(_, false)) => {
                let debug = String::from("Not read only statement");
                let description = String::from("Statement is not read only but it may modify the database, use `EXEC_STATEMENT` instead.",);
                Err(RediSQLError::new(debug, description))
            }
        }
    }

    fn to_replicate(&self, identifier: String) -> bool {
        false
    }
}

impl ReplicationData for ReplicationBook {
    fn to_replicate(&self) -> bool {
        false
    }
}

#[derive(Clone)]
pub struct Loop {
    db: Arc<Mutex<sql::RawConnection>>,
    replication_book: ReplicationBook,
}

pub trait LoopData {
    fn get_replication_book(&self) -> ReplicationBook;
    fn get_db(&self) -> Arc<Mutex<sql::RawConnection>>;
}

impl LoopData for Loop {
    fn get_replication_book(&self) -> ReplicationBook {
        self.replication_book.clone()
    }
    fn get_db(&self) -> Arc<Mutex<sql::RawConnection>> {
        Arc::clone(&self.db)
    }
}

impl Loop {
    fn new(db: sql::RawConnection) -> Self {
        let db = Arc::new(Mutex::new(db));
        let replication_book = ReplicationBook::new(&db);
        Loop {
            db,
            replication_book,
        }
    }
    fn new_from_arc(db: Arc<Mutex<sql::RawConnection>>) -> Loop {
        let replication_book = ReplicationBook::new(&db);
        Loop {
            db,
            replication_book,
        }
    }
}

pub struct RMString {
    pub ptr: *mut ffi::RedisModuleString,
    ctx: *mut ffi::RedisModuleCtx,
}

impl RMString {
    pub fn new(ctx: *mut ffi::RedisModuleCtx, s: String) -> RMString {
        let l = s.len();
        let cs = CString::new(s).unwrap();
        let ptr = unsafe {
            ffi::RedisModule_CreateString.unwrap()(ctx,
                                                   cs.as_ptr(),
                                                   l)
        };
        RMString { ptr, ctx }
    }
}

impl Drop for RMString {
    fn drop(&mut self) {
        unsafe {
            ffi::RedisModule_FreeString.unwrap()(self.ctx, self.ptr);
        }
    }
}
/*
pub fn create_rm_string(ctx: *mut ffi::RedisModuleCtx,
                        s: String)
                        -> *mut ffi::RedisModuleString {
    let l = s.len();
    let cs = CString::new(s).unwrap();

    unsafe {
        ffi::RedisModule_CreateString.unwrap()(ctx, cs.as_ptr(), l)
    }
}
*/


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

fn reply_with_simple_string(ctx: *mut ffi::RedisModuleCtx,
                            s: String)
                            -> i32 {
    let s = CString::new(s).unwrap();
    unsafe {
        ffi::RedisModule_ReplyWithSimpleString.unwrap()(ctx,
                                                        s.as_ptr())
    }
}

fn reply_with_ok(ctx: *mut ffi::RedisModuleCtx) -> i32 {
    reply_with_simple_string(ctx, String::from("OK"))
}

fn reply_with_done(ctx: *mut ffi::RedisModuleCtx,
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

fn reply_with_array(ctx: *mut ffi::RedisModuleCtx,
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

impl RedisReply for RediSQLError {
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
    Query {
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
    QueryStatement {
        identifier: String,
        arguments: Vec<String>,
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

pub fn do_execute(db: &Arc<Mutex<sql::RawConnection>>,
                  query: String)
                  -> Result<QueryResult, err::RediSQLError> {

    let stmt = MultiStatement::new(db.clone(), query)?;
    let cursor = stmt.execute()?;
    Ok(cursor_to_query_result(cursor))
}


pub fn do_query(db: &Arc<Mutex<sql::RawConnection>>,
                query: String)
                -> Result<QueryResult, err::RediSQLError> {

    let stmt = MultiStatement::new(db.clone(), query)?;
    match stmt.is_read_only() {
        true => {
            let cursor = stmt.execute()?;
            Ok(cursor_to_query_result(cursor))
        }
        false => {
            let debug = String::from("Not read only statement");
            let description = String::from("Statement is not read only but it may modify the database, use `EXEC_STATEMENT` instead.",);
            Err(RediSQLError::new(debug, description))
        }
    }
}

fn bind_statement<'a>
    (stmt: &'a MultiStatement,
     arguments: Vec<String>)
     -> Result<&'a MultiStatement, sql::SQLite3Error> {

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
fn restore_previous_statements<'a, L: LoopData + Clone>(loopdata: L)
                                                        -> () {
    let saved_statements = get_statement_metadata(loopdata.get_db());
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
                match compile_and_insert_statement(identifier,
                                                   statement,
                                                   loopdata.clone()) {
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

pub fn listen_and_execute<L: LoopData + Clone>(loopdata: L,
rx: Receiver<Command>){
    debug!("Start thread execution");
    restore_previous_statements(loopdata.clone());
    loop {
        debug!("Loop iteration");
        match rx.recv() {
            Ok(Command::Exec { query, client }) => {
                debug!("Exec | Query = {:?}", query);
                let result = do_execute(&loopdata.get_db(), query);
                return_value(client, result);
            }
            Ok(Command::Query { query, client }) => {
                debug!("Query | Query = {:?}", query);
                let result = do_query(&loopdata.get_db(), query);
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
                let result =
                    loopdata
                        .get_replication_book()
                        .update_statement(identifier, statement);
                return_value(client, result)
            }
            Ok(Command::DeleteStatement { identifier, client }) => {
                debug!("DeleteStatement | Identifier = {:?}",
                       identifier);
                let result = loopdata
                    .get_replication_book()
                    .delete_statement(identifier);

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
                let result =
                    loopdata
                        .get_replication_book()
                        .insert_new_statement(identifier, statement);
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
                let result =
                    loopdata
                        .get_replication_book()
                        .exec_statement(identifier, arguments);
                return_value(client, result);
            }
            Ok(Command::QueryStatement {
                   identifier,
                   arguments,
                   client,
               }) => {
                let result =
                    loopdata
                        .get_replication_book()
                        .query_statement(identifier, arguments);
                return_value(client, result);
            }
            Ok(Command::Stop) => return,
            Err(RecvError) => return,
        }
    }
}


fn compile_and_insert_statement<'a, L: LoopData + Clone>
    (identifier: String,
     statement: String,
     loop_data: L)
     -> Result<QueryResult, err::RediSQLError> {
    let stmt_cache = loop_data.get_replication_book().data;
    let mut statements_cache = stmt_cache.write().unwrap();
    match statements_cache.entry(identifier.clone()) {
        Entry::Vacant(v) => {
            let db = loop_data.get_db();
            match create_statement(db,
                                   identifier.clone(),
                                   statement) {
                Ok(stmt) => {
                    v.insert((stmt, false));
                    Ok(QueryResult::OK { to_replicate: true })
                }
                Err(e) => Err(e),
            }
        }
        Entry::Occupied(_) => {
            let err = RedisError {
                msg: String::from("Statement already exists, \
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

    match MultiStatement::new(db, statement) {
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

    let stmt = MultiStatement::new(db, statement)?;
    stmt.bind_index(1, &data_type)?;
    stmt.bind_index(2, &key)?;
    stmt.bind_index(3, &value)?;
    stmt.execute()?;
    Ok(())
}

pub fn enable_foreign_key(db: Arc<Mutex<sql::RawConnection>>)
                          -> Result<(), sql::SQLite3Error> {
    let enable_foreign_key = String::from("PRAGMA foreign_keys = ON;",);
    match MultiStatement::new(db, enable_foreign_key) {
        Err(e) => Err(e),
        Ok(stmt) => {
            match stmt.execute() {
                Err(e) => Err(e),
                Ok(_) => Ok(()),
            }
        }
    }
}

fn update_statement_metadata(db: Arc<Mutex<sql::RawConnection>>,
                             key: String,
                             value: String)
                             -> Result<(), sql::SQLite3Error> {
    let statement = String::from("UPDATE RediSQLMetadata SET value \
                                  = ?1 WHERE data_type = \
                                  'statement' AND key = ?2");

    let stmt = MultiStatement::new(db, statement)?;
    stmt.bind_index(1, &value)?;
    stmt.bind_index(2, &key)?;
    stmt.execute()?;
    Ok(())
}

fn remove_statement_metadata(db: Arc<Mutex<sql::RawConnection>>,
                             key: String)
                             -> Result<(), sql::SQLite3Error> {
    let statement = String::from("DELETE FROM RediSQLMetadata \
                                  WHERE data_type = 'statement' \
                                  AND key = ?1");

    let stmt = MultiStatement::new(db, statement)?;
    stmt.bind_index(1, &key)?;
    stmt.execute()?;
    Ok(())
}

fn get_statement_metadata
    (db: Arc<Mutex<sql::RawConnection>>)
     -> Result<QueryResult, sql::SQLite3Error> {

    let statement = String::from("SELECT * FROM RediSQLMetadata \
                                  WHERE data_type = 'statement';");

    let stmt = MultiStatement::new(db, statement)?;
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
        mem::forget(buffer);
        match y {
            Err(e) => return Err(e),
            _ => (),
        }
    }
    Ok(())
}

pub fn get_dbkey_from_name(ctx: *mut ffi::RedisModuleCtx,
                           name: String)
                           -> Result<Box<DBKey>, i32> {
    let key_name = RMString::new(ctx, name);
    let key = unsafe {
        ffi::Export_RedisModule_OpenKey(
            ctx,
            key_name.ptr,
            ffi::REDISMODULE_WRITE,
        )
    };
    let safe_key = RedisKey { key: key };
    let key_type =
        unsafe { ffi::RedisModule_KeyType.unwrap()(safe_key.key) };
    if unsafe {
           ffi::DBType ==
           ffi::RedisModule_ModuleTypeGetType.unwrap()(safe_key.key)
       } {
        let db_ptr = unsafe {
            ffi::RedisModule_ModuleTypeGetValue
                .unwrap()(safe_key.key) as *mut DBKey
        };
        let db: Box<DBKey> = unsafe { Box::from_raw(db_ptr) };
        Ok(db)
    } else {
        Err(key_type)
    }
}

pub fn get_db_channel_from_name(ctx: *mut ffi::RedisModuleCtx,
                                name: String)
                                -> Result<Sender<Command>, i32> {
    let db: Box<DBKey> = get_dbkey_from_name(ctx, name)?;
    let channel = db.tx.clone();
    std::mem::forget(db);
    Ok(channel)
}

pub fn reply_with_error_from_key_type(ctx: *mut ffi::RedisModuleCtx,
                                      key_type: i32)
                                      -> i32 {
    match key_type {
        ffi::REDISMODULE_KEYTYPE_EMPTY => {
            let error = CString::new("ERR - Error the key is empty")
                .unwrap();
            unsafe {
                ffi::RedisModule_ReplyWithError
                    .unwrap()(ctx, error.as_ptr())
            }
        }
        _ => {
            let error = CStr::from_bytes_with_nul(
                ffi::REDISMODULE_ERRORMSG_WRONGTYPE,
            ).unwrap();
            unsafe {
                ffi::RedisModule_ReplyWithError
                    .unwrap()(ctx, error.as_ptr())
            }
        }
    }
}

fn create_statement(db: Arc<Mutex<sql::RawConnection>>,
                    identifier: String,
                    statement: String)
                    -> Result<MultiStatement, err::RediSQLError> {

    let stmt = MultiStatement::new(Arc::clone(&db),
                                   statement.clone())?;
    insert_metadata(db,
                    String::from("statement"),
                    identifier.clone(),
                    statement)?;
    Ok(stmt)
}

fn update_statement(db: &Arc<Mutex<sql::RawConnection>>,
                    identifier: String,
                    statement: String)
                    -> Result<MultiStatement, err::RediSQLError> {

    let stmt = MultiStatement::new(Arc::clone(db),
                                   statement.clone())?;
    update_statement_metadata(Arc::clone(db),
                              identifier.clone(),
                              statement)?;
    Ok(stmt)
}

fn remove_statement(db: &Arc<Mutex<sql::RawConnection>>,
                    identifier: String)
                    -> Result<(), err::RediSQLError> {
    remove_statement_metadata(Arc::clone(db), identifier.clone())
        .or_else(|e| Err(err::RediSQLError::from(e)))
}

pub fn replicate_verbatim(ctx: *mut ffi::RedisModuleCtx) {
    unsafe { ffi::RedisModule_ReplicateVerbatim.unwrap()(ctx) };
}

pub fn replicate(_ctx: *mut ffi::RedisModuleCtx,
                 _command: String,
                 _argv: *mut *mut ffi::RedisModuleString,
                 _argc: std::os::raw::c_int) {
}

pub fn register_function(
    ctx: *mut ffi::RedisModuleCtx,
    name: String,
    flags: String,
    f: extern "C" fn(*mut ffi::RedisModuleCtx,
                     *mut *mut ffi::RedisModuleString,
                     ::std::os::raw::c_int)
                     -> i32,
) -> Result<(), i32>{

    let create_db: ffi::RedisModuleCmdFunc = Some(f);

    let command_c_name = CString::new(name).unwrap();
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
        return Err(ffi::REDISMODULE_ERR);
    }
    Ok(())
}

pub fn register_write_function(
    ctx: *mut ffi::RedisModuleCtx,
    name: String,
    f: extern "C" fn(*mut ffi::RedisModuleCtx,
                     *mut *mut ffi::RedisModuleString,
                     ::std::os::raw::c_int)
                     -> i32,
) -> Result<(), i32>{
    register_function(ctx, name, String::from("write"), f)
}
