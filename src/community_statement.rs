use std::mem;
use std::ptr;
use std::fmt;
use std::ffi::{CString, CStr};

use sqlite::ffi;

use sqlite::StatementTrait;
use sqlite::{SQLite3Error, Cursor, RawConnection, SQLiteOK};
use sqlite::generate_sqlite3_error;

#[cfg(feature = "pro")]
use replication;

#[derive(Clone, Debug)]
enum Parameters {
    Anonymous,
    Named { index: i32 },
}

pub struct MultiStatement<'a> {
    stmts: Vec<Statement<'a>>,
    conn: &'a RawConnection,
    number_parameters: i32,
    _parameters: Vec<Vec<Parameters>>,
}

impl<'a> fmt::Display for MultiStatement<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut buffer = String::new();
        buffer = self.stmts
            .iter()
            .fold(buffer, |mut buffer, stmt| {
                buffer.push_str(&stmt.to_string());
                buffer.push_str("\n");
                buffer
            });
        write!(f, "{}", buffer)
    }
}

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
        write!(f, "{}\n", sql)
    }
}

impl<'a> Drop for Statement<'a> {
    fn drop(&mut self) {
        unsafe {
            ffi::sqlite3_finalize(self.stmt);
        };
    }
}

pub fn generate_statements<'a>
    (conn: &'a RawConnection,
     query: String)
     -> Result<MultiStatement<'a>, SQLite3Error> {

    let raw_query = CString::new(query).unwrap();
    let mut next_query = raw_query.as_ptr();
    let mut stmts = Vec::new();

    loop {
        let mut stmt: *mut ffi::sqlite3_stmt =
            unsafe { mem::uninitialized() };

        let r = unsafe {
            ffi::sqlite3_prepare_v2(conn.get_db(),
                                    next_query,
                                    -1,
                                    &mut stmt,
                                    &mut next_query)
        };
        match r {
            ffi::SQLITE_OK => {
                let stmt = Statement {
                    stmt: stmt,
                    conn: conn,
                };
                stmts.push(stmt);
                if unsafe { *next_query } == 0 {
                    let (num_parameters, parameters) =
                        count_parameters(&stmts)?;
                    return Ok(MultiStatement {
                                  stmts: stmts,
                                  conn: conn,
                                  number_parameters: num_parameters,
                                  _parameters: parameters,
                              });
                }
            }
            _ => return Err(generate_sqlite3_error(conn.get_db())),
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
                let modified_rows =
                    unsafe {
                        ffi::sqlite3_changes(self.conn.get_db())
                    };
                Ok(Cursor::DONECursor {
                       modified_rows: modified_rows,
                       to_replicate: self.to_replicate(),
                   })
            }
            ffi::SQLITE_ROW => {
                let n_columns =
                    unsafe {
                        ffi::sqlite3_column_count(self.stmt)
                    } as i32;
                Ok(Cursor::RowsCursor {
                       stmt: self,
                       num_columns: n_columns,
                       previous_status: ffi::SQLITE_ROW,
                       to_replicate: self.to_replicate(),
                       modified_rows: 0,
                   })
            }
            _ => {
                Err(generate_sqlite3_error(
                    unsafe { ffi::sqlite3_db_handle(self.stmt) },
                ))
            }
        }
    }

    fn bind_texts(&self,
                  values: Vec<String>)
                  -> Result<SQLiteOK, SQLite3Error> {
        let mut index = 0;
        values
            .iter()
            .map(|value| {
                     index += 1;
                     self.bind_index(index, &value)
                 })
            .collect()
    }

    fn bind_index(&self,
                  index: i32,
                  value: &String)
                  -> Result<SQLiteOK, SQLite3Error> {

        #[allow(non_snake_case)]
        fn SQLITE_TRANSIENT() -> ffi::sqlite3_destructor_type {
            Some(unsafe { mem::transmute(-1isize) })
        }
        let value_c = CString::new(value.clone()).unwrap();
        match unsafe {
                  ffi::sqlite3_bind_text(self.stmt,
                                         index,
                                         value_c.as_ptr(),
                                         -1,
                                         SQLITE_TRANSIENT())
              } {
            ffi::SQLITE_OK => Ok(SQLiteOK::OK),
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

impl<'a> StatementTrait<'a> for MultiStatement<'a> {
    fn reset(&self) {
        self.stmts.iter().map(|stmt| stmt.reset()).count();
    }
    fn execute(&self) -> Result<Cursor, SQLite3Error> {
        let rows_modified_before_executing =
            unsafe { ffi::sqlite3_total_changes(self.conn.get_db()) };
        match self.stmts.iter().map(|stmt| stmt.execute()).collect() {
            Err(e) => Err(e),
            Ok(mut v) => {
                let rows_modified_after_executing = unsafe {
                    ffi::sqlite3_total_changes(self.conn.get_db())
                };
                let total_modified_rows =
                    rows_modified_after_executing -
                    rows_modified_before_executing;
                match v {
                    Cursor::DONECursor {
                        ref mut modified_rows, ..
                    } => {
                        *modified_rows = total_modified_rows;
                    }
                    Cursor::RowsCursor {
                        ref mut modified_rows, ..
                    } => {
                        *modified_rows = total_modified_rows;
                    }
                    _ => {}
                }
                Ok(v)
            }
        }
    }
    fn bind_index(&self,
                  index: i32,
                  value: &String)
                  -> Result<SQLiteOK, SQLite3Error> {
        for stmt in &self.stmts {
            stmt.bind_index(index, value)?;
        }
        Ok(SQLiteOK::OK)

    }
    fn bind_texts(&self,
                  values: Vec<String>)
                  -> Result<SQLiteOK, SQLite3Error> {
        if values.len() != self.number_parameters as usize {
            return Err(SQLite3Error {
                           code: 2021,
                           error_message: String::from("RediSQL MISUSE, only \
                                     parameters in the form \
                                     `?NNN`, where `N` is a digit, \
                                     are supported, also no gap \
                                     should be present."),
                           error_string:
                               format!("Wrong number of parameters, expected {}, provided {}",
                                       self.number_parameters,
                                       values.len()),
                       });
        }

        let mut i = 0;
        for value in values {
            i += 1;
            self.bind_index(i, &value)?;
        }
        Ok(SQLiteOK::OK)
    }
    fn new(conn: &'a RawConnection,
           query: String)
           -> Result<MultiStatement, SQLite3Error> {
        generate_statements(conn, query)
    }
    fn get_raw_stmt(&self) -> *mut ffi::sqlite3_stmt {
        self.stmts[0].stmt
    }
}

fn count_parameters<'a>
    (statements: &Vec<Statement<'a>>)
     -> Result<(i32, Vec<Vec<Parameters>>), SQLite3Error> {
    let error_wrong_paramenter = SQLite3Error {
        code: 1021,
        error_message: String::from("RediSQL MISUSE, only \
                                     parameters in the form \
                                     `?NNN`, where `N` is a digit, \
                                     are supported, also no gap \
                                     should be present."),
        error_string: String::from("Use of invalid parameters"),
    };

    let parameters: Result<Vec<Vec<Parameters>>, SQLite3Error> =
        statements.iter().map(|stmt| get_parameters(stmt)).collect();
    match parameters {
        Err(e) => Err(e),
        Ok(parameters) => {
            let mut discriminant: Vec<_> = parameters
                .clone()
                .iter()
                .flat_map(|params| {
                              params.iter().map(|p| {
                        mem::discriminant(p)
                    })
                          })
                .collect();
            discriminant.dedup();
            if discriminant.len() > 1 {
                return Err(error_wrong_paramenter);
            }
            match discriminant.first() {
                None => return Ok((0, vec![])),
                Some(d)
                    if *d ==
                           mem::discriminant(
                            &Parameters::Anonymous,
                        ) => {
                    return Err(error_wrong_paramenter);
                }
                Some(_) => {}
            }

            let mut flatted = parameters
                .iter()
                .flat_map(|params| {
                    params
                        .iter()
                        .map(|ref p| match **p {
                                 Parameters::Anonymous => 0,
                                 Parameters::Named { index } => index,
                             })
                })
                .collect::<Vec<_>>();
            flatted.sort();
            flatted.dedup();
            let count = flatted.len() as i32;
            Ok((count, parameters))
        }
    }
}
fn get_parameter_name<'a>
    (stmt: &Statement<'a>,
     index: i32)
     -> Result<Option<Parameters>, SQLite3Error> {
    let parameter_name_ptr =
        unsafe { ffi::sqlite3_bind_parameter_name(stmt.stmt, index) };
    let index_parameter =
        unsafe {
            ffi::sqlite3_bind_parameter_index(stmt.stmt,
                                              parameter_name_ptr)
        };
    match index_parameter {
        0 => Ok(None),
        n => Ok(Some(Parameters::Named { index: n })),
    }
}

fn get_parameters<'a>(stmt: &Statement<'a>)
                      -> Result<Vec<Parameters>, SQLite3Error> {
    let total_paramenters =
        unsafe { ffi::sqlite3_bind_parameter_count(stmt.stmt) } as
        usize;
    if total_paramenters == 0 {
        return Ok(vec![]);
    }
    let mut parameters = Vec::with_capacity(total_paramenters - 1);
    for i in 1..(total_paramenters + 1) {
        let param = get_parameter_name(stmt, i as i32)?;
        match param {
            None => {}
            Some(p) => parameters.push(p),
        }
    }
    Ok(parameters)
}

#[cfg(test)]
mod test {
    use community_statement::{generate_statements, MultiStatement};
    use sqlite as sql;
    use sql::StatementTrait;

    #[test]
    fn test_multiple_statements() {
        let db = sql::open_connection(String::from(":memory:"))
            .unwrap();
        let statements = String::from("BEGIN; SELECT 'a'; COMMIT;");
        let stmts = generate_statements(&db, statements).unwrap();
        assert!(stmts.stmts.len() == 3);
        let statements = String::from("BEGIN; SELECT 'a'; SELECT \
                                       'b'; SELECT 'c'; COMMIT;");
        let stmts = generate_statements(&db, statements).unwrap();
        assert!(stmts.stmts.len() == 5);
    }

    #[test]
    fn count_inside_nested_vector() {
        let nested_vector =
            vec![vec![1, 2, 3], vec![4, 5, 6, 6], vec![1, 1, 0]];
        let ten: i32 =
            nested_vector.iter().map(|v| v.len() as i32).sum();
        assert!(ten == 10);
    }

    #[test]
    fn test_parameters_standard() {
        let db = sql::open_connection(String::from(":memory:"))
            .unwrap();
        let stmts = String::from("BEGIN; SELECT CASE WHEN ?1 > ?2 \
                                  THEN ?3 END;");
        let stmt = MultiStatement::new(&db, stmts).unwrap();
        assert!(stmt.number_parameters == 3);
    }

    #[test]
    fn test_gap_parameters() {
        let db = sql::open_connection(String::from(":memory:"))
            .unwrap();
        let stmts = String::from("SELECT CASE WHEN ?1 > ?3 \
                                  THEN ?10 END;");
        let stmt = MultiStatement::new(&db, stmts);
        assert!(stmt.is_ok());
        assert!(stmt.unwrap().number_parameters == 3);
    }

    #[test]
    fn test_gap_but_valid() {
        let db = sql::open_connection(String::from(":memory:"))
            .unwrap();
        let stmts = String::from("BEGIN; \
            SELECT CASE WHEN ?1 > ?2 THEN ?3 END; \
            SELECT CASE WHEN ?2 > ?4 THEN ?5 END;
            COMMIT;");
        let stmt = MultiStatement::new(&db, stmts).unwrap();
        assert!(stmt.number_parameters == 5);
    }

    #[test]
    fn test_nameless_parameters_do_not_count() {
        let db = sql::open_connection(String::from(":memory:"))
            .unwrap();
        let stmts = String::from("BEGIN; \
            SELECT CASE WHEN ? > ? THEN ? END; \
            SELECT CASE WHEN ? > ? THEN ? END;
            COMMIT;");
        let stmt = MultiStatement::new(&db, stmts).unwrap();
        assert!(stmt.number_parameters == 0);
    }
}
