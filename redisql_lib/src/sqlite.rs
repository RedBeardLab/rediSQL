use std::cell::RefCell;
use std::error;
use std::ffi::{CStr, CString};
use std::fmt;
use std::iter::FromIterator;
use std::mem;
use std::os::raw::c_char;
use std::ptr;
use std::sync::{Arc, Mutex};

use redisql_error as err;

use community_statement::Statement;

use redis_type::Context;

#[allow(dead_code)]
#[allow(non_snake_case)]
#[allow(non_camel_case_types)]
#[allow(non_upper_case_globals)]
pub mod ffi {
    #![allow(clippy)]
    include!(concat!(
        env!("OUT_DIR"),
        "/bindings_sqlite.rs"
    ));
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
    pub code: i32,
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

impl error::Error for SQLite3Error {
    fn description(&self) -> &str {
        self.error_message.as_str()
    }
}

impl err::RediSQLErrorTrait for SQLite3Error {}

#[derive(Clone)]
pub struct RawConnection {
    db: *mut ffi::sqlite3,
    modified_rows: i32,
}

unsafe impl Send for RawConnection {}

impl Drop for RawConnection {
    fn drop(&mut self) {
        unsafe {
            ffi::sqlite3_close(self.db);
        }
    }
}

pub trait SQLiteConnection {
    fn get_db(&self) -> *mut ffi::sqlite3;
    fn get_last_error(&self) -> SQLite3Error;
}

impl SQLiteConnection for RawConnection {
    fn get_db(&self) -> *mut ffi::sqlite3 {
        self.db
    }
    fn get_last_error(&self) -> SQLite3Error {
        let error_code =
            unsafe { ffi::sqlite3_extended_errcode(self.get_db()) };
        let error_message = unsafe {
            CStr::from_ptr(ffi::sqlite3_errmsg(self.get_db()))
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
            error_message,
            error_string,
        }
    }
}

impl RawConnection {
    pub fn open_connection(
        path: &str,
    ) -> Result<RawConnection, SQLite3Error> {
        let mut db: *mut ffi::sqlite3 =
            unsafe { mem::uninitialized() };
        let c_path = CString::new(path).unwrap();
        let r = unsafe {
            let ptr_path = c_path.as_ptr();
            ffi::sqlite3_open_v2(
                ptr_path,
                &mut db,
                ffi::SQLITE_OPEN_CREATE | ffi::SQLITE_OPEN_READWRITE,
                ptr::null(),
            )
        };
        let rc = RawConnection {
            db,
            modified_rows: 0,
        };
        match r {
            ffi::SQLITE_OK => Ok(rc),
            _ => Err(rc.get_last_error()),
        }
    }
    pub fn from_db_handler(db: *mut ffi::sqlite3) -> RawConnection {
        RawConnection {
            db,
            modified_rows: 0,
        }
    }
}

pub fn get_arc_connection(
    path: &str,
) -> Result<Arc<Mutex<RawConnection>>, SQLite3Error> {
    let raw = RawConnection::open_connection(path)?;
    Ok(Arc::new(Mutex::new(raw)))
}

pub trait StatementTrait<'a>: Sized {
    fn new(
        conn: Arc<Mutex<RawConnection>>,
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
    fn get_raw_stmt(&self) -> *mut ffi::sqlite3_stmt;
    fn is_read_only(&self) -> bool {
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

// TODO XXX explore it is possible to change these String into &str
pub enum Entity {
    Integer {
        int: i32,
    },
    Float {
        float: f64,
    },
    Text {
        text: String,
    },
    Blob {
        blob: String,
    },

    Null,
    OK {
        to_replicate: bool,
    },
    DONE {
        modified_rows: i32,
        to_replicate: bool,
    },
}

pub type Row = Vec<Entity>;

pub enum Cursor<'a> {
    OKCursor,
    DONECursor {
        modified_rows: i32,
    },
    RowsCursor {
        num_columns: i32,
        previous_status: i32,
        stmt: &'a Statement,
        modified_rows: i32,
    },
    /* ADD empty cursor, it will be the easiest (and maybe
     * cleaner?) way to manage empty return statements */
}

impl<'a> FromIterator<Cursor<'a>> for Cursor<'a> {
    fn from_iter<I: IntoIterator<Item = Cursor<'a>>>(
        cursors: I,
    ) -> Cursor<'a> {
        let mut modified = 0;
        let mut last: Option<Cursor<'a>> = None;
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
                modified_rows: 0,
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

impl<'a> Iterator for Cursor<'a> {
    type Item = Row;

    fn next(&mut self) -> Option<Self::Item> {
        match *self {
            Cursor::OKCursor {} => Some(vec![
                Entity::OK {
                    to_replicate: true,
                },
            ]),
            Cursor::DONECursor { modified_rows } => Some(vec![
                Entity::DONE {
                    modified_rows,
                    to_replicate: true,
                },
            ]),

            Cursor::RowsCursor {
                stmt,
                num_columns,
                ref mut previous_status,
                ..
            } => match *previous_status {
                ffi::SQLITE_ROW => {
                    let mut result = vec![];
                    for i in 0..num_columns {
                        let entity_value = match get_entity_type(
                            stmt.get_raw_stmt(),
                            i,
                        ) {
                            EntityType::Integer => {
                                let value = unsafe {
                                    ffi::sqlite3_column_int(
                                        stmt.get_raw_stmt(),
                                        i,
                                    )
                                };
                                debug!("Got integer: {:?}", value);
                                Entity::Integer { int: value }
                            }
                            EntityType::Float => {
                                let value = unsafe {
                                    ffi::sqlite3_column_double(
                                        stmt.get_raw_stmt(),
                                        i,
                                    )
                                };
                                debug!("Got float: {:?}", value);
                                Entity::Float { float: value }
                            }
                            EntityType::Text => {
                                let value = unsafe {
                                    CStr::from_ptr(
                                        ffi::sqlite3_column_text(
                                            stmt.get_raw_stmt(),
                                            i,
                                        )
                                            as *const c_char,
                                    ).to_string_lossy()
                                        .into_owned()
                                };
                                debug!("Got text: {:?}", value);
                                Entity::Text { text: value }
                            }
                            EntityType::Blob => {
                                let value = unsafe {
                                    CStr::from_ptr(
                                        ffi::sqlite3_column_blob(
                                            stmt.get_raw_stmt(),
                                            i,
                                        )
                                            as *const c_char,
                                    ).to_string_lossy()
                                        .into_owned()
                                };
                                debug!("Got blob: {:?}", value);
                                Entity::Blob { blob: value }
                            }
                            EntityType::Null => {
                                debug!("Got null");
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
            },
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
    src: &RawConnection,
    dest: &RawConnection,
) -> Result<Backup, SQLite3Error> {
    let dest_name = CString::new("main").unwrap();
    let src_name = CString::new("main").unwrap();
    let result = unsafe {
        ffi::sqlite3_backup_init(
            dest.db,
            dest_name.as_ptr(),
            src.db,
            src_name.as_ptr(),
        )
    };
    match result {
        null if null.is_null() => Err(dest.get_last_error()),
        ptr => Ok(Backup { bk: ptr }),
    }
}

// TODO XXX finish work here
#[allow(non_snake_case)]
pub unsafe fn BackupStep(bk: &Backup, steps: i32) -> i32 {
    ffi::sqlite3_backup_step(bk.as_ptr(), steps)
}

#[allow(non_snake_case)]
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
