use std::mem;
use std::ptr;
use std::fmt;
use std::ffi::{CString, CStr};

use std::os::raw::c_void;

#[allow(dead_code)]
#[allow(non_snake_case)]
#[allow(non_camel_case_types)]
#[allow(non_upper_case_globals)]
mod ffi {
    include!(concat!(env!("OUT_DIR"), "/bindings_sqlite.rs"));
}

pub struct SQLite3Error {
    pub code: i32,
    pub error_message: String,
    pub error_string: String,
}

fn generate_sqlite3_error(conn: *mut ffi::sqlite3) -> SQLite3Error {
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

#[derive(Clone)]
pub struct RawConnection {
    db: *mut ffi::sqlite3,
}

unsafe impl Send for RawConnection {}

#[derive(Clone)]
pub struct Statement {
    stmt: *mut ffi::sqlite3_stmt,
}

impl Drop for Statement {
    fn drop(&mut self) {
        let sql = unsafe { CStr::from_ptr(ffi::sqlite3_sql(self.stmt)) };
        println!("DROPPED STATETMENT: {:?}", sql);
        unsafe {
            ffi::sqlite3_finalize(self.stmt);
        }
    }
}

impl Drop for RawConnection {
    fn drop(&mut self) {
        unsafe {
            ffi::sqlite3_close(self.db);
        }
    }
}


pub fn create_statement(conn: &RawConnection,
                        query: String)
                        -> Result<Statement, SQLite3Error> {

    let raw_query = CString::new(query).unwrap();

    let mut stmt: *mut ffi::sqlite3_stmt =
        unsafe { mem::uninitialized() };
    // let mut db = conn.db;
    let r = unsafe {
        ffi::sqlite3_prepare_v2(conn.db,
                                raw_query.as_ptr(),
                                -1,
                                &mut stmt,
                                ptr::null_mut())
    };
    match r {
        ffi::SQLITE_OK => Ok(Statement { stmt: stmt }),
        _ => Err(generate_sqlite3_error(conn.db)),
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

pub enum Cursor {
    OKCursor,
    DONECursor,
    RowsCursor {
        stmt: Statement,
        num_columns: i32,
        types: Vec<EntityType>,
        previous_status: i32,
    },
}

unsafe extern "C" fn aaa(_: *mut c_void) {
    println!("zzz");
}

pub fn SQLITE_TRANSIENT() -> ffi::sqlite3_destructor_type {
    Some(unsafe { mem::transmute(-1isize) })
}

pub fn bind_text(db: &RawConnection,
                 stmt: &Statement,
                 index: i32,
                 value: String)
                 -> Result<(), SQLite3Error> {

    let value_c = CString::new(value).unwrap();

    match unsafe {
        ffi::sqlite3_bind_text(stmt.stmt,
                               index,
                               value_c.as_ptr(),
                               -1,
                               SQLITE_TRANSIENT())
    } {
        ffi::SQLITE_OK => Ok(()),
        _ => Err(generate_sqlite3_error(db.db)),
    }
}

pub fn execute_statement(stmt: Statement)
                         -> Result<Cursor, SQLite3Error> {

    match unsafe { ffi::sqlite3_step(stmt.stmt) } {
        ffi::SQLITE_OK => Ok(Cursor::OKCursor),
        ffi::SQLITE_DONE => Ok(Cursor::DONECursor),
        ffi::SQLITE_ROW => {
            let n_columns =
                unsafe { ffi::sqlite3_column_count(stmt.stmt) } as i32;
            let mut types: Vec<EntityType> = Vec::new();
            for i in 0..n_columns {
                types.push(match unsafe {
                    ffi::sqlite3_column_type(stmt.stmt, i)
                } {
                    ffi::SQLITE_INTEGER => EntityType::Integer,
                    ffi::SQLITE_FLOAT => EntityType::Float,
                    ffi::SQLITE_TEXT => EntityType::Text,
                    ffi::SQLITE_BLOB => EntityType::Blob,
                    ffi::SQLITE_NULL => EntityType::Null,
                    _ => EntityType::Null,
                })
            }
            Ok(Cursor::RowsCursor {
                stmt: stmt,
                num_columns: n_columns,
                types: types,
                previous_status: ffi::SQLITE_ROW,
            })
        }
        _ => {
            return Err(generate_sqlite3_error(unsafe {
                ffi::sqlite3_db_handle(stmt.stmt)
            }));
        }
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
    OK,
    DONE,
}


pub type Row = Vec<Entity>;

impl Iterator for Cursor {
    type Item = Row;

    fn next(&mut self) -> Option<Self::Item> {
        match *self {
            Cursor::OKCursor => Some(vec![Entity::OK]),
            Cursor::DONECursor => Some(vec![Entity::DONE]),

            Cursor::RowsCursor { ref stmt,
                                 num_columns,
                                 ref types,
                                 ref mut previous_status } => {
                match *previous_status {
                    ffi::SQLITE_ROW => {
                        let mut result = vec![];
                        for i in 0..num_columns {
                            let entity_value = match types[i as usize] {
                                EntityType::Integer => {
                                    let value =
                                        unsafe {
                                            ffi::sqlite3_column_int(stmt.stmt, i)
                                        };
                                    Entity::Integer { int: value }
                                }
                                EntityType::Float => {
                                    let value = unsafe { ffi::sqlite3_column_double(stmt.stmt, i) };
                                    Entity::Float { float: value }
                                }
                                EntityType::Text => {
                                    let value =
                                unsafe {
                                    CStr::from_ptr(ffi::sqlite3_column_text(stmt.stmt, i) as *const i8).to_string_lossy().into_owned()
                                };
                                    Entity::Text { text: value }
                                }
                                EntityType::Blob => {
                                    let value = 
                                unsafe { 
                                    CStr::from_ptr(ffi::sqlite3_column_blob(stmt.stmt, i) as *const i8).to_string_lossy().into_owned() 
                                };
                                    Entity::Blob { blob: value }
                                }
                                EntityType::Null => Entity::Null {},
                            };
                            result.push(entity_value);
                        }
                        unsafe {
                            *previous_status =
                                ffi::sqlite3_step(stmt.stmt);
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
        null if null.is_null() => Err(generate_sqlite3_error(dest.db)),
        ptr => Ok(ptr),
    }
}

pub fn errcode(conn: RawConnection) -> i32 {
    unsafe { ffi::sqlite3_errcode(conn.db) }
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
