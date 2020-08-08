use std::convert::TryFrom;
use std::ffi::{CStr, CString};
use std::fmt;
use std::iter::FromIterator;
use std::mem;
use std::os::raw::c_char;
use std::ptr;
use std::sync::{Arc, Mutex};

use crate::redisql_error as err;

use crate::community_statement::Statement;

#[allow(dead_code)]
#[allow(non_snake_case)]
#[allow(non_camel_case_types)]
#[allow(non_upper_case_globals)]
#[allow(unknown_lints)]
pub mod ffi {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/bindings_sqlite.rs"));
}

pub enum SQLiteOK {
    OK,
}

impl FromIterator<SQLiteOK> for SQLiteOK {
    fn from_iter<I: IntoIterator<Item = SQLiteOK>>(
        _iter: I,
    ) -> SQLiteOK {
        SQLiteOK::OK
    }
}

#[derive(Clone)]
pub struct SQLite3Error {
    pub code: u32,
    pub error_message: String,
    pub error_string: String,
}

impl fmt::Display for SQLite3Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "ERR - Error Code: {} => {} | {}",
            self.code, self.error_string, self.error_message
        )
    }
}

impl fmt::Debug for SQLite3Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl err::RediSQLErrorTrait for SQLite3Error {}

struct RawConnection {
    conn: *mut ffi::sqlite3,
    flags: i32,
}

impl RawConnection {
    pub fn new_serialize(path: &str) -> Result<Self, SQLite3Error> {
        Self::new(path, Self::fullmutex_flags())
    }
    pub fn new_multithread(path: &str) -> Result<Self, SQLite3Error> {
        Self::new(path, Self::nomutex_flags())
    }
    fn new(path: &str, flags: i32) -> Result<Self, SQLite3Error> {
        let mut conn = std::mem::MaybeUninit::uninit();
        let c_path = CString::new(path).unwrap();
        let r = unsafe {
            let ptr_path = c_path.as_ptr();
            ffi::sqlite3_open_v2(
                ptr_path,
                conn.as_mut_ptr(),
                flags,
                ptr::null(),
            )
        };
        let conn = unsafe { conn.assume_init() };
        match r {
            ffi::SQLITE_OK => Ok(RawConnection { conn, flags }),
            _ => Err(unsafe {
                get_last_error_from_db_connection(conn)
            }),
        }
    }
    fn nomutex_flags() -> i32 {
        ffi::SQLITE_OPEN_URI
            | ffi::SQLITE_OPEN_CREATE
            | ffi::SQLITE_OPEN_READWRITE
            | ffi::SQLITE_OPEN_NOMUTEX
    }
    fn fullmutex_flags() -> i32 {
        ffi::SQLITE_OPEN_URI
            | ffi::SQLITE_OPEN_CREATE
            | ffi::SQLITE_OPEN_READWRITE
            | ffi::SQLITE_OPEN_FULLMUTEX
    }
    pub fn is_multithread(&self) -> bool {
        (self.flags & ffi::SQLITE_OPEN_NOMUTEX) != 0
    }
    pub fn is_serialized(&self) -> bool {
        (self.flags & ffi::SQLITE_OPEN_FULLMUTEX) != 0
    }
}

impl Drop for RawConnection {
    fn drop(&mut self) {
        unsafe {
            ffi::sqlite3_close(self.conn);
        }
    }
}

pub trait SQLiteConnection {
    fn get_db(&self) -> *mut ffi::sqlite3;
    fn get_last_error(&self) -> SQLite3Error;
}

impl SQLiteConnection for Connection {
    fn get_db(&self) -> *mut ffi::sqlite3 {
        self.db.conn
    }
    fn get_last_error(&self) -> SQLite3Error {
        unsafe { get_last_error_from_db_connection(self.get_db()) }
    }
}

pub unsafe fn get_last_error_from_db_connection(
    conn: *mut ffi::sqlite3,
) -> SQLite3Error {
    let error_code = ffi::sqlite3_extended_errcode(conn);
    let error_message = CStr::from_ptr(ffi::sqlite3_errmsg(conn))
        .to_string_lossy()
        .into_owned();
    let error_string =
        CStr::from_ptr(ffi::sqlite3_errstr(error_code))
            .to_string_lossy()
            .into_owned();
    SQLite3Error {
        code: error_code as u32,
        error_message,
        error_string,
    }
}

pub struct Connection {
    db: RawConnection,
    pub path: String,
}

impl Connection {
    pub fn open_connection(
        path: &str,
    ) -> Result<Connection, SQLite3Error> {
        let db = RawConnection::new_multithread(path)?;

        Ok(Connection {
            db,
            path: String::from(path),
        })
    }
    pub fn duplicate_connection(
        &self,
    ) -> Result<Connection, SQLite3Error> {
        let cn1 = RawConnection::new_serialize(&self.path)?;
        let cn1 = Connection {
            db: cn1,
            path: String::from(&self.path),
        };

        Ok(cn1)
    }
    pub fn is_serialized(&self) -> bool {
        self.db.is_serialized()
    }
    pub fn is_multithread(&self) -> bool {
        self.db.is_multithread()
    }
}

pub fn get_arc_connection(
    path: &str,
) -> Result<Arc<Mutex<Connection>>, SQLite3Error> {
    let raw = Connection::open_connection(path)?;
    Ok(Arc::new(Mutex::new(raw)))
}

pub trait StatementTrait<'a>: Sized {
    fn new(
        conn: Arc<Mutex<Connection>>,
        query: &str,
    ) -> Result<Self, SQLite3Error>;
    fn reset(&self);
    fn execute(&self) -> Result<Cursor, SQLite3Error>;
    fn bind_texts(
        &self,
        values: &[&str],
    ) -> Result<SQLiteOK, SQLite3Error>;
    fn bind_index(
        &self,
        index: i32,
        value: &str,
    ) -> Result<SQLiteOK, SQLite3Error>;
    fn is_read_only(&self) -> bool {
        false
    }
    fn parameters_count(&self) -> u32;
    fn sql(&self) -> String;
}

enum EntityType {
    Integer,
    Float,
    Text,
    Blob,
    Null,
}

// TODO XXX explore it is possible to change these String into &str
// and then use Copy instead of Clone
#[derive(Clone, Debug)]
pub enum Entity {
    Integer { int: i64 },
    Float { float: f64 },
    Text { text: String },
    Blob { blob: String },

    Null,
    OK {},
    DONE { modified_rows: i32 },
}

impl Entity {
    pub fn new(stmt: &Statement, i: i32) -> Entity {
        match get_entity_type(stmt.as_ptr(), i) {
            EntityType::Integer => {
                let int = unsafe {
                    ffi::sqlite3_column_int64(stmt.as_ptr(), i)
                };
                Entity::Integer { int }
            }
            EntityType::Float => {
                let value = unsafe {
                    ffi::sqlite3_column_double(stmt.as_ptr(), i)
                };
                Entity::Float { float: value }
            }
            EntityType::Text => {
                let value = unsafe {
                    CStr::from_ptr(ffi::sqlite3_column_text(
                        stmt.as_ptr(),
                        i,
                    )
                        as *const c_char)
                    .to_string_lossy()
                    .into_owned()
                };
                Entity::Text { text: value }
            }

            EntityType::Blob => {
                let value = unsafe {
                    CStr::from_ptr(ffi::sqlite3_column_blob(
                        stmt.as_ptr(),
                        i,
                    )
                        as *const c_char)
                    .to_string_lossy()
                    .into_owned()
                };
                Entity::Blob { blob: value }
            }
            EntityType::Null => Entity::Null {},
        }
    }
}

pub enum Cursor {
    OKCursor,
    DONECursor {
        modified_rows: i32,
    },
    RowsCursor {
        num_columns: i32,
        previous_status: i32,
        stmt: Statement,
        modified_rows: i32,
    },
    /* ADD empty cursor, it will be the easiest (and maybe
     * cleaner?) way to manage empty return statements */
}

impl<'a> FromIterator<Cursor> for Cursor {
    fn from_iter<I: IntoIterator<Item = Cursor>>(
        cursors: I,
    ) -> Cursor {
        let mut modified = 0;
        let mut last: Option<Cursor> = None;
        for cursor in cursors {
            match cursor {
                Cursor::OKCursor {} => {
                    debug!("FromIterator => OKCursor");
                }
                Cursor::DONECursor { modified_rows } => {
                    debug!("FromIterator => DONECursor");
                    modified += modified_rows;
                }
                Cursor::RowsCursor { .. } => {
                    debug!("FromIterator => RowsCursor");
                }
            }
            last = Some(cursor);
        }
        match last {
            None => Cursor::DONECursor {
                modified_rows: modified,
            },
            Some(cursor) => cursor,
        }
    }
}

fn get_entity_type(
    stmt: *mut ffi::sqlite3_stmt,
    i: i32,
) -> EntityType {
    let entity_type = unsafe { ffi::sqlite3_column_type(stmt, i) };
    match entity_type {
        ffi::SQLITE_INTEGER => EntityType::Integer,
        ffi::SQLITE_FLOAT => EntityType::Float,
        ffi::SQLITE_TEXT => EntityType::Text,
        ffi::SQLITE_BLOB => EntityType::Blob,
        ffi::SQLITE_NULL => EntityType::Null,
        _ => EntityType::Null,
    }
}

pub enum QueryResult {
    OK {},
    DONE {
        modified_rows: i32,
    },
    Array {
        names: Vec<String>,
        types: Vec<&'static str>,
        array: Vec<Entity>,
    },
    Stream {
        stream: String,
        first_id: String,
        last_id: String,
        size: u32,
    },
}

fn type_to_string(t: i32) -> &'static str {
    match t {
        ffi::SQLITE_INTEGER => "INT",
        ffi::SQLITE_FLOAT => "FLOAT",
        ffi::SQLITE_TEXT => "TEXT",
        ffi::SQLITE_BLOB => "BLOB",
        ffi::SQLITE_NULL => "NULL",
        _ => "NULL",
    }
}

impl QueryResult {
    pub fn from_cursor_before(
        mut cursor: Cursor,
        timeout: std::time::Instant,
    ) -> Result<Self, err::RediSQLError> {
        let now = std::time::Instant::now();
        if now > timeout {
            return Err(err::RediSQLError::timeout());
        }
        match cursor {
            Cursor::OKCursor {} => Ok(QueryResult::OK {}),
            Cursor::DONECursor { modified_rows } => {
                Ok(QueryResult::DONE { modified_rows })
            }
            Cursor::RowsCursor {
                ref stmt,
                num_columns,
                ref mut previous_status,
                ..
            } => {
                let mut now = std::time::Instant::now();
                if now > timeout {
                    return Err(err::RediSQLError::timeout());
                }
                let mut result = vec![];
                let mut names =
                    Vec::with_capacity(num_columns as usize);
                let mut types =
                    Vec::with_capacity(num_columns as usize);
                for i in 0..num_columns {
                    let name = unsafe {
                        CStr::from_ptr(ffi::sqlite3_column_name(
                            stmt.as_ptr(),
                            i,
                        ))
                        .to_string_lossy()
                        .into_owned()
                    };
                    names.push(name);
                    let t = type_to_string(unsafe {
                        ffi::sqlite3_column_type(stmt.as_ptr(), i)
                    });
                    types.push(t);
                }
                while *previous_status == ffi::SQLITE_ROW {
                    now = std::time::Instant::now();
                    if now > timeout {
                        return Err(err::RediSQLError::timeout());
                    }
                    for i in 0..num_columns {
                        let entity_value = Entity::new(stmt, i);
                        result.push(entity_value);
                    }
                    unsafe {
                        *previous_status =
                            ffi::sqlite3_step(stmt.as_ptr());
                    };
                }
                match *previous_status {
                     ffi::SQLITE_INTERRUPT => {
                        Err(err::RediSQLError::new("Query Interrupted".to_string(), "The query was interrupted, most likely because it runs out of time.".to_string()))
                    },
                    _ => Ok(QueryResult::Array {
                        names,
                        array: result,
                        types,
                    }),
                }
            }
        }
    }

    pub fn from_cursor_before_with_header(
        mut cursor: Cursor,
        timeout: std::time::Instant,
    ) -> Result<Self, err::RediSQLError> {
        let now = std::time::Instant::now();
        if now > timeout {
            return Err(err::RediSQLError::timeout());
        }
        match cursor {
            Cursor::OKCursor {} => Ok(QueryResult::OK {}),
            Cursor::DONECursor { modified_rows } => {
                Ok(QueryResult::DONE { modified_rows })
            }
            Cursor::RowsCursor {
                ref stmt,
                num_columns,
                ref mut previous_status,
                ..
            } => {
                let mut now = std::time::Instant::now();
                if now > timeout {
                    return Err(err::RediSQLError::timeout());
                }
                let mut result = vec![];
                let mut names =
                    Vec::with_capacity(num_columns as usize);
                let mut types =
                    Vec::with_capacity(num_columns as usize);
                for i in 0..num_columns {
                    let name = unsafe {
                        CStr::from_ptr(ffi::sqlite3_column_name(
                            stmt.as_ptr(),
                            i,
                        ))
                        .to_string_lossy()
                        .into_owned()
                    };
                    names.push(name.clone());
                    result.push(Entity::Text { text: name });
                }
                for i in 0..num_columns {
                    let t = type_to_string(unsafe {
                        ffi::sqlite3_column_type(stmt.as_ptr(), i)
                    });
                    types.push(t);
                    result.push(Entity::Text {
                        text: t.to_string(),
                    });
                }
                while *previous_status == ffi::SQLITE_ROW {
                    now = std::time::Instant::now();
                    if now > timeout {
                        return Err(err::RediSQLError::timeout());
                    }
                    for i in 0..num_columns {
                        let entity_value = Entity::new(stmt, i);
                        result.push(entity_value);
                    }
                    unsafe {
                        *previous_status =
                            ffi::sqlite3_step(stmt.as_ptr());
                    };
                }
                match *previous_status {
                     ffi::SQLITE_INTERRUPT => {
                        Err(err::RediSQLError::new("Query Interrupted".to_string(), "The query was interrupted, most likely because it runs out of time.".to_string()))
                    },
                    _ => Ok(QueryResult::Array {
                        names,
                        array: result,
                        types,
                    }),
                }
            }
        }
    }
}

impl TryFrom<Cursor> for QueryResult {
    type Error = err::RediSQLError;
    fn try_from(
        mut cursor: Cursor,
    ) -> Result<QueryResult, Self::Error> {
        match cursor {
            Cursor::OKCursor {} => Ok(QueryResult::OK {}),
            Cursor::DONECursor { modified_rows } => {
                Ok(QueryResult::DONE { modified_rows })
            }

            Cursor::RowsCursor {
                ref stmt,
                num_columns,
                ref mut previous_status,
                ..
            } => {
                let mut result = vec![];
                let mut names =
                    Vec::with_capacity(num_columns as usize);
                let mut types =
                    Vec::with_capacity(num_columns as usize);
                for i in 0..num_columns {
                    let name = unsafe {
                        CStr::from_ptr(ffi::sqlite3_column_name(
                            stmt.as_ptr(),
                            i,
                        ))
                        .to_string_lossy()
                        .into_owned()
                    };
                    names.push(name);
                    let t = type_to_string(unsafe {
                        ffi::sqlite3_column_type(stmt.as_ptr(), i)
                    });
                    types.push(t);
                }
                while *previous_status == ffi::SQLITE_ROW {
                    for i in 0..num_columns {
                        let entity_value = Entity::new(stmt, i);
                        result.push(entity_value);
                    }
                    unsafe {
                        *previous_status =
                            ffi::sqlite3_step(stmt.as_ptr());
                    };
                }
                match *previous_status {
                     ffi::SQLITE_INTERRUPT => {
                        Err(Self::Error::new("Query Interrupted".to_string(), "The query was interrupted, most likely because it runs out of time.".to_string()))
                    },
                    _ => Ok(QueryResult::Array {
                        names,
                        array: result,
                        types,
                    }),
                }
            }
        }
    }
}

pub struct Backup {
    bk: *mut ffi::sqlite3_backup,
}

impl Backup {
    fn as_ptr(&self) -> *mut ffi::sqlite3_backup {
        self.bk
    }
}

pub fn create_backup(
    src: &Connection,
    dest: &Connection,
) -> Result<Backup, SQLite3Error> {
    let dest_name = CString::new("main").unwrap();
    let src_name = CString::new("main").unwrap();
    let result = unsafe {
        ffi::sqlite3_backup_init(
            dest.get_db(),
            dest_name.as_ptr(),
            src.get_db(),
            src_name.as_ptr(),
        )
    };
    match result {
        null if null.is_null() => Err(dest.get_last_error()),
        ptr => Ok(Backup { bk: ptr }),
    }
}

// TODO XXX finish work here
#[allow(non_snake_case, clippy::missing_safety_doc)]
pub unsafe fn BackupStep(bk: &Backup, steps: i32) -> i32 {
    ffi::sqlite3_backup_step(bk.as_ptr(), steps)
}

#[allow(non_snake_case, clippy::missing_safety_doc)]
pub unsafe fn BackupFinish(bk: &Backup) -> i32 {
    ffi::sqlite3_backup_finish(bk.as_ptr())
}

pub fn backup_step_is_ok(result: i32) -> bool {
    result == ffi::SQLITE_OK
}

pub fn backup_step_should_retry(result: i32) -> bool {
    result == ffi::SQLITE_BUSY || result == ffi::SQLITE_LOCKED
}

pub fn backup_should_step_again(result: i32) -> bool {
    backup_step_is_ok(result) || backup_step_should_retry(result)
}

pub fn backup_complete_with_done(result: i32) -> bool {
    result == ffi::SQLITE_DONE
}

pub fn disable_global_memory_statistics() {
    unsafe {
        ffi::sqlite3_config(ffi::SQLITE_CONFIG_MEMSTATUS, 0);
    }
}

#[allow(non_snake_case)]
pub fn SQLITE_TRANSIENT() -> ffi::sqlite3_destructor_type {
    Some(unsafe { mem::transmute(-1isize) })
}
