use std::mem;
use std::ptr;
use std::fmt;
use std::ffi::{CString, CStr};
use std::error;

use redisql_error as err;

pub use community_statement;
pub use community_statement::Statement;

#[allow(dead_code)]
#[allow(non_snake_case)]
#[allow(non_camel_case_types)]
#[allow(non_upper_case_globals)]
pub mod ffi {
    include!(concat!(env!("OUT_DIR"), "/bindings_sqlite.rs"));
}

#[derive(Clone)]
pub struct SQLite3Error {
    pub code: i32,
    pub error_message: String,
    pub error_string: String,
}

pub fn generate_sqlite3_error(conn: *mut ffi::sqlite3)
                              -> SQLite3Error {
    let error_code = unsafe { ffi::sqlite3_extended_errcode(conn) };
    let error_message = unsafe {
        CStr::from_ptr(ffi::sqlite3_errmsg(conn))
            .to_string_lossy()
            .into_owned()
    };
    let error_string = unsafe {
        CStr::from_ptr(ffi::sqlite3_errstr(error_code))
            .to_string_lossy()
            .into_owned()
    };
    SQLite3Error {
        code: error_code,
        error_message: error_message,
        error_string: error_string,
    }
}

impl fmt::Display for SQLite3Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "ERR - Error Code: {} => {} | {}",
               self.code,
               self.error_string,
               self.error_message)
    }
}

impl fmt::Debug for SQLite3Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl error::Error for SQLite3Error {
    fn description(&self) -> &str {
        self.error_message.as_str()
    }
}

impl err::RediSQLErrorTrait for SQLite3Error {}



#[derive(Clone)]
pub struct RawConnection {
    db: *mut ffi::sqlite3,
}

unsafe impl Send for RawConnection {}

impl Drop for RawConnection {
    fn drop(&mut self) {
        unsafe {
            ffi::sqlite3_close(self.db);
        }
    }
}

impl RawConnection {
    pub fn get_db(&self) -> *mut ffi::sqlite3 {
        self.db
    }
}

pub fn open_connection(path: String)
                       -> Result<RawConnection, SQLite3Error> {
    let mut db: *mut ffi::sqlite3 = unsafe { mem::uninitialized() };
    let c_path = CString::new(path).unwrap();
    let r = unsafe {
        let ptr_path = c_path.as_ptr();
        ffi::sqlite3_open_v2(ptr_path,
                             &mut db,
                             ffi::SQLITE_OPEN_CREATE |
                             ffi::SQLITE_OPEN_READWRITE,
                             ptr::null())
    };
    match r {
        ffi::SQLITE_OK => Ok(RawConnection { db: db }),
        _ => {
            return Err(generate_sqlite3_error(db));
        }
    }
}

pub trait StatementTrait<'a>: Sized {
    fn new(conn: &'a RawConnection,
           query: String)
           -> Result<Self, SQLite3Error>;
    fn reset(&self);
    fn execute(&self) -> Result<Cursor, SQLite3Error>;
    fn bind_text(&self,
                 index: i32,
                 value: String)
                 -> Result<(), SQLite3Error>;
    fn get_raw_stmt(&self) -> *mut ffi::sqlite3_stmt;
    fn to_replicate(&self) -> bool {
        false
    }
}

pub enum EntityType {
    Integer,
    Float,
    Text,
    Blob,
    Null,
}

pub enum Entity {
    Integer { int: i32 },
    Float { float: f64 },
    Text { text: String },
    Blob { blob: String },
    Null,
    OK { to_replicate: bool },
    DONE {
        modified_rows: i32,
        to_replicate: bool,
    },
}

pub type Row = Vec<Entity>;

pub enum Cursor<'a> {
    OKCursor { to_replicate: bool },
    DONECursor {
        modified_rows: i32,
        to_replicate: bool,
    },
    RowsCursor {
        num_columns: i32,
        types: Vec<EntityType>,
        previous_status: i32,
        stmt: &'a Statement<'a>,
        to_replicate: bool,
    },
}

impl<'a> Iterator for Cursor<'a> {
    type Item = Row;

    fn next(&mut self) -> Option<Self::Item> {
        match *self {
            Cursor::OKCursor { to_replicate } => {
                Some(vec![Entity::OK {to_replicate}])
            }
            Cursor::DONECursor { modified_rows, to_replicate } => {
                Some(vec![Entity::DONE {modified_rows, to_replicate}])
            }

            Cursor::RowsCursor { ref stmt,
                                 num_columns,
                                 ref types,
                                 ref mut previous_status,
                                 .. } => {
                match *previous_status {
                    ffi::SQLITE_ROW => {
                        let mut result = vec![];
                        for i in 0..num_columns {
                            let entity_value =
                                match types[i as usize] {
                                    EntityType::Integer => {
                                        let value =
                                        unsafe {
                                            ffi::sqlite3_column_int(stmt.get_raw_stmt(), i)
                                        };
                                        Entity::Integer { int: value }
                                    }
                                    EntityType::Float => {
                                        let value = unsafe { ffi::sqlite3_column_double(stmt.get_raw_stmt(), i) };
                                        Entity::Float { float: value }
                                    }
                                    EntityType::Text => {
                                        let value =
                                unsafe {
                                    CStr::from_ptr(ffi::sqlite3_column_text(stmt.get_raw_stmt(), i) as *const i8).to_string_lossy().into_owned()
                                };
                                        Entity::Text { text: value }
                                    }
                                    EntityType::Blob => {
                                        let value = 
                                unsafe { 
                                    CStr::from_ptr(ffi::sqlite3_column_blob(stmt.get_raw_stmt(), i) as *const i8).to_string_lossy().into_owned() 
                                };
                                        Entity::Blob { blob: value }
                                    }
                                    EntityType::Null => {
                                        Entity::Null {}
                                    }
                                };
                            result.push(entity_value);
                        }
                        unsafe {
                            *previous_status =
                                ffi::sqlite3_step(stmt.get_raw_stmt());
                        };
                        Some(result)
                    }
                    _ => None,
                }
            }
        }
    }
}

pub fn create_backup
    (src: &RawConnection,
     dest: &RawConnection)
     -> Result<*mut ffi::sqlite3_backup, SQLite3Error> {
    let dest_name = CString::new("main").unwrap();
    let src_name = CString::new("main").unwrap();
    let result = unsafe {
        ffi::sqlite3_backup_init(dest.db,
                                 dest_name.as_ptr(),
                                 src.db,
                                 src_name.as_ptr())
    };
    match result {
        null if null.is_null() => {
            Err(generate_sqlite3_error(dest.db))
        }
        ptr => Ok(ptr),
    }
}

pub fn backup_step(bk: *mut ffi::sqlite3_backup, steps: i32) -> i32 {
    unsafe { ffi::sqlite3_backup_step(bk, steps) }
}

pub fn backup_finish(bk: *mut ffi::sqlite3_backup) -> i32 {
    unsafe { ffi::sqlite3_backup_finish(bk) }
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
