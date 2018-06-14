use redis_type::Context;
use redisql_error::RediSQLError;
use sqlite::ffi;
use sqlite::{SQLite3Error, SQLiteConnection};
use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::os::raw;
use std::ptr;
use std::str::Utf8Error;
use std::sync::{Arc, Mutex};

static BRUTE_HASH_MODULE: ffi::sqlite3_module = ffi::sqlite3_module {
    iVersion: 1,
    xBegin: None,
    xBestIndex: Some(best_index_brute_hash),
    xClose: Some(close_brute_hash),
    xColumn: Some(column_brute_hash),
    xCommit: None,
    xConnect: Some(create_brute_hash),
    xCreate: Some(create_brute_hash),
    xDestroy: Some(disconnect_brute_hash),
    xDisconnect: Some(disconnect_brute_hash),
    xEof: Some(eof_brute_hash),
    xFilter: Some(filter_brute_hash),
    xFindFunction: None,
    xNext: Some(next_brute_hash),
    xOpen: Some(open_brute_hash),
    xRelease: None,
    xRename: Some(rename_brute_hash),
    xRollback: None,
    xRollbackTo: None,
    xRowid: Some(rowid_brute_hash),
    xSavepoint: None,
    xSync: None,
    xUpdate: None,
};

static BRUTE_HASH_NAME: &[u8] = b"REDISQL_TABLES_BRUTE_HASH\0";

#[repr(C)]
struct VirtualTable {
    base: ffi::sqlite3_vtab,
    ctx: Arc<Mutex<Option<Context>>>,
}

impl VirtualTable {
    pub fn new(ctx: Arc<Mutex<Option<Context>>>) -> VirtualTable {
        VirtualTable {
            base: ffi::sqlite3_vtab {
                nRef: 0,
                pModule: ptr::null(),
                zErrMsg: ptr::null_mut(),
            },
            ctx: ctx,
        }
    }
    pub fn from_raw_ptr(
        void_ptr: *mut ffi::sqlite3_vtab,
    ) -> VirtualTable {
        let vtab: Box<VirtualTable> =
            unsafe { Box::from_raw(void_ptr as *mut VirtualTable) };
        *vtab
    }
    pub fn with_vtab<F>(void_ptr: *mut ffi::sqlite3_vtab, f: F) -> i32
    where
        F: Fn(&VirtualTable) -> i32,
    {
        let vtab: Box<VirtualTable> =
            unsafe { Box::from_raw(void_ptr as *mut VirtualTable) };

        let result = f(&*vtab);
        Box::leak(vtab);
        result
    }
    pub fn reset_context(&self) {
        *self.ctx.lock().unwrap() = None
    }
}

#[repr(C)]
struct VirtualTableCursor {
    vtab: ffi::sqlite3_vtab_cursor,
    redis_cursor: Option<i64>,
}

impl VirtualTableCursor {
    fn new() -> VirtualTableCursor {
        VirtualTableCursor {
            vtab: ffi::sqlite3_vtab_cursor {
                pVtab: ptr::null_mut(),
            },
            redis_cursor: None,
        }
    }
}

pub fn register_modules<Conn>(
    conn: Arc<Mutex<Conn>>,
) -> Result<Arc<Mutex<Option<Context>>>, SQLite3Error>
where
    Conn: SQLiteConnection + Sized,
{
    debug!("Registering modules");
    let conn = conn.lock().unwrap();
    match register_module_vtabs(conn.get_db()) {
        Ok(context) => Ok(context),
        _ => Err(conn.get_last_error()),
    }
}

fn register_module_vtabs(
    conn: *mut ffi::sqlite3,
) -> Result<Arc<Mutex<Option<Context>>>, ()> {
    debug!("Registering REDISQL_TABLES_BRUTE_HASH");
    let client_data = Box::new(Arc::new(Mutex::new(None)));
    let to_return = client_data.clone();
    let destructor = None;
    match unsafe {
        ffi::sqlite3_create_module_v2(
            conn,
            BRUTE_HASH_NAME.as_ptr() as *const i8,
            &BRUTE_HASH_MODULE,
            Box::into_raw(client_data) as *mut raw::c_void,
            destructor,
        )
    } {
        ffi::SQLITE_OK => Ok(*to_return),
        _ => {
            println!("Error in creating the vtab");
            Err(())
        }
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

// need to be sure that the context exist here and put it into pp_vtab
#[no_mangle]
pub extern "C" fn create_brute_hash(
    conn: *mut ffi::sqlite3,
    aux: *mut raw::c_void,
    argc: raw::c_int,
    argv: *const *const raw::c_char,
    pp_vtab: *mut *mut ffi::sqlite3_vtab,
    pz_err: *mut *mut raw::c_char,
) -> raw::c_int {
    debug!("Creating REDISQL_TABLES_BRUTE_HASH");
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
    let redis_context: Box<Arc<Mutex<Option<Context>>>> = unsafe {
        Box::from_raw(aux as *mut Arc<Mutex<Option<Context>>>)
    };
    let vtab = Box::new(VirtualTable::new(*redis_context));
    unsafe {
        *pp_vtab = Box::into_raw(vtab) as *mut ffi::sqlite3_vtab
    };
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

// need the context here, one way or another
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

extern "C" fn disconnect_brute_hash(
    p_vtab: *mut ffi::sqlite3_vtab,
) -> i32 {
    debug!("Disconnect");
    VirtualTable::with_vtab(p_vtab, |ref vtab| -> i32 {
        vtab.reset_context();
        ffi::SQLITE_OK
    })
}

extern "C" fn open_brute_hash(
    p_vtab: *mut ffi::sqlite3_vtab,
    p_vtab_cursor: *mut *mut ffi::sqlite3_vtab_cursor,
) -> i32 {
    debug!("Open");
    let vtab_cur = Box::new(VirtualTableCursor::new());
    unsafe {
        *p_vtab_cursor =
            Box::into_raw(vtab_cur) as *mut ffi::sqlite3_vtab_cursor
    };
    ffi::SQLITE_OK
}

extern "C" fn close_brute_hash(
    p_vtab_cursor: *mut ffi::sqlite3_vtab_cursor,
) -> i32 {
    unsafe {
        Box::from_raw(p_vtab_cursor as *mut VirtualTableCursor)
    };
    ffi::SQLITE_OK
}

extern "C" fn eof_brute_hash(
    p_vtab_cursor: *mut ffi::sqlite3_vtab_cursor,
) -> i32 {
    true as i32
}

extern "C" fn rowid_brute_hash(
    p_vtab_cursor: *mut ffi::sqlite3_vtab_cursor,
    rowId: *mut i64,
) -> i32 {
    ffi::SQLITE_OK
}

extern "C" fn rename_brute_hash(
    p_vtab: *mut ffi::sqlite3_vtab,
    new: *const raw::c_char,
) -> i32 {
    ffi::SQLITE_OK
}
