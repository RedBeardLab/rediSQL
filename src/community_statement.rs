use std::mem;
use std::ptr;
use std::fmt;
use std::ffi::{CString, CStr};

use sqlite::ffi;

use sqlite::StatementTrait;
use sqlite::{SQLite3Error, Cursor, EntityType, RawConnection};
use sqlite::generate_sqlite3_error;

#[cfg(feature = "pro")]
use replication;

pub struct Statement<'a> {
    stmt: *mut ffi::sqlite3_stmt,
    conn: &'a RawConnection,
}

impl<'a> fmt::Display for Statement<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let sql = unsafe {
            CStr::from_ptr(ffi::sqlite3_sql(self.stmt))
                .to_string_lossy()
                .into_owned()
        };
        write!(f, "{}", sql)
    }
}

impl<'a> Drop for Statement<'a> {
    fn drop(&mut self) {
        unsafe {
            ffi::sqlite3_finalize(self.stmt);
        }
    }
}

impl<'a> StatementTrait<'a> for Statement<'a> {
    fn new(conn: &'a RawConnection,
           query: String)
           -> Result<Statement, SQLite3Error> {
        let raw_query = CString::new(query).unwrap();

        let mut stmt: *mut ffi::sqlite3_stmt =
            unsafe { mem::uninitialized() };

        let r = unsafe {
            ffi::sqlite3_prepare_v2(conn.get_db(),
                                    raw_query.as_ptr(),
                                    -1,
                                    &mut stmt,
                                    ptr::null_mut())
        };
        match r {
            ffi::SQLITE_OK => {
                Ok(Statement {
                    stmt: stmt,
                    conn: conn,
                })
            }
            _ => Err(generate_sqlite3_error(conn.get_db())),
        }
    }

    fn reset(&self) {
        unsafe {
            ffi::sqlite3_reset(self.stmt);
            ffi::sqlite3_clear_bindings(self.stmt);
        }
    }

    fn execute(&self) -> Result<Cursor, SQLite3Error> {
        match unsafe { ffi::sqlite3_step(self.stmt) } {
            ffi::SQLITE_OK => {
                Ok(Cursor::OKCursor {
                    to_replicate: self.to_replicate(),
                })
            }
            ffi::SQLITE_DONE => {
                let modified_rows = unsafe {
                    ffi::sqlite3_changes(self.conn.get_db())
                };
                Ok(Cursor::DONECursor {
                    modified_rows: modified_rows,
                    to_replicate: self.to_replicate(),
                })
            }
            ffi::SQLITE_ROW => {
                let n_columns = unsafe {
                    ffi::sqlite3_column_count(self.stmt)
                } as i32;
                let mut types: Vec<EntityType> = Vec::new();
                for i in 0..n_columns {
                    types.push(match unsafe {
                        ffi::sqlite3_column_type(self.stmt, i)
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
                    stmt: self,
                    num_columns: n_columns,
                    types: types,
                    previous_status: ffi::SQLITE_ROW,
                    to_replicate: self.to_replicate(),
                })
            }
            _ => {
            Err(generate_sqlite3_error(unsafe {
                ffi::sqlite3_db_handle(self.stmt)
            }))
        }
        }
    }

    fn bind_text(&self,
                 index: i32,
                 value: String)
                 -> Result<(), SQLite3Error> {

        #[allow(non_snake_case)]
        fn SQLITE_TRANSIENT() -> ffi::sqlite3_destructor_type {
            Some(unsafe { mem::transmute(-1isize) })
        }

        let value_c = CString::new(value).unwrap();
        match unsafe {
            ffi::sqlite3_bind_text(self.stmt,
                                   index,
                                   value_c.as_ptr(),
                                   -1,
                                   SQLITE_TRANSIENT())
        } {
            ffi::SQLITE_OK => Ok(()),
            _ => {
                let db = unsafe { ffi::sqlite3_db_handle(self.stmt) };
                Err(generate_sqlite3_error(db))
            }
        }
    }

    fn get_raw_stmt(&self) -> *mut ffi::sqlite3_stmt {
        self.stmt
    }

    #[cfg(feature = "pro")]
    fn to_replicate(&self) -> bool {
        replication::to_replicate(self)
    }
}
