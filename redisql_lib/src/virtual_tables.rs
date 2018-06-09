use redis_type::Context;
use redisql_error::RediSQLError;
use sqlite::ffi;
use sqlite::ffi::{sqlite3_create_module_v2, sqlite3_vtab};
use sqlite::{SQLite3Error, SQLiteConnection};
use std::ffi::{CStr, CString};
use std::os::raw;
use std::ptr;
use std::str::Utf8Error;
use std::sync::{Arc, Mutex};

#[repr(C)]
struct VirtualTable {
    base: sqlite3_vtab,
    ctx: Context,
}

impl VirtualTable {
    pub fn new(ctx: Context) -> VirtualTable {
        VirtualTable {
            base: sqlite3_vtab {
                nRef: 0,
                pModule: ptr::null(),
                zErrMsg: ptr::null_mut(),
            },
            ctx: ctx,
        }
    }
}

#[repr(C)]
struct VirtualTableCursor {
    vtab: *mut sqlite3_vtab,
    redis_cursor: Option<i64>,
}

pub fn register_modules<Conn>(
    conn: Arc<Mutex<Conn>>,
) -> Result<(), SQLite3Error>
where
    Conn: SQLiteConnection + Sized,
{
    let conn = conn.lock().unwrap();
    match register_module_vtabs(conn.get_db()) {
        ffi::SQLITE_OK => Ok(()),
        _ => Err(conn.get_last_error()),
    }
}

fn register_module_vtabs(conn: *mut ffi::sqlite3) -> i32 {
    let name = CString::new("REDISQL.TABLES.BRUTE.HASH").unwrap();
    let module = ffi::sqlite3_module {
        iVersion: 1,
        xBegin: None,
        xBestIndex: Some(best_index_brute_hash),
        xClose: None,
        xColumn: Some(column_brute_hash),
        xCommit: None,
        xConnect: Some(create_brute_hash),
        xCreate: Some(create_brute_hash),
        xDestroy: None,
        xDisconnect: None,
        xEof: None,
        xFilter: Some(filter_brute_hash),
        xFindFunction: None,
        xNext: Some(next_brute_hash),
        xOpen: None,
        xRelease: None,
        xRename: None,
        xRollback: None,
        xRollbackTo: None,
        xRowid: None,
        xSavepoint: None,
        xSync: None,
        xUpdate: None,
    };
    let client_data = ptr::null_mut();
    let destructor = None;
    unsafe {
        sqlite3_create_module_v2(
            conn,
            name.as_ptr(),
            &module,
            client_data,
            destructor,
        )
    }
}

unsafe fn get_str_at_index(
    argv: *const *const raw::c_char,
    index: isize,
) -> Result<&'static str, &'static str> {
    match CStr::from_ptr(*argv.offset(index)).to_str() {
        Ok(s) => Ok(s),
        Err(_) => Err("Not UTF8 input string"),
    }
}

fn create_table_name(
    argc: isize,
    argv: *const *const raw::c_char,
) -> Result<String, &'static str> {
    let table_name = unsafe { get_str_at_index(argv, 2)? };
    let mut table = format!("CREATE TABLE {} (ID STRING", table_name);
    for i in 3..argc {
        let column_name = unsafe { get_str_at_index(argv, i)? };
        table.push_str(&format!(", {}", column_name));
    }
    table.push_str(");");
    Ok(table)
}

fn set_error(to_set: *mut *mut raw::c_char, error: &str) {
    let error = CString::new(error).unwrap();
    unsafe {
        *to_set =
            ffi::sqlite3_mprintf(error.as_ptr() as *const raw::c_char)
    };
}

extern "C" fn create_brute_hash(
    conn: *mut ffi::sqlite3,
    aux: *mut raw::c_void,
    argc: raw::c_int,
    argv: *const *const raw::c_char,
    pp_vtab: *mut *mut ffi::sqlite3_vtab,
    pz_err: *mut *mut raw::c_char,
) -> raw::c_int {
    let table_name = match create_table_name(argc as isize, argv) {
        Ok(name) => CString::new(name).unwrap(),
        Err(err) => {
            set_error(pz_err, err);
            return ffi::SQLITE_ERROR;
        }
    };
    if unsafe { ffi::sqlite3_declare_vtab(conn, table_name.as_ptr()) }
        != ffi::SQLITE_OK
    {
        set_error(pz_err, "Impossible to create the vtab");
        return ffi::SQLITE_ERROR;
    }
    ffi::SQLITE_OK
}

extern "C" fn best_index_brute_hash(
    p_vtab: *mut ffi::sqlite3_vtab,
    index_info: *mut ffi::sqlite3_index_info,
) -> raw::c_int {
    unsafe {
        (*index_info).orderByConsumed = 0;
        (*index_info).estimatedCost = 100_000.0;
    }
    ffi::SQLITE_OK
}

extern "C" fn filter_brute_hash(
    p_vtab_cursor: *mut ffi::sqlite3_vtab_cursor,
    idx_num: raw::c_int,
    idx_str: *const raw::c_char,
    argc: raw::c_int,
    argv: *mut *mut ffi::sqlite3_value,
) -> i32 {
    ffi::SQLITE_OK
}

extern "C" fn next_brute_hash(
    p_vtab_cursor: *mut ffi::sqlite3_vtab_cursor,
) -> i32 {
    ffi::SQLITE_OK
}

extern "C" fn column_brute_hash(
    p_vtab_cursor: *mut ffi::sqlite3_vtab_cursor,
    sqlite_context: *mut ffi::sqlite3_context,
    N: i32,
) -> i32 {
    ffi::SQLITE_OK
}
