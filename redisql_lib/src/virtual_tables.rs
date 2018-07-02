use redis_type::ffi as rffi;
use redis_type::{CallReply, Context};
use sqlite::ffi;
use sqlite::{SQLite3Error, SQLiteConnection, SQLITE_TRANSIENT};
use std::collections::VecDeque;
use std::ffi::{CStr, CString};
use std::os::raw;
use std::ptr;
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
#[derive(Debug)]
struct VirtualTable {
    base: ffi::sqlite3_vtab,
    ctx: Arc<Mutex<Option<Context>>>,
    columns: Vec<&'static str>,
}

impl VirtualTable {
    pub fn new(
        ctx: Arc<Mutex<Option<Context>>>,
        columns: Vec<&'static str>,
    ) -> VirtualTable {
        VirtualTable {
            base: ffi::sqlite3_vtab {
                nRef: 0,
                pModule: ptr::null(),
                zErrMsg: ptr::null_mut(),
            },
            ctx,
            columns,
        }
    }
    pub fn with_vtab<F>(void_ptr: *mut ffi::sqlite3_vtab, f: F) -> i32
    where
        F: Fn(&VirtualTable) -> i32,
    {
        let vtab = unsafe { &*(void_ptr as *mut VirtualTable) };

        f(&*vtab)
    }
    pub fn reset_context(&self) {
        *self.ctx.lock().unwrap() = None
    }
}

#[repr(C)]
#[derive(Debug)]
struct VirtualTableCursor<'vtab> {
    vtabc: ffi::sqlite3_vtab_cursor,
    vtab: &'vtab VirtualTable,
    redis_context: Arc<Mutex<Option<Context>>>,
    redis_cursor: Option<String>,
    columns: &'vtab [&'static str],
    rows: Option<VecDeque<String>>,
    expanded_row: Option<Vec<String>>,
}

impl<'vtab> Drop for VirtualTableCursor<'vtab> {
    fn drop(&mut self) {
        debug!("Dropping VirtualTableCursor")
    }
}

impl<'vtab> VirtualTableCursor<'vtab> {
    unsafe fn from_raw_vtab(
        vtab: *mut VirtualTable,
    ) -> VirtualTableCursor<'vtab> {
        VirtualTableCursor {
            vtabc: ffi::sqlite3_vtab_cursor {
                pVtab: ptr::null_mut(),
            },
            vtab: &(*vtab),
            redis_context: Arc::clone(&(*vtab).ctx),
            redis_cursor: None,
            columns: &(*vtab).columns,
            rows: None,
            expanded_row: None,
        }
    }
    //TODO remove the clone? Replace with (Ref)Cell maybe?
    fn get_cursor(&self) -> Option<String> {
        self.redis_cursor.clone()
    }
}

pub fn register_modules<Conn>(
    conn: &Arc<Mutex<Conn>>,
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
    let client_data = Arc::new(Mutex::new(None));
    let to_return = client_data.clone();
    let destructor = None;
    match unsafe {
        ffi::sqlite3_create_module_v2(
            conn,
            BRUTE_HASH_NAME.as_ptr() as *const i8,
            &BRUTE_HASH_MODULE,
            Arc::into_raw(client_data) as *mut raw::c_void,
            destructor,
        )
    } {
        ffi::SQLITE_OK => Ok(to_return),
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

fn get_table_name(column_definition: &'static str) -> &str {
    column_definition.split(' ').next().unwrap()
}

fn create_table_name(
    argc: isize,
    argv: *const *const raw::c_char,
) -> Result<(String, Vec<&'static str>), &'static str> {
    let table_name = unsafe { get_str_at_index(argv, 2)? };
    let mut table = format!("CREATE TABLE {} (", table_name);
    let mut columns: Vec<&str> =
        Vec::with_capacity((argc - 3) as usize);

    debug!("Argc: {}, columns size: {}", argc, (argc - 3));

    let index_definition = unsafe { get_str_at_index(argv, 3)? };
    debug!("Index: {}", index_definition);
    columns.push(get_table_name(index_definition));
    table.push_str(&index_definition.to_string());

    for i in 4..argc {
        let column_definition = unsafe { get_str_at_index(argv, i)? };
        debug!("Column {}: {}", i, column_definition);
        columns.push(get_table_name(column_definition));
        table.push_str(&format!(", {}", column_definition));
    }
    table.push_str(");");
    debug!("Columns: {:?}", columns);
    Ok((table, columns))
}

fn set_error(to_set: *mut *mut raw::c_char, error: &str) {
    let error = CString::new(error).unwrap();
    unsafe {
        *to_set =
            ffi::sqlite3_mprintf(error.as_ptr() as *const raw::c_char)
    };
}

// need to be sure that the context exist here and put it into pp_vtab
pub unsafe extern "C" fn create_brute_hash(
    conn: *mut ffi::sqlite3,
    aux: *mut raw::c_void,
    argc: raw::c_int,
    argv: *const *const raw::c_char,
    pp_vtab: *mut *mut ffi::sqlite3_vtab,
    pz_err: *mut *mut raw::c_char,
) -> raw::c_int {
    debug!("Creating REDISQL_TABLES_BRUTE_HASH");
    let (table_name, columns) = match create_table_name(
        argc as isize,
        argv,
    ) {
        Ok((name, columns)) => (CString::new(name).unwrap(), columns),
        Err(err) => {
            set_error(pz_err, err);
            return ffi::SQLITE_ERROR;
        }
    };
    debug!("Table: {:?}", table_name);
    if ffi::sqlite3_declare_vtab(conn, table_name.as_ptr())
        != ffi::SQLITE_OK
    {
        set_error(pz_err, "Impossible to create the vtab");
        return ffi::SQLITE_ERROR;
    }
    let redis_context: Arc<Mutex<Option<Context>>> =
        Arc::from_raw(aux as *mut Mutex<Option<Context>>);
    let vtab = Box::new(VirtualTable::new(redis_context, columns));

    *pp_vtab = Box::into_raw(vtab) as *mut ffi::sqlite3_vtab;
    ffi::SQLITE_OK
}

extern "C" fn best_index_brute_hash(
    _p_vtab: *mut ffi::sqlite3_vtab,
    index_info: *mut ffi::sqlite3_index_info,
) -> raw::c_int {
    debug!("BestIndex");
    unsafe {
        (*index_info).orderByConsumed = 0;
        (*index_info).estimatedCost = 100_000.0;
    }
    debug!("BestIndex Exit");
    ffi::SQLITE_OK
}

fn do_scan(
    redis_context: Context,
    index: &str,
    to_match: &str,
) -> CallReply {
    let scan = CString::new("SCAN").unwrap();

    // 3 Null terminated C string as argument
    let call_specifiers = CString::new("ccc").unwrap();

    let index = CString::new(index).unwrap();
    let match_keyword = CString::new("MATCH").unwrap();
    let to_match = to_match.to_string();
    let to_match = CString::new(to_match).unwrap();

    let reply = unsafe {
        rffi::RedisModule_Call.unwrap()(
            redis_context.as_ptr(),
            scan.as_ptr(),
            call_specifiers.as_ptr(),
            index.as_ptr(),
            match_keyword.as_ptr(),
            to_match.as_ptr(),
        )
    };
    unsafe { CallReply::new(reply) }
}

fn get_next_index_and_results(
    cr: &CallReply,
) -> Option<(String, VecDeque<String>)> {
    debug!("get_next_index_and_results");
    let index_cr = cr.access_array_subelement(0)?;
    let index = index_cr.access_string()?;
    let results_array = cr.access_array_subelement(1)?;
    let results_len = results_array.length()?;
    let mut results = VecDeque::with_capacity(results_len);
    for i in 0..results_len {
        let result_strs = results_array.access_array_subelement(i)?;
        let strs = result_strs.access_string()?;
        results.push_back(strs);
    }
    debug!("get_next_index_and_results Exit");
    Some((index, results))
}

fn advance_redis_cursor(
    vtab_cur: &mut VirtualTableCursor,
) -> Result<(), ()> {
    let index = match vtab_cur.get_cursor() {
        None => String::from("0"),
        Some(i) => i,
    };
    let mut matcher = String::from(vtab_cur.columns[0]);
    matcher.push_str(":*");

    let redis_context =
        vtab_cur.redis_context.lock().unwrap().unwrap();

    let cr = do_scan(redis_context, &index, &matcher);

    match get_next_index_and_results(&cr) {
        None => {
            debug!("advance_redis_cursor some error -> got None");
            return Err(());
        }
        Some((i, rows)) => {
            debug!("advance_redis_cursor i {}. rows, {:?}", i, rows);
            vtab_cur.redis_cursor = Some(i);
            vtab_cur.rows = Some(rows);
        }
    }
    Ok(())
}

extern "C" fn filter_brute_hash(
    p_vtab_cursor: *mut ffi::sqlite3_vtab_cursor,
    _idx_num: raw::c_int,
    _idx_str: *const raw::c_char,
    _argc: raw::c_int,
    _argv: *mut *mut ffi::sqlite3_value,
) -> i32 {
    debug!("Filter");

    let mut vtab_cur =
        unsafe { &mut *(p_vtab_cursor as *mut VirtualTableCursor) };

    debug!("Filter got vtab_cur");

    debug!("Filter Exit");
    match advance_redis_cursor(&mut vtab_cur) {
        Ok(_) => ffi::SQLITE_OK,
        Err(_) => ffi::SQLITE_ERROR,
    }
}

extern "C" fn next_brute_hash(
    p_vtab_cursor: *mut ffi::sqlite3_vtab_cursor,
) -> i32 {
    debug!("Next");

    let vtab_cur =
        unsafe { &mut *(p_vtab_cursor as *mut VirtualTableCursor) };

    let key = vtab_cur.rows.as_mut().unwrap().pop_front().unwrap();

    debug!("Next exploring key: {}", key);

    debug!("Next Exit");
    ffi::SQLITE_OK
}

#[no_mangle]
pub fn do_hget(
    redis_context: Context,
    obj: &str,
    key: &str,
) -> CallReply {
    let hmget = CString::new("HGET").unwrap();

    let call_specifiers = CString::new("!cc").unwrap();

    let obj = CString::new(obj).unwrap();
    let key = CString::new(key).unwrap();

    debug!("do_HGET {:?} {:?}", obj, key);

    debug!("Real one");
    let reply = unsafe {
        rffi::RedisModule_Call.unwrap()(
            redis_context.as_ptr(),
            hmget.as_ptr(),
            call_specifiers.as_ptr(),
            obj.as_ptr(),
            key.as_ptr(),
        )
    };
    unsafe { CallReply::new(reply) }
}

extern "C" fn column_brute_hash(
    p_vtab_cursor: *mut ffi::sqlite3_vtab_cursor,
    sqlite3_context: *mut ffi::sqlite3_context,
    n: i32,
) -> i32 {
    debug!("Column {}", n);

    let vtab_cur =
        unsafe { &*(p_vtab_cursor as *mut VirtualTableCursor) };

    debug!("Column get vtab");

    match vtab_cur.rows {
        None => {
            debug!("Empty rows");
            return ffi::SQLITE_ERROR;
        }
        Some(ref rows) => match rows.front() {
            None => {
                debug!("No front row");
                return ffi::SQLITE_ERROR;
            }
            Some(obj) => {
                if n == 0 {
                    let result = CString::new(obj.clone()).unwrap();
                    unsafe {
                        ffi::sqlite3_result_text(
                            sqlite3_context,
                            result.as_ptr(),
                            -1,
                            SQLITE_TRANSIENT(),
                        )
                    };
                } else {
                    let key = vtab_cur.columns[n as usize];

                    debug!(
                        "Column Getting column from {} -> {}",
                        obj, key
                    );

                    let redis_context = vtab_cur
                        .redis_context
                        .lock()
                        .unwrap()
                        .unwrap();

                    debug!("RedisContext: {:?}", redis_context);

                    let cr = do_hget(redis_context, obj, key);

                    debug!("Column cr: {:?}", cr);

                    match cr {
                        CallReply::RString { .. } => {
                            let value = cr.access_string().unwrap();
                            let value = CString::new(value).unwrap();
                            unsafe {
                                ffi::sqlite3_result_text(
                                    sqlite3_context,
                                    value.as_ptr(),
                                    -1,
                                    SQLITE_TRANSIENT(),
                                );
                            }
                        }
                        CallReply::RNull { .. } => unsafe {
                            ffi::sqlite3_result_null(sqlite3_context);
                        },
                        _ => {
                            debug!("Column getting an error");
                            return ffi::SQLITE_ERROR;
                        }
                    }
                }
            }
        },
    }

    debug!("Column Exit");
    ffi::SQLITE_OK
}

extern "C" fn disconnect_brute_hash(
    p_vtab: *mut ffi::sqlite3_vtab,
) -> i32 {
    debug!("Disconnect");
    let result = VirtualTable::with_vtab(p_vtab, |ref vtab| -> i32 {
        vtab.reset_context();
        ffi::SQLITE_OK
    });
    debug!("Disconnect Exit");
    result
}

extern "C" fn open_brute_hash(
    p_vtab: *mut ffi::sqlite3_vtab,
    p_vtab_cursor: *mut *mut ffi::sqlite3_vtab_cursor,
) -> i32 {
    debug!("Open");

    unsafe {
        let vtab_cur = Box::new(VirtualTableCursor::from_raw_vtab(
            p_vtab as *mut VirtualTable,
        ));
        *p_vtab_cursor =
            Box::into_raw(vtab_cur) as *mut ffi::sqlite3_vtab_cursor
    };
    debug!("Open Exit");
    ffi::SQLITE_OK
}

extern "C" fn close_brute_hash(
    p_vtab_cursor: *mut ffi::sqlite3_vtab_cursor,
) -> i32 {
    debug!("Close");
    unsafe {
        Box::from_raw(p_vtab_cursor as *mut VirtualTableCursor)
    };
    debug!("Close Exit");
    ffi::SQLITE_OK
}

extern "C" fn eof_brute_hash(
    p_vtab_cursor: *mut ffi::sqlite3_vtab_cursor,
) -> i32 {
    debug!("EOF");

    let mut vtab_cur =
        unsafe { &mut *(p_vtab_cursor as *mut VirtualTableCursor) };

    debug!("vtab_cur begin: {:?}", vtab_cur);
    let result = match vtab_cur.rows.as_ref().unwrap().len() {
        0 => match vtab_cur.get_cursor().as_ref() {
            None => true,
            Some(index) => match index.as_ref() {
                "0" => true,
                _ => {
                    debug!("EOF advancing the cursor");
                    let _ = advance_redis_cursor(&mut vtab_cur);
                    // advance_redis_cursor does not guarantee that
                    // there are more element to
                    // analyze
                    match vtab_cur.rows.as_ref().unwrap().len() {
                        0 => true,
                        _ => false,
                    }
                }
            },
        },
        _ => false,
    };

    debug!("vtab_cur end: {:?}", vtab_cur);
    debug!("EOF Exit");
    result as i32
}

extern "C" fn rowid_brute_hash(
    p_vtab_cursor: *mut ffi::sqlite3_vtab_cursor,
    row_id: *mut i64,
) -> i32 {
    debug!("Rowid");

    let vtab_cur =
        unsafe { &*(p_vtab_cursor as *mut VirtualTableCursor) };

    match vtab_cur.rows {
        None => {
            debug!("Empty rows");
            return ffi::SQLITE_ERROR;
        }
        Some(ref rows) => {
            match rows.front() {
                None => {
                    debug!("No front row");
                    return ffi::SQLITE_ERROR;
                }
                Some(obj) => match obj.split(':').nth(1) {
                    None => {
                        debug!("Error in formatting of the keys");
                        return ffi::SQLITE_ERROR;
                    }
                    Some(index) => match index.parse::<i64>() {
                        Err(_) => {
                            debug!("Impossible to parse index {} into i64", index);
                            return ffi::SQLITE_ERROR;
                        }
                        Ok(idx) => unsafe {
                            *row_id = idx;
                        },
                    },
                },
            }
        }
    }

    debug!("Rowid Exit");
    ffi::SQLITE_OK
}

extern "C" fn rename_brute_hash(
    _p_vtab: *mut ffi::sqlite3_vtab,
    _new: *const raw::c_char,
) -> i32 {
    ffi::SQLITE_OK
}
