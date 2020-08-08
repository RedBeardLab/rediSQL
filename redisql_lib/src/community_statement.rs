use crate::sqlite::ffi;
use crate::sqlite::SQLiteConnection;
use crate::sqlite::StatementTrait;
use crate::sqlite::{
    get_last_error_from_db_connection, Connection, Cursor,
    SQLite3Error, SQLiteOK,
};

use std::ffi::{CStr, CString};
use std::fmt;
use std::mem;
use std::os::raw::c_char;
use std::ptr;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
enum Parameters {
    Anonymous,
    Named { index: i32 },
}

#[derive(Clone)]
pub struct MultiStatement {
    stmts: Vec<Statement>,
    db: Arc<Mutex<Connection>>,
    number_parameters: i32,
    _parameters: Vec<Vec<Parameters>>,
}

unsafe impl Send for MultiStatement {}

impl<'a> fmt::Display for MultiStatement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut buffer = String::new();
        buffer =
            self.stmts.iter().fold(buffer, |mut buffer, stmt| {
                buffer.push_str(&stmt.to_string());
                buffer.push_str("\n");
                buffer
            });
        write!(f, "{}", buffer)
    }
}

#[derive(Clone)]
pub struct Statement {
    stmt: Arc<InternalStatement>,
}

struct InternalStatement {
    stmt: ptr::NonNull<ffi::sqlite3_stmt>,
}

unsafe impl Send for InternalStatement {}
unsafe impl Sync for InternalStatement {}

impl<'a> fmt::Display for Statement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let sql = self.sql();
        writeln!(f, "{}", sql)
    }
}

impl<'a> Drop for InternalStatement {
    fn drop(&mut self) {
        unsafe {
            ffi::sqlite3_finalize(self.stmt.as_ptr());
        };
    }
}

pub fn generate_statements(
    db: Arc<Mutex<Connection>>,
    query: &str,
) -> Result<MultiStatement, SQLite3Error> {
    let raw_query = match CString::new(query.to_string()) {
        Ok(r) => r,
        Err(e) => return Err(SQLite3Error{
            code: 999,
            error_message: "Trying to create a statement with a NULL byte.".to_string(),
            error_string: format!("Find NULL byte in position {} while trying to create a statement", e.nul_position()),
        }),
    };
    let mut next_query = raw_query.as_ptr();
    let mut stmts = Vec::new();

    let arc_db = db.clone();
    let conn = arc_db.lock().unwrap();
    loop {
        let mut stmt = std::mem::MaybeUninit::uninit();

        let r = unsafe {
            ffi::sqlite3_prepare_v2(
                conn.get_db(),
                next_query,
                -1,
                stmt.as_mut_ptr(),
                &mut next_query,
            )
        };

        match r {
            ffi::SQLITE_OK => {
                let stmt = unsafe { stmt.assume_init() };
                if !stmt.is_null() {
                    let stmt = Statement::from_ptr(stmt);
                    stmts.push(stmt);
                }
                if unsafe { *next_query } == 0 {
                    let (num_parameters, parameters) =
                        count_parameters(&stmts)?;
                    return Ok(MultiStatement {
                        stmts,
                        db,
                        number_parameters: num_parameters,
                        _parameters: parameters,
                    });
                };
            }
            _ => return Err(conn.get_last_error()),
        }
    }
}

impl Statement {
    fn from_ptr(stmt: *mut ffi::sqlite3_stmt) -> Self {
        Statement {
            stmt: Arc::new(InternalStatement {
                stmt: ptr::NonNull::new(stmt).unwrap(),
            }),
        }
    }
    fn execute(
        &self,
        db: &Connection,
    ) -> Result<Cursor, SQLite3Error> {
        match unsafe { ffi::sqlite3_step(self.as_ptr()) } {
            ffi::SQLITE_OK => Ok(Cursor::OKCursor {}),
            ffi::SQLITE_DONE => {
                let modified_rows =
                    unsafe { ffi::sqlite3_changes(db.get_db()) };
                Ok(Cursor::DONECursor { modified_rows })
            }
            ffi::SQLITE_ROW => {
                let num_columns = unsafe {
                    ffi::sqlite3_column_count(self.as_ptr())
                } as i32;
                Ok(Cursor::RowsCursor {
                    stmt: self.clone(),
                    num_columns,
                    previous_status: ffi::SQLITE_ROW,
                    modified_rows: 0,
                })
            }
            _ => Err(self.get_last_error()),
        }
    }
    fn get_last_error(&self) -> SQLite3Error {
        let db = unsafe { ffi::sqlite3_db_handle(self.as_ptr()) };
        unsafe { get_last_error_from_db_connection(db) }
    }
    pub fn as_ptr(&self) -> *mut ffi::sqlite3_stmt {
        self.stmt.stmt.as_ptr()
    }
}

impl<'a> StatementTrait<'a> for Statement {
    fn execute(&self) -> Result<Cursor, SQLite3Error> {
        Err(self.get_last_error())
    }

    fn new(
        conn: Arc<Mutex<Connection>>,
        query: &str,
    ) -> Result<Self, SQLite3Error> {
        let raw_query = CString::new(query).unwrap();

        let mut stmt = std::mem::MaybeUninit::uninit();

        let conn = conn.lock().unwrap();
        let r = unsafe {
            ffi::sqlite3_prepare_v2(
                conn.get_db(),
                raw_query.as_ptr(),
                -1,
                stmt.as_mut_ptr(),
                ptr::null_mut(),
            )
        };
        let stmt = unsafe { stmt.assume_init() };
        match r {
            ffi::SQLITE_OK => Ok(Statement::from_ptr(stmt)),
            _ => Err(conn.get_last_error()),
        }
    }

    fn reset(&self) {
        unsafe {
            ffi::sqlite3_reset(self.as_ptr());
            ffi::sqlite3_clear_bindings(self.as_ptr());
        }
    }

    fn bind_texts(
        &self,
        values: &[&str],
    ) -> Result<SQLiteOK, SQLite3Error> {
        let mut index = 0;
        values
            .iter()
            .map(|value| {
                index += 1;
                self.bind_index(index, value)
            })
            .collect()
    }

    fn bind_index(
        &self,
        index: i32,
        value: &str,
    ) -> Result<SQLiteOK, SQLite3Error> {
        #[allow(non_snake_case)]
        fn SQLITE_TRANSIENT() -> ffi::sqlite3_destructor_type {
            Some(unsafe { mem::transmute(-1isize) })
        }
        match unsafe {
            ffi::sqlite3_bind_text(
                self.as_ptr(),
                index,
                value.as_ptr() as *const c_char,
                value.len() as i32,
                SQLITE_TRANSIENT(),
            )
        } {
            ffi::SQLITE_OK => Ok(SQLiteOK::OK),

            // it means that a statement requires less than $index paramenter, it is fine to just
            // shortcut it to Ok.
            ffi::SQLITE_RANGE => Ok(SQLiteOK::OK),
            _ => Err(self.get_last_error()),
        }
    }

    fn is_read_only(&self) -> bool {
        let v = unsafe { ffi::sqlite3_stmt_readonly(self.as_ptr()) };
        v != 0
    }

    fn parameters_count(&self) -> u32 {
        unsafe {
            ffi::sqlite3_bind_parameter_count(self.as_ptr()) as u32
        }
    }
    fn sql(&self) -> String {
        unsafe {
            CStr::from_ptr(ffi::sqlite3_sql(self.as_ptr()))
                .to_string_lossy()
                .into_owned()
        }
    }
}

impl<'a> StatementTrait<'a> for MultiStatement {
    fn reset(&self) {
        self.stmts.iter().for_each(StatementTrait::reset);
    }
    fn execute(&self) -> Result<Cursor, SQLite3Error> {
        let db = self.db.clone();
        let conn = db.lock().unwrap();
        debug!("Execute | Acquired db lock");
        let rows_modified_before_executing =
            unsafe { ffi::sqlite3_total_changes(conn.get_db()) };
        debug!("Execute | Read row modified before");
        match self
            .stmts
            .iter()
            .map(|stmt| stmt.execute(&conn))
            .collect()
        {
            Err(e) => Err(e),
            Ok(mut v) => {
                debug!("Execute=> Executed trains of statements");
                let rows_modified_after_executing = unsafe {
                    ffi::sqlite3_total_changes(conn.get_db())
                };
                let total_modified_rows =
                    rows_modified_after_executing
                        - rows_modified_before_executing;
                match v {
                    Cursor::DONECursor {
                        ref mut modified_rows,
                        ..
                    } => {
                        debug!("Execute=>DONECursor");
                        *modified_rows = total_modified_rows;
                    }
                    Cursor::RowsCursor {
                        ref mut modified_rows,
                        ..
                    } => {
                        *modified_rows = total_modified_rows;
                    }
                    _ => {}
                }
                Ok(v)
            }
        }
    }
    fn bind_index(
        &self,
        index: i32,
        value: &str,
    ) -> Result<SQLiteOK, SQLite3Error> {
        for stmt in &self.stmts {
            stmt.bind_index(index, value)?;
        }
        Ok(SQLiteOK::OK)
    }
    fn bind_texts(
        &self,
        values: &[&str],
    ) -> Result<SQLiteOK, SQLite3Error> {
        if values.len() != self.number_parameters as usize {
            return Err(SQLite3Error {
                code: 2021,
                error_message: String::from(
                    "RediSQL MISUSE, only \
                     parameters in the form \
                     `?NNN`, where `N` is a digit, \
                     are supported, also no gap \
                     should be present.",
                ),
                error_string: format!(
                    "Wrong number of parameters, expected {}, provided {}",
                    self.number_parameters,
                    values.len()
                ),
            });
        }

        for (i, value) in values.iter().enumerate() {
            self.bind_index(i as i32 + 1, value)?;
        }
        Ok(SQLiteOK::OK)
    }
    fn new(
        conn: Arc<Mutex<Connection>>,
        query: &str,
    ) -> Result<Self, SQLite3Error> {
        generate_statements(conn, query)
    }

    fn is_read_only(&self) -> bool {
        for stmt in &self.stmts {
            let v = stmt.is_read_only();
            if !v {
                return false;
            }
        }
        true
    }
    fn parameters_count(&self) -> u32 {
        self.stmts
            .iter()
            .map(|s| s.parameters_count())
            .max()
            .unwrap_or(0)
    }
    fn sql(&self) -> String {
        let sqls = self.stmts.iter().map(|s| s.sql());
        let n: usize = sqls.clone().map(|s| s.len()).sum();
        let mut s = String::with_capacity(n);
        for sql in sqls {
            s.push_str(&sql);
        }
        s
    }
}

fn count_parameters(
    statements: &[Statement],
) -> Result<(i32, Vec<Vec<Parameters>>), SQLite3Error> {
    let error_wrong_paramenter = SQLite3Error {
        code: 1021,
        error_message: String::from(
            "RediSQL MISUSE, only \
             parameters in the form \
             `?NNN`, where `N` is a digit, \
             are supported, also no gap \
             should be present.",
        ),
        error_string: String::from("Use of invalid parameters"),
    };

    let parameters: Result<Vec<Vec<Parameters>>, SQLite3Error> =
        statements.iter().map(|stmt| get_parameters(stmt)).collect();
    match parameters {
        Err(e) => Err(e),
        Ok(parameters) => {
            let mut discriminant: Vec<_> = parameters
                .iter()
                .flat_map(|params| {
                    params.iter().map(|p| mem::discriminant(p))
                })
                .collect();
            discriminant.dedup();
            if discriminant.len() > 1 {
                return Err(error_wrong_paramenter);
            }
            match discriminant.first() {
                None => return Ok((0, vec![])),
                Some(d)
                    if *d
                        == mem::discriminant(
                            &Parameters::Anonymous,
                        ) =>
                {
                    return Err(error_wrong_paramenter);
                }
                Some(_) => {}
            }

            let mut flatted = parameters
                .iter()
                .flat_map(|params| {
                    params.iter().map(|p| match *p {
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

fn get_parameter_name(
    stmt: &Statement,
    index: i32,
) -> Result<Option<Parameters>, SQLite3Error> {
    let parameter_name_ptr = unsafe {
        ffi::sqlite3_bind_parameter_name(stmt.as_ptr(), index)
    };
    let index_parameter = unsafe {
        ffi::sqlite3_bind_parameter_index(
            stmt.as_ptr(),
            parameter_name_ptr,
        )
    };
    match index_parameter {
        0 => Ok(None),
        n => Ok(Some(Parameters::Named { index: n })),
    }
}

fn get_parameters(
    stmt: &Statement,
) -> Result<Vec<Parameters>, SQLite3Error> {
    let total_paramenters =
        unsafe { ffi::sqlite3_bind_parameter_count(stmt.as_ptr()) }
            as usize;
    if total_paramenters == 0 {
        return Ok(vec![]);
    }
    let mut parameters = Vec::with_capacity(total_paramenters - 1);
    for i in 1..=total_paramenters {
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
    use sql::StatementTrait;
    use sqlite as sql;

    #[test]
    fn test_multiple_statements() {
        let db =
            sql::open_connection(String::from(":memory:")).unwrap();
        let statements = String::from("BEGIN; SELECT 'a'; COMMIT;");
        let stmts = generate_statements(&db, statements).unwrap();
        assert!(stmts.stmts.len() == 3);
        let statements = String::from(
            "BEGIN; SELECT 'a'; SELECT \
             'b'; SELECT 'c'; COMMIT;",
        );
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
        let db =
            sql::open_connection(String::from(":memory:")).unwrap();
        let stmts = String::from(
            "BEGIN; SELECT CASE WHEN ?1 > ?2 \
             THEN ?3 END;",
        );
        let stmt = MultiStatement::new(&db, stmts).unwrap();
        assert!(stmt.number_parameters == 3);
    }

    #[test]
    fn test_gap_parameters() {
        let db =
            sql::open_connection(String::from(":memory:")).unwrap();
        let stmts = String::from(
            "SELECT CASE WHEN ?1 > ?3 \
             THEN ?10 END;",
        );
        let stmt = MultiStatement::new(&db, stmts);
        assert!(stmt.is_ok());
        assert!(stmt.unwrap().number_parameters == 3);
    }

    #[test]
    fn test_gap_but_valid() {
        let db =
            sql::open_connection(String::from(":memory:")).unwrap();
        let stmts = String::from(
            "BEGIN; \
            SELECT CASE WHEN ?1 > ?2 THEN ?3 END; \
            SELECT CASE WHEN ?2 > ?4 THEN ?5 END;
            COMMIT;",
        );
        let stmt = MultiStatement::new(&db, stmts).unwrap();
        assert!(stmt.number_parameters == 5);
    }

    #[test]
    fn test_nameless_parameters_do_not_count() {
        let db =
            sql::open_connection(String::from(":memory:")).unwrap();
        let stmts = String::from(
            "BEGIN; \
            SELECT CASE WHEN ? > ? THEN ? END; \
            SELECT CASE WHEN ? > ? THEN ? END;
            COMMIT;",
        );
        let stmt = MultiStatement::new(&db, stmts).unwrap();
        assert!(stmt.number_parameters == 0);
    }
}
