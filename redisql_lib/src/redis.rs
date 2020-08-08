use fnv::FnvHashMap;
use std::clone::Clone;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::ffi::{CStr, CString};
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::os::raw::{c_char, c_long};
use std::slice;
use std::str;
use std::sync::mpsc::channel;
use std::sync::mpsc::{Receiver, RecvError, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;

pub use crate::redis_type as rm;
use crate::redis_type::{
    BlockedClient, Context, OpenKey, ReplyWithError,
};

use crate::redisql_error as err;
use crate::redisql_error::RediSQLError;

use crate::sqlite::{
    Connection, Cursor, Entity, QueryResult, SQLite3Error,
    StatementTrait,
};

use crate::community_statement::MultiStatement;

use crate::sqlite as sql;

use crate::statistics::STATISTICS;

#[derive(Clone)]
pub struct ReplicationBook {
    data: Arc<RwLock<FnvHashMap<String, (MultiStatement, bool)>>>,
    db: ConcurrentConnection,
}

impl ReplicationBook {
    fn clone_replication_book(
        &self,
        db: &ConcurrentConnection,
    ) -> Self {
        let mut new = ReplicationBook::new(db);
        let data = self.data.read().unwrap();
        for (name, (statement, _)) in data.iter() {
            // this could fail in theory, but panic would be too much
            let _ = new.insert_new_statement(
                name,
                &statement.to_string(),
                false,
            );
        }
        new
    }
}

pub trait StatementCache<'a> {
    fn new(db: &ConcurrentConnection) -> Self;
    fn is_statement_present(&self, identifier: &str) -> bool;
    fn insert_new_statement(
        &mut self,
        identifier: &str,
        statement: &str,
        can_update: bool,
    ) -> Result<QueryResult, RediSQLError>;
    fn delete_statement(
        &mut self,
        identifier: &str,
    ) -> Result<QueryResult, RediSQLError>;
    fn update_statement(
        &mut self,
        identifier: &str,
        statement: &str,
        can_create: bool,
    ) -> Result<QueryResult, RediSQLError>;
    fn exec_statement(
        &self,
        identifier: &str,
        args: &[&str],
    ) -> Result<Cursor, RediSQLError>;
    fn query_statement(
        &self,
        identifier: &str,
        args: &[&str],
    ) -> Result<Cursor, RediSQLError>;
    fn show_statement(
        &self,
        identifier: &str,
    ) -> Result<QueryResult, RediSQLError>;
    fn list_statements(&self) -> Result<QueryResult, RediSQLError>;
}

impl<'a> StatementCache<'a> for ReplicationBook {
    fn new(db: &ConcurrentConnection) -> Self {
        ReplicationBook {
            data: Arc::new(RwLock::new(FnvHashMap::default())),
            db: Arc::clone(db),
        }
    }
    fn is_statement_present(&self, identifier: &str) -> bool {
        self.data.read().unwrap().contains_key(identifier)
    }

    fn insert_new_statement(
        &mut self,
        identifier: &str,
        statement: &str,
        can_update: bool,
    ) -> Result<QueryResult, RediSQLError> {
        let db = self.db.clone();
        let mut map = self.data.write().unwrap();
        match map.entry(identifier.to_owned()) {
            Entry::Vacant(v) => {
                let stmt =
                    create_statement(db, identifier, statement)?;
                let read_only = stmt.is_read_only();
                v.insert((stmt, read_only));
                Ok(QueryResult::OK {})
            }
            Entry::Occupied(mut o) => {
                if can_update {
                    let stmt =
                        update_statement(&db, identifier, statement)?;
                    let read_only = stmt.is_read_only();
                    o.insert((stmt, read_only));
                    Ok(QueryResult::OK {})
                } else {
                    let debug =
                        String::from("Statement already present");
                    let description = String::from(
                    "The statement is already present in the database, try with UPDATE_STATEMENT",
                );
                    Err(RediSQLError::new(debug, description))
                }
            }
        }
    }

    fn delete_statement(
        &mut self,
        identifier: &str,
    ) -> Result<QueryResult, RediSQLError> {
        let db = self.db.clone();
        let mut map = self.data.write().unwrap();
        match map.entry(identifier.to_owned()) {
            Entry::Vacant(_) => {
                let debug = String::from("Statement not present.");
                let description = String::from(
                    "The statement is not present in the database, impossible to delete it.",
                );
                Err(RediSQLError::new(debug, description))
            }
            Entry::Occupied(o) => {
                remove_statement(&db, identifier)?;
                o.remove_entry();
                Ok(QueryResult::OK {})
            }
        }
    }

    fn update_statement(
        &mut self,
        identifier: &str,
        statement: &str,
        can_create: bool,
    ) -> Result<QueryResult, RediSQLError> {
        let db = self.db.clone();
        let mut map = self.data.write().unwrap();
        match map.entry(identifier.to_owned()) {
            Entry::Vacant(v) => {
                if can_create {
                    let stmt =
                        create_statement(db, identifier, statement)?;
                    let read_only = stmt.is_read_only();
                    v.insert((stmt, read_only));
                    Ok(QueryResult::OK {})
                } else {
                    let debug =
                        String::from("Statement not present.");
                    let description = String::from(
                    "The statement is not present in the database, impossible to update it.",
                );
                    Err(RediSQLError::new(debug, description))
                }
            }
            Entry::Occupied(mut o) => {
                let stmt =
                    update_statement(&db, identifier, statement)?;
                let read_only = stmt.is_read_only();
                o.insert((stmt, read_only));
                Ok(QueryResult::OK {})
            }
        }
    }

    fn query_statement(
        &self,
        identifier: &str,
        args: &[&str],
    ) -> Result<Cursor, RediSQLError> {
        let map = self.data.read().unwrap();
        match map.get(identifier) {
            None => {
                let debug = String::from("No statement found");
                let description = String::from(
                    "The statement is not present in the database",
                );
                Err(RediSQLError::new(debug, description))
            }
            Some(&(ref stmt, true)) => {
                stmt.reset();
                let stmt = bind_statement(stmt, args)?;
                let cursor = stmt.execute()?;
                Ok(cursor)
            }
            Some(&(_, false)) => {
                let debug = String::from("Not read only statement");
                let description = String::from("Statement is not read only but it may modify the database, use `EXEC_STATEMENT` instead.",);
                Err(RediSQLError::new(debug, description))
            }
        }
    }

    fn exec_statement(
        &self,
        identifier: &str,
        args: &[&str],
    ) -> Result<Cursor, RediSQLError> {
        let map = self.data.read().unwrap();
        match map.get(identifier) {
            None => {
                let debug = String::from("No statement found");
                let description = String::from(
                    "The statement is not present in the database",
                );
                Err(RediSQLError::new(debug, description))
            }
            Some(&(ref stmt, _)) => {
                stmt.reset();
                let stmt = bind_statement(stmt, args)?;
                let cursor = stmt.execute()?;
                Ok(cursor)
            }
        }
    }
    fn show_statement(
        &self,
        identifier: &str,
    ) -> Result<QueryResult, RediSQLError> {
        let map = self.data.read().unwrap();
        match map.get(identifier) {
            None => {
                let debug = String::from("No statement found");
                let description = String::from(
                    "The statement is not present in the database",
                );
                Err(RediSQLError::new(debug, description))
            }
            Some(&(ref stmt, read_only)) => {
                let names = vec![
                    "identifier".to_string(),
                    "SQL".to_string(),
                    "parameters_count".to_string(),
                    "read_only".to_string(),
                ];
                let types = vec!["TEXT", "TEXT", "INT", "INT"];
                let array = vec![
                    Entity::Text {
                        text: identifier.to_string(),
                    },
                    Entity::Text { text: stmt.sql() },
                    Entity::Integer {
                        int: stmt.parameters_count() as i64,
                    },
                    Entity::Integer {
                        int: if read_only { 1 } else { 0 },
                    },
                ];
                Ok(QueryResult::Array {
                    names,
                    types,
                    array,
                })
            }
        }
    }
    fn list_statements(&self) -> Result<QueryResult, RediSQLError> {
        let map = self.data.read().unwrap();
        let names = vec![
            "identifier".to_string(),
            "SQL".to_string(),
            "parameters_count".to_string(),
            "read_only".to_string(),
        ];
        let types = vec!["TEXT", "TEXT", "INT", "INT"];
        let n = map.len() * 4;
        let mut array = Vec::with_capacity(n);
        for (key, (multi_stmt, read_only)) in map.iter() {
            array.push(Entity::Text {
                text: key.to_string(),
            });
            array.push(Entity::Text {
                text: multi_stmt.sql(),
            });
            array.push(Entity::Integer {
                int: multi_stmt.parameters_count() as i64,
            });
            array.push(Entity::Integer {
                int: if *read_only { 1 } else { 0 },
            });
        }
        Ok(QueryResult::Array {
            names,
            types,
            array,
        })
    }
}

type ConcurrentConnection = Arc<Mutex<Connection>>;

#[derive(Clone)]
pub struct Loop {
    db: ConcurrentConnection,
    replication_book: ReplicationBook,
}

impl Drop for Loop {
    fn drop(&mut self) {
        debug!("### Dropping Loop ###")
    }
}

unsafe impl Send for Loop {}

pub trait LoopData {
    fn get_replication_book(&self) -> ReplicationBook;
    fn get_db(&self) -> ConcurrentConnection;
}

impl LoopData for Loop {
    fn get_replication_book(&self) -> ReplicationBook {
        self.replication_book.clone()
    }
    fn get_db(&self) -> ConcurrentConnection {
        Arc::clone(&self.db)
    }
}

impl Loop {
    fn new_from_arc(db: ConcurrentConnection) -> Self {
        let replication_book = ReplicationBook::new(&db);
        Loop {
            db,
            replication_book,
        }
    }
    fn new_from_db_and_replication_book(
        db: ConcurrentConnection,
        replication_book: ReplicationBook,
    ) -> Self {
        Loop {
            db,
            replication_book,
        }
    }
}

pub trait RedisReply {
    fn reply(&mut self, ctx: &rm::Context) -> i32;
    fn reply_v2(&mut self, ctx: &rm::Context) -> i32 {
        self.reply(ctx)
    }
}

impl RedisReply for Entity {
    fn reply(&mut self, ctx: &rm::Context) -> i32 {
        match *self {
            Entity::Integer { int } => {
                rm::ReplyWithLongLong(ctx, int)
            }
            Entity::Float { float } => {
                rm::ReplyWithDouble(ctx, float)
            }
            Entity::Text { ref text } => {
                rm::ReplyWithStringBuffer(ctx, text.as_bytes())
            }
            Entity::Blob { ref blob } => {
                rm::ReplyWithStringBuffer(ctx, blob.as_bytes())
            }
            Entity::Null => rm::ReplyWithNull(ctx),
            Entity::OK { .. } => (QueryResult::OK {}).reply(ctx),
            Entity::DONE { modified_rows, .. } => {
                QueryResult::DONE { modified_rows }.reply(ctx)
            }
        }
    }
}

fn reply_with_simple_string(
    ctx: *mut rm::ffi::RedisModuleCtx,
    s: &str,
) -> i32 {
    unsafe {
        rm::ffi::RedisModule_ReplyWithSimpleString.unwrap()(
            ctx,
            s.as_ptr() as *const c_char,
        )
    }
}

fn reply_with_ok(ctx: *mut rm::ffi::RedisModuleCtx) -> i32 {
    reply_with_simple_string(ctx, "OK\0")
}

fn reply_with_done(
    ctx: *mut rm::ffi::RedisModuleCtx,
    modified_rows: i32,
) -> i32 {
    unsafe {
        rm::ffi::RedisModule_ReplyWithArray.unwrap()(ctx, 2);
    }
    reply_with_simple_string(ctx, "DONE\0");
    unsafe {
        rm::ffi::RedisModule_ReplyWithLongLong.unwrap()(
            ctx,
            i64::from(modified_rows),
        );
    }
    rm::ffi::REDISMODULE_OK
}

fn reply_with_array(
    ctx: &rm::Context,
    mut array: impl RowFiller,
) -> i32 {
    unsafe {
        rm::ffi::RedisModule_ReplyWithArray.unwrap()(
            ctx.as_ptr(),
            rm::ffi::REDISMODULE_POSTPONED_ARRAY_LEN.into(),
        );
    }
    let mut row = Vec::new();
    let mut i = 0;
    while array.fill_row(&mut row) != None {
        i += 1;
        unsafe {
            rm::ffi::RedisModule_ReplyWithArray.unwrap()(
                ctx.as_ptr(),
                row.len() as c_long,
            );
        }

        for entity in row.iter_mut() {
            entity.reply(&ctx);
        }

        row.clear();
    }
    unsafe {
        rm::ffi::RedisModule_ReplySetArrayLength.unwrap()(
            ctx.as_ptr(),
            i,
        );
    }
    rm::ffi::REDISMODULE_OK
}

impl RedisReply for SQLite3Error {
    fn reply(&mut self, ctx: &Context) -> i32 {
        let error = format!("{}", self);
        reply_with_error(ctx.as_ptr(), error)
    }
}

impl RedisReply for RediSQLError {
    fn reply(&mut self, ctx: &Context) -> i32 {
        let error = format!("{}", self);
        reply_with_error(ctx.as_ptr(), error)
    }
}

fn reply_with_error(
    ctx: *mut rm::ffi::RedisModuleCtx,
    s: String,
) -> i32 {
    let s = CString::new(s).unwrap();
    unsafe {
        rm::ffi::RedisModule_ReplyWithError.unwrap()(ctx, s.as_ptr())
    }
}

pub fn create_argument(
    argv: *mut *mut rm::ffi::RedisModuleString,
    argc: i32,
) -> Result<Vec<&'static str>, RediSQLError> {
    match parse_args(argv, argc) {
        Err(e) => Err(RediSQLError::new(
            format!(
                "String valid up to byte number {}",
                e.valid_up_to()
            ),
            "Got a non-valid UTF8 string as input".to_string(),
        )),
        Ok(argvector) => Ok(argvector),
    }
}

fn parse_args(
    argv: *mut *mut rm::ffi::RedisModuleString,
    argc: i32,
) -> Result<Vec<&'static str>, std::str::Utf8Error> {
    let mut args: Vec<&'static str> =
        Vec::with_capacity(argc as usize);
    for i in 0..argc {
        let redis_str = unsafe { *argv.offset(i as isize) };
        let arg = unsafe { string_ptr_len(redis_str)? };
        args.push(arg);
    }
    Ok(args)
}

unsafe fn string_ptr_len(
    str: *mut rm::ffi::RedisModuleString,
) -> Result<&'static str, std::str::Utf8Error> {
    let mut len = 0;
    let base =
        rm::ffi::RedisModule_StringPtrLen.unwrap()(str, &mut len)
            as *mut u8;
    let slice = slice::from_raw_parts(base, len);
    let s = str::from_utf8(slice)?;
    Ok(s.trim_end_matches(char::from(0)))
}

pub enum KeyTypes {
    Empty,
    String,
    List,
    Hash,
    Set,
    Zset,
    ExternalModule,
    RediSQL,
    Stream,
    Unknow,
}

pub struct RedisKey {
    pub key: *mut rm::ffi::RedisModuleKey,
}

impl RedisKey {
    pub fn new(key_name: &str, ctx: &Context) -> Self {
        let key_name = rm::RMString::new(ctx, key_name);
        let key =
            rm::OpenKey(ctx, &key_name, rm::ffi::REDISMODULE_WRITE);
        RedisKey { key }
    }
    pub fn key_type(&self) -> KeyTypes {
        match unsafe {
            rm::ffi::RedisModule_KeyType.unwrap()(self.key)
        } {
            rm::ffi::REDISMODULE_KEYTYPE_MODULE => {
                if unsafe {
                    rm::ffi::RedisModule_ModuleTypeGetType.unwrap()(
                        self.key,
                    ) == rm::ffi::DBType
                } {
                    KeyTypes::RediSQL
                } else {
                    KeyTypes::ExternalModule
                }
            }
            rm::ffi::REDISMODULE_KEYTYPE_EMPTY => KeyTypes::Empty,
            rm::ffi::REDISMODULE_KEYTYPE_STRING => KeyTypes::String,
            rm::ffi::REDISMODULE_KEYTYPE_LIST => KeyTypes::List,
            rm::ffi::REDISMODULE_KEYTYPE_HASH => KeyTypes::Hash,
            rm::ffi::REDISMODULE_KEYTYPE_SET => KeyTypes::Set,
            rm::ffi::REDISMODULE_KEYTYPE_ZSET => KeyTypes::Zset,
            //rm::ffi::REDISMODULE_KEYTYPE_STREAM => KeyTypes::Stream,
            _ => KeyTypes::Unknow,
        }
    }
    fn get_dbkey(&self) -> Result<&DBKey, RediSQLError> {
        match self.key_type() {
            KeyTypes::RediSQL => {
                let dbkey = unsafe {
                    let dbkey = {
                        rm::ffi::RedisModule_ModuleTypeGetValue
                            .unwrap()(self.key)
                            as *mut DBKey
                    };
                    &*dbkey
                };
                Ok(dbkey)
            }
            KeyTypes::Empty => Err(RediSQLError::empty_key()),
            _ => Err(RediSQLError::no_redisql_key()),
        }
    }
    pub fn get_channel(
        &self,
    ) -> Result<Sender<Command>, RediSQLError> {
        let dbkey = self.get_dbkey()?;
        let channel = dbkey.tx.clone();
        Ok(channel)
    }
    pub fn get_db(
        &self,
    ) -> Result<ConcurrentConnection, RediSQLError> {
        let dbkey = self.get_dbkey()?;
        Ok(dbkey.loop_data.get_db())
    }
    pub fn get_loop_data(&self) -> Result<Loop, RediSQLError> {
        let dbkey = self.get_dbkey()?;
        Ok(dbkey.loop_data.clone())
    }
}

impl Drop for RedisKey {
    fn drop(&mut self) {
        unsafe {
            rm::ffi::RedisModule_CloseKey.unwrap()(self.key);
        }
    }
}

pub enum ReturnMethod {
    Reply,
    ReplyWithHeader,
    Stream { name: &'static str },
}

pub enum Command {
    Ping,
    Stop,
    Exec {
        query: &'static str,
        arguments: Vec<&'static str>,
        timeout: std::time::Instant,
        return_method: ReturnMethod,
        client: BlockedClient,
    },
    Query {
        query: &'static str,
        arguments: Vec<&'static str>,
        timeout: std::time::Instant,
        return_method: ReturnMethod,
        client: BlockedClient,
    },
    CompileStatement {
        identifier: &'static str,
        statement: &'static str,
        client: BlockedClient,
        can_update: bool,
    },
    ExecStatement {
        identifier: &'static str,
        arguments: Vec<&'static str>,
        timeout: std::time::Instant,
        return_method: ReturnMethod,
        client: BlockedClient,
    },
    UpdateStatement {
        identifier: &'static str,
        statement: &'static str,
        client: BlockedClient,
        can_create: bool,
    },
    DeleteStatement {
        identifier: &'static str,
        client: BlockedClient,
    },
    QueryStatement {
        identifier: &'static str,
        arguments: Vec<&'static str>,
        timeout: std::time::Instant,
        return_method: ReturnMethod,
        client: BlockedClient,
    },
    ShowStatement {
        identifier: &'static str,
        return_method: ReturnMethod,
        client: BlockedClient,
    },
    ListStatements {
        return_method: ReturnMethod,
        client: BlockedClient,
    },
    MakeCopy {
        destination: DBKey<'static>,
        client: BlockedClient,
    },
}

struct SQLiteResultIterator<'s> {
    num_columns: i32,
    previous_status: i32,
    stmt: &'s crate::community_statement::Statement,
}

impl<'s> SQLiteResultIterator<'s> {
    fn from_stmt(
        stmt: &'s crate::community_statement::Statement,
    ) -> Self {
        let num_columns =
            unsafe { sql::ffi::sqlite3_column_count(stmt.as_ptr()) };
        let previous_status = sql::ffi::SQLITE_ROW;
        Self {
            num_columns,
            previous_status,
            stmt,
        }
    }
    fn get_next_row(
        &mut self,
        row: &mut Vec<Entity>,
    ) -> Option<usize> {
        row.clear();
        if self.previous_status != sql::ffi::SQLITE_ROW {
            return None;
        }
        for i in 0..self.num_columns {
            let entity_value = Entity::new(self.stmt, i);
            row.push(entity_value);
        }
        unsafe {
            self.previous_status =
                sql::ffi::sqlite3_step(self.stmt.as_ptr());
        };
        Some(self.num_columns as usize)
    }
}

pub trait RowFiller {
    fn fill_row(&mut self, row: &mut Vec<Entity>) -> Option<usize>;
}

impl<'s> RowFiller for SQLiteResultIterator<'s> {
    fn fill_row(&mut self, row: &mut Vec<Entity>) -> Option<usize> {
        row.clear();
        self.get_next_row(row)
    }
}

impl<'r> RowFiller for std::slice::Chunks<'_, Entity> {
    fn fill_row(&mut self, row: &mut Vec<Entity>) -> Option<usize> {
        row.clear();
        let r = self.next();
        match r {
            None => None,
            Some(r) => {
                for e in r.iter() {
                    row.push(e.clone());
                }
                Some(r.len())
            }
        }
    }
}

struct QueryResultRowFiller<'qr> {
    filled_rows: u64,
    query_result: &'qr QueryResult,
}

impl<'qr> From<&'qr mut QueryResult> for QueryResultRowFiller<'qr> {
    fn from(q: &'qr mut QueryResult) -> Self {
        QueryResultRowFiller {
            filled_rows: 0,
            query_result: q,
        }
    }
}

impl RowFiller for QueryResultRowFiller<'_> {
    fn fill_row(&mut self, row: &mut Vec<Entity>) -> Option<usize> {
        row.clear();
        match &self.query_result {
            QueryResult::OK {} => match self.filled_rows {
                0 => {
                    row.push(Entity::Text {
                        text: "OK".to_string(),
                    });
                    self.filled_rows += 1;
                    Some(1)
                }
                _ => None,
            },
            QueryResult::DONE { modified_rows } => {
                match self.filled_rows {
                    0 => {
                        let v = Entity::Text {
                            text: "DONE".to_string(),
                        };
                        row.push(v);
                        self.filled_rows += 1;
                        Some(1)
                    }
                    1 => {
                        let v = Entity::Integer {
                            int: *modified_rows as i64,
                        };
                        row.push(v);
                        self.filled_rows += 1;
                        Some(1)
                    }
                    _ => None,
                }
            }
            QueryResult::Array { array, names, .. } => {
                if self.filled_rows == 0 {
                    let v = Entity::Text {
                        text: "RESULT".to_string(),
                    };
                    row.push(v);
                    self.filled_rows += 1;
                    return Some(1);
                }
                let len = names.len();
                let offset = (self.filled_rows as usize - 1) * len;
                if array.len() <= offset {
                    return None;
                }
                for i in 0..len {
                    row.push(array[offset + i].clone())
                }
                self.filled_rows += 1;
                Some(len)
            }
            QueryResult::Stream {
                stream,
                first_id,
                last_id,
                size,
            } => match self.filled_rows {
                0 => {
                    let v = Entity::Text {
                        text: "STREAM".to_string(),
                    };
                    row.push(v);
                    self.filled_rows += 1;
                    Some(1)
                }
                1 => {
                    let v = Entity::Text {
                        text: stream.to_owned(),
                    };
                    row.push(v);
                    let v = Entity::Text {
                        text: first_id.to_owned(),
                    };
                    row.push(v);
                    let v = Entity::Text {
                        text: last_id.to_owned(),
                    };
                    row.push(v);
                    let v = Entity::Text {
                        text: size.to_string(),
                    };
                    row.push(v);
                    self.filled_rows += 1;
                    Some(1)
                }
                _ => None,
            },
        }
    }
}

impl<'s> Iterator for SQLiteResultIterator<'s> {
    type Item = Vec<Entity>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.previous_status != sql::ffi::SQLITE_ROW {
            return None;
        }
        let mut row = Vec::with_capacity(self.num_columns as usize);
        for i in 0..self.num_columns {
            let entity_value = Entity::new(self.stmt, i);
            row.push(entity_value);
        }
        unsafe {
            self.previous_status =
                sql::ffi::sqlite3_step(self.stmt.as_ptr());
        };

        Some(row)
    }
}

pub trait Returner {
    fn create_data_to_return(
        self,
        ctx: &Context,
        return_method: &ReturnMethod,
        timeout: std::time::Instant,
    ) -> Box<Box<dyn RedisReply>>;
}

impl Returner for QueryResult {
    fn create_data_to_return(
        self,
        ctx: &Context,
        return_method: &ReturnMethod,
        timeout: std::time::Instant,
    ) -> Box<Box<dyn RedisReply>> {
        match return_method {
            ReturnMethod::Stream { name: stream_name } => {
                match self {
                    QueryResult::Array {
                        array,
                        names: columns_names,
                        ..
                    } => {
                        match stream_query_result_array(
                            ctx,
                            stream_name,
                            &columns_names,
                            array.chunks(columns_names.len()),
                            timeout,
                        ) {
                            Ok(res) => Box::new(Box::new(res)),
                            Err(e) => Box::new(Box::new(e)),
                        }
                    }
                    _ => Box::new(Box::new(self)),
                }
            }
            ReturnMethod::ReplyWithHeader {} => match self {
                QueryResult::Array {
                    mut array,
                    names,
                    types,
                } => {
                    let mut new_array = Vec::with_capacity(
                        names.len() + types.len() + array.len(),
                    );
                    for name in names.clone() {
                        new_array.push(Entity::Text { text: name });
                    }
                    for t in &types {
                        new_array.push(Entity::Text {
                            text: t.to_string(),
                        });
                    }
                    new_array.append(&mut array);
                    let res = QueryResult::Array {
                        names,
                        types,
                        array: new_array,
                    };
                    Box::new(Box::new(res))
                }
                _ => Box::new(Box::new(self)),
            },
            _ => Box::new(Box::new(self)),
        }
    }
}

impl Returner for RediSQLError {
    fn create_data_to_return(
        self,
        _ctx: &Context,
        _return_method: &ReturnMethod,
        _timeout: std::time::Instant,
    ) -> Box<Box<dyn RedisReply>> {
        Box::new(Box::new(self))
    }
}

impl<'s> Returner for Cursor {
    fn create_data_to_return(
        self,
        ctx: &Context,
        return_method: &ReturnMethod,
        timeout: std::time::Instant,
    ) -> Box<Box<dyn RedisReply>> {
        match self {
            Cursor::RowsCursor {
                ref stmt,
                num_columns,
                ..
            } => match return_method {
                ReturnMethod::Stream { name: stream_name } => {
                    let mut names =
                        Vec::with_capacity(num_columns as usize);
                    for i in 0..num_columns {
                        let name = unsafe {
                            CStr::from_ptr(
                                sql::ffi::sqlite3_column_name(
                                    stmt.as_ptr(),
                                    i,
                                ),
                            )
                            .to_string_lossy()
                            .into_owned()
                        };
                        names.push(name);
                    }

                    match stream_query_result_array(
                        ctx,
                        stream_name,
                        &names,
                        SQLiteResultIterator::from_stmt(stmt),
                        timeout,
                    ) {
                        Ok(res) => Box::new(Box::new(res)),
                        Err(e) => Box::new(Box::new(e)),
                    }
                }
                ReturnMethod::ReplyWithHeader => {
                    let query_result =
                        QueryResult::from_cursor_before_with_header(
                            self, timeout,
                        );
                    Box::new(Box::new(query_result))
                }
                ReturnMethod::Reply => {
                    let query_result =
                        QueryResult::from_cursor_before(
                            self, timeout,
                        );
                    Box::new(Box::new(query_result))
                }
            },
            Cursor::OKCursor => {
                Box::new(Box::new(QueryResult::OK {}))
            }
            Cursor::DONECursor { modified_rows } => {
                Box::new(Box::new(QueryResult::DONE {
                    modified_rows,
                }))
            }
        }
    }
}

impl RedisReply for Result<QueryResult, err::RediSQLError> {
    fn reply(&mut self, ctx: &Context) -> i32 {
        match self {
            Ok(ok) => ok.reply(ctx),
            Err(e) => e.reply(ctx),
        }
    }
    fn reply_v2(&mut self, ctx: &Context) -> i32 {
        match self {
            Ok(ok) => ok.reply_v2(ctx),
            Err(e) => e.reply_v2(ctx),
        }
    }
}

impl RedisReply for QueryResult {
    fn reply(&mut self, ctx: &rm::Context) -> i32 {
        match self {
            QueryResult::OK {} => reply_with_ok(ctx.as_ptr()),
            QueryResult::DONE { modified_rows, .. } => {
                reply_with_done(ctx.as_ptr(), *modified_rows)
            }
            QueryResult::Array { array, names, .. } => {
                debug!("QueryResult::Array");
                reply_with_array(ctx, array.chunks(names.len()))
            }
            QueryResult::Stream {
                stream,
                first_id,
                last_id,
                size,
            } => {
                let row = vec![
                    Entity::Text {
                        text: stream.to_owned(),
                    },
                    Entity::Text {
                        text: first_id.to_owned(),
                    },
                    Entity::Text {
                        text: last_id.to_owned(),
                    },
                    Entity::Text {
                        text: size.to_string(),
                    },
                ];
                reply_with_array(ctx, row.chunks(4))
            }
        }
    }
    fn reply_v2(&mut self, ctx: &rm::Context) -> i32 {
        let filler = QueryResultRowFiller::from(self);
        reply_with_array(ctx, filler)
    }
}

pub fn do_execute(
    db: &ConcurrentConnection,
    query: &str,
    args: &Vec<&str>,
) -> Result<impl Returner, err::RediSQLError> {
    let mut stmt = MultiStatement::new(db.clone(), query)?;
    stmt.reset();
    let stmt = bind_statement(&mut stmt, args)?;
    debug!("do_execute | created statement");
    let cursor = stmt.execute()?;
    debug!("do_execute | statement executed");
    Ok(cursor)
}

pub fn do_query(
    db: &ConcurrentConnection,
    query: &str,
    args: &Vec<&str>,
) -> Result<impl Returner, err::RediSQLError> {
    let mut stmt = MultiStatement::new(db.clone(), query)?;
    if stmt.is_read_only() {
        stmt.reset();
        let stmt = bind_statement(&mut stmt, args)?;
        Ok(stmt.execute()?)
    } else {
        let debug = String::from("Not read only statement");
        let description = String::from("Statement is not read only but it may modify the database, use `EXEC_STATEMENT` instead.",);
        Err(RediSQLError::new(debug, description))
    }
}

/// implements the copy of the source database into the destination one
/// it also leak the two DBKeys
pub fn do_copy(
    source_db: &ConcurrentConnection,
    destination_loopdata: &Loop,
) -> Result<impl Returner, err::RediSQLError> {
    debug!("DoCopy | Start");

    let destination_path = {
        let db = destination_loopdata.get_db();
        get_path_from_db(db)
    }?;

    let backup_result = {
        let destination_db = destination_loopdata.get_db();

        let destination_db = destination_db.lock().unwrap();
        let source_db = source_db.lock().unwrap();
        match make_backup(&source_db, &destination_db) {
            Err(e) => Err(RediSQLError::from(e)),
            Ok(_) => Ok(QueryResult::OK {}),
        }
    };

    if backup_result.is_ok() {
        restore_previous_statements(destination_loopdata);
        update_path_metadata(
            destination_loopdata.get_db(),
            &destination_path,
        )?;
    }
    debug!("DoCopy | End");

    backup_result
}

fn bind_statement<'a>(
    stmt: &'a MultiStatement,
    arguments: &[&str],
) -> Result<&'a MultiStatement, SQLite3Error> {
    match stmt.bind_texts(arguments) {
        Err(e) => Err(e),
        Ok(_) => Ok(stmt),
    }
}

/// restore_previous_statements read the statements written in the database and add them to the
/// loopdata datastructure.
/// At the moment this function returns `()` no matter if there are errors or not.
/// Errors are pretty unlikely, especially if nobody touched manually the metadata database, but
/// still they may happens.
/// I am not sure if it is a good idea or if I should upgrade the code to return an error, and
/// maybe just ignore the error to keep the whole flow as it is now.
fn restore_previous_statements<'a, L: 'a + LoopData>(loopdata: &L) {
    let saved_statements = get_statement_metadata(loopdata.get_db());
    match saved_statements {
        Ok(QueryResult::Array { array, names, .. }) => {
            for row in array.chunks(names.len()) {
                let identifier = match row[1] {
                    Entity::Text { ref text } => text,
                    _ => continue,
                };
                let statement = match row[2] {
                    Entity::Text { ref text } => text,
                    _ => continue,
                };
                if let Err(e) = compile_and_insert_statement(
                    identifier, statement, loopdata,
                ) {
                    println!("Error: {}", e)
                }
            }
        }
        Err(e) => println!("Error: {}", e),
        _ => (),
    }
}

fn return_value(
    client: &BlockedClient,
    return_method: &ReturnMethod,
    result: Result<impl Returner, err::RediSQLError>,
    timeout: std::time::Instant,
) {
    let ctx = Context::thread_safe(client);
    let result = match result {
        Ok(res) => {
            res.create_data_to_return(&ctx, return_method, timeout)
        }
        Err(e) => {
            e.create_data_to_return(&ctx, return_method, timeout)
        }
    };
    unsafe {
        rm::ffi::RedisModule_UnblockClient.unwrap()(
            client.client,
            Box::into_raw(result) as *mut std::os::raw::c_void,
        );
    }
}

pub fn stream_query_result_array<A>(
    context: &Context,
    stream_name: &str,
    columns_names: &[String],
    mut array: A,
    timeout: std::time::Instant,
) -> Result<QueryResult, err::RediSQLError>
where
    A: RowFiller,
{
    let mut result = Vec::with_capacity(4);
    result.push(Entity::Text {
        text: stream_name.to_string(),
    });

    let mut i = 0;
    let mut first_stream_index = None;
    let mut second_stream_index = None;

    let mut now = std::time::Instant::now();

    if now > timeout {
        return Err(err::RediSQLError::timeout());
    }

    let mut lock = context.lock();
    let mut row = Vec::new();
    while array.fill_row(&mut row) != None {
        now = std::time::Instant::now();

        if now > timeout {
            context.release(lock);
            return Err(err::RediSQLError::timeout());
        }

        if i % 256 == 255 {
            // avoid that a big results lock the context for too long, should help in
            // keeping the latency low.
            context.release(lock);
            lock = context.lock();
        }
        let mut xadd = rm::XADDCommand::new(&context, stream_name);

        for (j, entity) in row.iter().enumerate() {
            match entity {
                Entity::OK {} | Entity::DONE { .. } => {
                    // do nothing
                }
                Entity::Null => {
                    xadd.add_element(
                        &format!("null:{}", &columns_names[j]),
                        "(null)",
                    );
                }
                Entity::Integer { int } => {
                    xadd.add_element(
                        &format!("int:{}", &columns_names[j]),
                        &int.to_string(),
                    );
                }
                Entity::Float { float } => {
                    xadd.add_element(
                        &format!("real:{}", &columns_names[j]),
                        &float.to_string(),
                    );
                }
                Entity::Text { text } => {
                    xadd.add_element(
                        &format!("text:{}", &columns_names[j]),
                        &text,
                    );
                }
                Entity::Blob { blob } => {
                    xadd.add_element(
                        &format!("blob:{}", &columns_names[j]),
                        &blob,
                    );
                }
            }
        }
        debug!("XADD {:?}", xadd);
        let xadd_result = xadd.execute(&lock);
        match xadd_result {
            rm::CallReply::RString { .. } => match i {
                0 => {
                    let stream_index = Entity::Text {
                        text: xadd_result.access_string().unwrap(),
                    };
                    first_stream_index = Some(stream_index.clone());
                    second_stream_index = Some(stream_index);
                }
                _ => {
                    second_stream_index = Some(Entity::Text {
                        text: xadd_result.access_string().unwrap(),
                    });
                }
            },
            rm::CallReply::RError { .. } => {
                context.release(lock);
                return Err(RediSQLError::new(
                    xadd_result.access_error().unwrap(),
                    format!("Error in XADD to {}", stream_name),
                ));
                // return an error and unlock
            }
            _ => {
                context.release(lock);
                return Err(RediSQLError::new(
                        "Stream not supported".to_string(),
                            "Stream seems to don't be supported, it is a version of Redis > 5?".to_string(),
                        ));
            }
        };
        i += 1;
    }
    context.release(lock);

    result.push(
        first_stream_index
            .expect("Not found first index when returning a stream"),
    );
    result
        .push(second_stream_index.expect(
            "Not found second index when returning a stream",
        ));
    result.push(Entity::Integer { int: i });

    Ok(QueryResult::Array {
        names: vec![
            String::from("stream"),
            String::from("first_id"),
            String::from("last_id"),
            String::from("size"),
        ],
        types: vec!["TEXT", "TEXT", "TEXT", "INT"],
        array: result,
    })
}

pub fn listen_and_execute<'a, L: 'a + LoopData>(
    loopdata: &mut L,
    rx: &Receiver<Command>,
) {
    debug!("Start thread execution");
    restore_previous_statements(loopdata);
    debug!("Done restoring statements");
    loop {
        debug!("Loop iteration");
        match rx.recv() {
            Ok(Command::Ping {}) => debug!("Received PING!"),
            Ok(Command::Exec {
                query,
                arguments,
                client,
                return_method,
                timeout,
            }) => {
                debug!("Exec | Query = {:?}", query);
                let result =
                    do_execute(&loopdata.get_db(), query, &arguments);
                match result {
                    Ok(_) => STATISTICS.exec_ok(),
                    Err(_) => STATISTICS.exec_err(),
                }
                return_value(
                    &client,
                    &return_method,
                    result,
                    timeout,
                );
                debug!("Exec | DONE, returning result");
            }
            Ok(Command::Query {
                query,
                arguments,
                timeout,
                return_method,
                client,
            }) => {
                debug!("Query | Query = {:?}", query);
                let result =
                    do_query(&loopdata.get_db(), query, &arguments);

                match (&return_method, &result) {
                    (ReturnMethod::Reply, Ok(_)) => {
                        STATISTICS.query_ok()
                    }
                    (ReturnMethod::Reply, Err(_)) => {
                        STATISTICS.query_err()
                    }
                    (ReturnMethod::ReplyWithHeader, Ok(_)) => {
                        STATISTICS.query_ok()
                    }
                    (ReturnMethod::ReplyWithHeader, Err(_)) => {
                        STATISTICS.query_err()
                    }
                    (ReturnMethod::Stream { .. }, Ok(_)) => {
                        STATISTICS.query_into_ok()
                    }
                    (ReturnMethod::Stream { .. }, Err(_)) => {
                        STATISTICS.query_into_err()
                    }
                };
                return_value(
                    &client,
                    &return_method,
                    result,
                    timeout,
                );
            }
            Ok(Command::UpdateStatement {
                identifier,
                statement,
                can_create,
                client,
            }) => {
                debug!(
                    "UpdateStatement | Identifier = {:?} Statement = {:?}",
                    identifier, statement
                );
                let result =
                    loopdata.get_replication_book().update_statement(
                        identifier, statement, can_create,
                    );
                match result {
                    Ok(_) => STATISTICS.update_statement_ok(),
                    Err(_) => STATISTICS.update_statement_err(),
                };
                let t = std::time::Instant::now()
                    + std::time::Duration::from_secs(10);

                return_value(&client, &ReturnMethod::Reply, result, t)
            }
            Ok(Command::DeleteStatement { identifier, client }) => {
                debug!(
                    "DeleteStatement | Identifier = {:?}",
                    identifier
                );
                let result = loopdata
                    .get_replication_book()
                    .delete_statement(identifier);
                match result {
                    Ok(_) => STATISTICS.delete_statement_ok(),
                    Err(_) => STATISTICS.delete_statement_err(),
                }
                let t = std::time::Instant::now()
                    + std::time::Duration::from_secs(10);

                return_value(
                    &client,
                    &ReturnMethod::Reply,
                    result,
                    t,
                );
            }
            Ok(Command::CompileStatement {
                identifier,
                statement,
                client,
                can_update,
            }) => {
                debug!(
                    "CompileStatement | Identifier = {:?} Statement = {:?}",
                    identifier, statement
                );
                let result = loopdata
                    .get_replication_book()
                    .insert_new_statement(
                        identifier, statement, can_update,
                    );
                match result {
                    Ok(_) => STATISTICS.create_statement_ok(),
                    Err(_) => STATISTICS.create_statement_err(),
                }
                let t = std::time::Instant::now()
                    + std::time::Duration::from_secs(10);

                return_value(
                    &client,
                    &ReturnMethod::Reply,
                    result,
                    t,
                );
            }

            Ok(Command::ExecStatement {
                identifier,
                arguments,
                timeout,
                return_method,
                client,
            }) => {
                debug!(
                    "ExecStatement | Identifier = {:?} Arguments = {:?}",
                    identifier, arguments
                );
                let result = loopdata
                    .get_replication_book()
                    .exec_statement(identifier, &arguments);
                match result {
                    Ok(_) => STATISTICS.exec_statement_ok(),
                    Err(_) => STATISTICS.exec_statement_err(),
                }

                return_value(
                    &client,
                    &return_method,
                    result,
                    timeout,
                );
            }
            Ok(Command::QueryStatement {
                identifier,
                arguments,
                return_method,
                timeout,
                client,
            }) => {
                let result =
                    loopdata.get_replication_book().query_statement(
                        identifier,
                        arguments.as_slice(),
                    );
                match (&return_method, &result) {
                    (ReturnMethod::Reply, Ok(_)) => {
                        STATISTICS.query_statement_ok()
                    }
                    (ReturnMethod::Reply, Err(_)) => {
                        STATISTICS.query_statement_err()
                    }
                    (ReturnMethod::ReplyWithHeader, Ok(_)) => {
                        STATISTICS.query_statement_ok()
                    }
                    (ReturnMethod::ReplyWithHeader, Err(_)) => {
                        STATISTICS.query_statement_err()
                    }
                    (ReturnMethod::Stream { .. }, Ok(_)) => {
                        STATISTICS.query_statement_into_ok()
                    }
                    (ReturnMethod::Stream { .. }, Err(_)) => {
                        STATISTICS.query_statement_into_err()
                    }
                };

                return_value(
                    &client,
                    &return_method,
                    result,
                    timeout,
                );
            }
            Ok(Command::ShowStatement {
                identifier,
                return_method,
                client,
            }) => {
                let result = loopdata
                    .get_replication_book()
                    .show_statement(identifier);
                let t = std::time::Instant::now()
                    + std::time::Duration::from_secs(10);
                return_value(&client, &return_method, result, t);
            }
            Ok(Command::ListStatements {
                return_method,
                client,
            }) => {
                let result =
                    loopdata.get_replication_book().list_statements();
                let t = std::time::Instant::now()
                    + std::time::Duration::from_secs(10);
                return_value(&client, &return_method, result, t);
            }
            Ok(Command::MakeCopy {
                destination,
                client,
            }) => {
                debug!("MakeCopy | Doing do_copy");
                let destination_loopdata = &destination.loop_data;
                let result =
                    do_copy(&loopdata.get_db(), destination_loopdata);
                match result {
                    Ok(_) => STATISTICS.copy_ok(),
                    Err(_) => STATISTICS.copy_err(),
                };
                let t = std::time::Instant::now()
                    + std::time::Duration::from_secs(10);

                return_value(
                    &client,
                    &ReturnMethod::Reply,
                    result,
                    t,
                );
                std::mem::forget(destination);
            }
            Ok(Command::Stop) => {
                dbg!("Stop, exiting from work loop");
                return;
            }
            Err(RecvError) => {
                dbg!(
                    "RecvError {}, exiting from work loop",
                    RecvError
                );
                return;
            }
        }
    }
}

fn compile_and_insert_statement<'a, L: 'a + LoopData>(
    identifier: &str,
    statement: &str,
    loop_data: &L,
) -> Result<QueryResult, err::RediSQLError> {
    let stmt_cache = &loop_data.get_replication_book().data;
    let mut statements_cache = stmt_cache.write().unwrap();
    /* On the map (statements_cache) we need to own the value of
     * identifiers, we are ok with this
     * since it happens rarely.
     */
    match statements_cache.entry(identifier.to_owned()) {
        Entry::Vacant(v) => {
            let db = loop_data.get_db();
            match create_statement(db, identifier, statement) {
                Ok(stmt) => {
                    let read_only = stmt.is_read_only();
                    v.insert((stmt, read_only));
                    Ok(QueryResult::OK {})
                }
                Err(e) => Err(e),
            }
        }
        Entry::Occupied(_) => {
            let err = RediSQLError::new(
                "Statement already exists".to_string(),
                String::from(
                    "Impossible to overwrite it with \
                     this command, try with \
                     UPDATE_STATEMENT",
                ),
            );

            Err(err)
        }
    }
}

pub struct DBKey<'c> {
    pub tx: Sender<Command>,
    pub loop_data: Loop,
    pub context: Option<&'c Context>,
    pub connections: HashMap<String, Sender<Command>>,
}

impl<'c> DBKey<'c> {
    pub fn new_from_arc(
        tx: Sender<Command>,
        db: ConcurrentConnection,
    ) -> Self {
        let loop_data = Loop::new_from_arc(db);
        DBKey {
            tx,
            loop_data,
            connections: HashMap::new(),
            context: None,
        }
    }
    pub fn add_connection(
        &mut self,
        clone_name: &str,
    ) -> Result<(), ()> {
        if self.connections.get(clone_name).is_some() {
            return Err(());
        }
        {
            let replication_book =
                self.loop_data.get_replication_book();
            let db = self.loop_data.get_db();
            let db = db.lock().unwrap();
            if db.is_multithread() {
                let serialized_db = match db.duplicate_connection() {
                    Ok(db) => Arc::new(Mutex::new(db)),
                    Err(_) => return Err(()),
                };
                let new_replication_book = replication_book
                    .clone_replication_book(&serialized_db);
                let mut new_loop =
                    Loop::new_from_db_and_replication_book(
                        serialized_db,
                        new_replication_book,
                    );
                let (new_tx, new_rx) = channel();
                if self.tx.send(Command::Stop).is_err() {
                    return Err(());
                }
                self.tx = new_tx;
                self.loop_data = new_loop.clone();
                thread::spawn(move || {
                    listen_and_execute(&mut new_loop, &new_rx);
                });
            }
        }

        let db = self.loop_data.get_db();
        let db = db.lock().unwrap();
        let new_db = match db.duplicate_connection() {
            Ok(db) => Arc::new(Mutex::new(db)),
            Err(_) => return Err(()),
        };
        let new_replication_book = self
            .loop_data
            .get_replication_book()
            .clone_replication_book(&new_db);
        let mut new_loop = Loop::new_from_db_and_replication_book(
            new_db,
            new_replication_book,
        );
        let (tx, rx) = channel();
        thread::spawn(move || {
            listen_and_execute(&mut new_loop, &rx);
        });
        self.connections.insert(clone_name.to_string(), tx);
        Ok(())
    }
}

impl<'c> Drop for DBKey<'c> {
    fn drop(&mut self) {
        debug!("### Dropping DBKey ###")
    }
}

pub fn create_metadata_table(
    db: ConcurrentConnection,
) -> Result<ConcurrentConnection, SQLite3Error> {
    let statement = "CREATE TABLE IF NOT EXISTS RediSQLMetadata(data_type TEXT, key TEXT, value TEXT);";

    let stmt = MultiStatement::new(db.clone(), statement)?;
    stmt.execute()?;
    Ok(db)
}

pub fn insert_metadata(
    db: ConcurrentConnection,
    data_type: &str,
    key: &str,
    value: &str,
) -> Result<ConcurrentConnection, SQLite3Error> {
    let statement = "INSERT INTO RediSQLMetadata VALUES(?1, ?2, ?3);";

    let stmt = MultiStatement::new(db.clone(), statement)?;
    stmt.bind_index(1, data_type)?;
    stmt.bind_index(2, key)?;
    stmt.bind_index(3, value)?;
    stmt.execute()?;
    Ok(db)
}

pub fn enable_foreign_key_v2(
    db: Result<ConcurrentConnection, SQLite3Error>,
) -> Result<(), SQLite3Error> {
    let enable_foreign_key = "PRAGMA foreign_keys = ON;";
    match MultiStatement::new(db.expect("cve"), enable_foreign_key) {
        Err(e) => Err(e),
        Ok(stmt) => match stmt.execute() {
            Err(e) => Err(e),
            Ok(_) => Ok(()),
        },
    }
}
pub fn enable_foreign_key(
    db: ConcurrentConnection,
) -> Result<ConcurrentConnection, SQLite3Error> {
    let enable_foreign_key = "PRAGMA foreign_keys = ON;";
    match MultiStatement::new(db.clone(), enable_foreign_key) {
        Err(e) => Err(e),
        Ok(stmt) => match stmt.execute() {
            Err(e) => Err(e),
            Ok(_) => Ok(db),
        },
    }
}

fn update_statement_metadata(
    db: ConcurrentConnection,
    key: &str,
    value: &str,
) -> Result<(), SQLite3Error> {
    let statement =
        "UPDATE RediSQLMetadata SET value = ?1 WHERE data_type = 'statement' AND key = ?2";

    let stmt = MultiStatement::new(db, statement)?;
    stmt.bind_index(1, value)?;
    stmt.bind_index(2, key)?;
    stmt.execute()?;
    Ok(())
}

fn remove_statement_metadata(
    db: ConcurrentConnection,
    key: &str,
) -> Result<(), SQLite3Error> {
    let statement = "DELETE FROM RediSQLMetadata WHERE data_type = 'statement' AND key = ?1";

    let stmt = MultiStatement::new(db, statement)?;
    stmt.bind_index(1, key)?;
    stmt.execute()?;
    Ok(())
}

fn get_statement_metadata(
    db: ConcurrentConnection,
) -> Result<QueryResult, err::RediSQLError> {
    let statement = "SELECT * FROM RediSQLMetadata WHERE data_type = 'statement';";

    let stmt = MultiStatement::new(db, statement)?;
    let cursor = stmt.execute()?;
    QueryResult::try_from(cursor)
}

fn get_path_metadata(
    db: ConcurrentConnection,
) -> Result<QueryResult, err::RediSQLError> {
    let statement = "SELECT value FROM RediSQLMetadata WHERE data_type = 'path' AND key = 'path';";

    let stmt = MultiStatement::new(db, statement)?;
    let cursor = stmt.execute()?;
    QueryResult::try_from(cursor)
}

pub fn is_redisql_database(db: ConcurrentConnection) -> bool {
    let query = "SELECT name FROM sqlite_master WHERE type='table' AND name='RediSQLMetadata;";

    let query = MultiStatement::new(db, query);
    if query.is_err() {
        return false;
    };

    let query = query.unwrap();
    let cursor = query.execute();
    if cursor.is_err() {
        return false;
    };

    match QueryResult::try_from(cursor.unwrap()) {
        Ok(QueryResult::Array { .. }) => true,
        Ok(_) => false,
        Err(_) => false,
    }
}

pub fn get_path_from_db(
    db: ConcurrentConnection,
) -> Result<String, RediSQLError> {
    match get_path_metadata(db) {
        Err(e) => Err(e),
        // we have one big vector of results, else the first element is just [0] and not [0][0]
        // it use to be a matrix, is not anymore the case.
        Ok(QueryResult::Array { array, .. }) => match array[0] {
            Entity::Text { ref text } => match text {
                t if t.is_empty() => {
                    let err = RediSQLError::new(
                        "Found empty path".to_string(),
                        "The field of the path of the database is empty in the metadata table.".to_string());
                    Err(err)
                }
                t => Ok(t.to_string()),
            },

            _ => {
                let err = RediSQLError::new(
                    "Not found path as text of the database in metadata".to_string(),
                    "While looking into the metadata of the database we found information about the path of the database itself, but the path was expected to be of TEXT type while it is not".to_string());
                Err(err)
            }
        },
        _ => Err(RediSQLError::new(
                "Path not found".to_string(),
                "Couldn't find the path of the database in the metadata table".to_string())),
    }
}

pub fn insert_path_metadata(
    db: ConcurrentConnection,
    path: &str,
) -> Result<ConcurrentConnection, SQLite3Error> {
    insert_metadata(db, "path", "path", path)
}

fn update_path_metadata(
    db: ConcurrentConnection,
    value: &str,
) -> Result<(), SQLite3Error> {
    let statement =
        "UPDATE RediSQLMetadata SET value = ?1 WHERE data_type = 'path' AND key = 'path'";

    let stmt = MultiStatement::new(db, statement)?;
    stmt.bind_index(1, value)?;
    stmt.execute()?;
    Ok(())
}

pub fn make_backup(
    conn1: &Connection,
    conn2: &Connection,
) -> Result<i32, SQLite3Error> {
    match sql::create_backup(conn1, conn2) {
        Err(e) => Err(e),
        Ok(bk) => {
            let mut result = unsafe { sql::BackupStep(&bk, 1) };
            while sql::backup_should_step_again(result) {
                result = unsafe { sql::BackupStep(&bk, 1) };
            }
            unsafe { sql::BackupFinish(&bk) };
            Ok(result)
        }
    }
}

pub fn create_backup(
    conn: &Connection,
    path: &str,
) -> Result<i32, SQLite3Error> {
    match Connection::open_connection(path) {
        Err(e) => Err(e),
        Ok(new_db) => make_backup(conn, &new_db),
    }
}

pub unsafe fn write_file_to_rdb(
    f: File,
    rdb: *mut rm::ffi::RedisModuleIO,
) -> Result<(), std::io::Error> {
    let block_size = 1024 * 4 * 10;
    let lenght = f.metadata().unwrap().len();
    let blocks = lenght / block_size;
    let blocks = match lenght % block_size {
        0 => blocks,
        _n => blocks + 1,
    };

    rm::SaveSigned(rdb, blocks as i64);
    debug!(
        "Saved {} blocks from a file of len {} and block of size {}",
        blocks, lenght, block_size
    );

    let to_write: Vec<u8> = vec![0; block_size as usize];
    let mut buffer = BufReader::with_capacity(block_size as usize, f);
    loop {
        let mut tw = to_write.clone();
        match buffer.read(tw.as_mut_slice()) {
            Ok(0) => return Ok(()),
            Ok(n) => rm::SaveStringBuffer(rdb, tw.as_slice(), n),
            Err(e) => return Err(e),
        }
    }
}

struct SafeRedisModuleString {
    ptr: *mut std::os::raw::c_char,
}

impl Drop for SafeRedisModuleString {
    fn drop(&mut self) {
        unsafe {
            rm::ffi::RedisModule_Free.unwrap()(
                self.ptr as *mut std::os::raw::c_void,
            )
        }
    }
}

pub unsafe fn write_rdb_to_file(
    f: &mut File,
    rdb: *mut rm::ffi::RedisModuleIO,
) -> Result<(), std::io::Error> {
    let blocks = rm::LoadSigned(rdb);
    for _ in 0..blocks {
        debug!("Loop reading");
        let mut dimension: usize = 0;
        let c_str_ptr = SafeRedisModuleString {
            ptr: rm::ffi::RedisModule_LoadStringBuffer.unwrap()(
                rdb,
                &mut dimension,
            ),
        };
        debug!("Read {} bytes!", dimension);
        if dimension == 0 {
            break;
        }
        let slice = slice::from_raw_parts(
            c_str_ptr.ptr as *mut u8,
            dimension,
        );
        let y = f.write_all(slice);
        if let Err(e) = y {
            debug!("Error in writing to file: {}", e);
            return Err(e);
        }
    }
    Ok(())
}

pub fn get_dbkeyptr_from_name(
    ctx: *mut rm::ffi::RedisModuleCtx,
    name: &str,
) -> Result<*mut DBKey, i32> {
    let context = Context::new(ctx);
    let key_name = rm::RMString::new(&context, name);
    let key =
        OpenKey(&context, &key_name, rm::ffi::REDISMODULE_WRITE);
    let safe_key = RedisKey { key };
    let key_type = unsafe {
        rm::ffi::RedisModule_KeyType.unwrap()(safe_key.key)
    };
    if unsafe {
        rm::ffi::DBType
            == rm::ffi::RedisModule_ModuleTypeGetType.unwrap()(
                safe_key.key,
            )
    } {
        let db_ptr = unsafe {
            rm::ffi::RedisModule_ModuleTypeGetValue.unwrap()(
                safe_key.key,
            ) as *mut DBKey
        };
        Ok(db_ptr)
    } else {
        Err(key_type)
    }
}

pub fn get_dbkey_from_name(
    ctx: *mut rm::ffi::RedisModuleCtx,
    name: &str,
) -> Result<DBKey, i32> {
    let dbptr = get_dbkeyptr_from_name(ctx, name)?;
    let dbkey = unsafe { dbptr.read() };
    Ok(dbkey)
}

pub unsafe fn get_ch_from_dbkeyptr(
    db: *mut DBKey,
) -> Sender<Command> {
    (*db).tx.clone()
}

pub fn reply_with_error_from_key_type(
    ctx: *mut rm::ffi::RedisModuleCtx,
    key_type: i32,
) -> i32 {
    let context = &Context::new(ctx);
    match key_type {
        rm::ffi::REDISMODULE_KEYTYPE_EMPTY => {
            ReplyWithError(context, "ERR - Error the key is empty\0")
        }
        _ => {
            let error = CStr::from_bytes_with_nul(
                rm::ffi::REDISMODULE_ERRORMSG_WRONGTYPE,
            )
            .unwrap();
            ReplyWithError(context, error.to_str().unwrap())
        }
    }
}

fn create_statement(
    db: ConcurrentConnection,
    identifier: &str,
    statement: &str,
) -> Result<MultiStatement, err::RediSQLError> {
    let stmt = MultiStatement::new(Arc::clone(&db), statement)?;
    insert_metadata(db, "statement", identifier, statement)?;
    Ok(stmt)
}

fn update_statement(
    db: &ConcurrentConnection,
    identifier: &str,
    statement: &str,
) -> Result<MultiStatement, err::RediSQLError> {
    let stmt = MultiStatement::new(Arc::clone(db), statement)?;
    update_statement_metadata(Arc::clone(db), identifier, statement)?;
    Ok(stmt)
}

fn remove_statement(
    db: &ConcurrentConnection,
    identifier: &str,
) -> Result<(), err::RediSQLError> {
    remove_statement_metadata(Arc::clone(db), identifier)
        .or_else(|e| Err(err::RediSQLError::from(e)))
}

pub fn register_function(
    context: &rm::Context,
    name: &str,
    flags: &str,
    f: extern "C" fn(
        *mut rm::ffi::RedisModuleCtx,
        *mut *mut rm::ffi::RedisModuleString,
        ::std::os::raw::c_int,
    ) -> i32,
) -> Result<(), i32> {
    let create_db: rm::ffi::RedisModuleCmdFunc = Some(f);

    if { rm::CreateCommand(context, name, create_db, flags) }
        == rm::ffi::REDISMODULE_ERR
    {
        return Err(rm::ffi::REDISMODULE_ERR);
    }
    Ok(())
}

pub fn register_function_with_keys(
    context: &rm::Context,
    name: &str,
    flags: &str,
    first_key: i32,
    last_key: i32,
    key_step: i32,
    f: extern "C" fn(
        *mut rm::ffi::RedisModuleCtx,
        *mut *mut rm::ffi::RedisModuleString,
        ::std::os::raw::c_int,
    ) -> i32,
) -> Result<(), i32> {
    let create_db: rm::ffi::RedisModuleCmdFunc = Some(f);

    if {
        rm::CreateCommandWithKeys(
            context, name, create_db, flags, first_key, last_key,
            key_step,
        )
    } == rm::ffi::REDISMODULE_ERR
    {
        return Err(rm::ffi::REDISMODULE_ERR);
    }
    Ok(())
}
pub fn register_write_function(
    ctx: &rm::Context,
    name: &str,
    f: extern "C" fn(
        *mut rm::ffi::RedisModuleCtx,
        *mut *mut rm::ffi::RedisModuleString,
        ::std::os::raw::c_int,
    ) -> i32,
) -> Result<(), i32> {
    register_function(ctx, name, "write", f)
}
