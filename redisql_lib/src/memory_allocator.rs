use std::os::raw;
use std::ptr;

use redis_type::ffi as rffi;
use sqlite::ffi as sffi;

static SQLITE_ALLOCATOR: sffi::sqlite3_mem_methods =
    sffi::sqlite3_mem_methods {
        xMalloc: Some(xMalloc),
        xFree: Some(xFree),
        xRealloc: Some(xRealloc),
        xSize: Some(xSize),
        xRoundup: Some(xRoundup),

        xInit: Some(xInit),
        xShutdown: None,
        pAppData: ptr::null_mut(),
    };

unsafe impl Sync for sffi::sqlite3_mem_methods {}

unsafe extern "C" fn xMalloc(size: raw::c_int) -> *mut raw::c_void {
    let ptr = rffi::RedisModule_Alloc.unwrap()(size as usize);
    debug!("xMalloc size: {}, result: {}", size, ptr as usize);
    ptr
}

unsafe extern "C" fn xFree(ptr: *mut raw::c_void) {
    debug!("xFree");
    rffi::RedisModule_Free.unwrap()(ptr)
}

unsafe extern "C" fn xRealloc(
    ptr: *mut raw::c_void,
    size: raw::c_int,
) -> *mut raw::c_void {
    debug!("xRealloc size: {}", size);
    rffi::RedisModule_Realloc.unwrap()(ptr, size as usize)
}

unsafe extern "C" fn xSize(ptr: *mut raw::c_void) -> raw::c_int {
    let result = ptr.offset(-1) as raw::c_int;
    debug!("xSize ptr: {}, size: {}", ptr as usize, result as usize);
    result
}

unsafe extern "C" fn xRoundup(size: raw::c_int) -> raw::c_int {
    debug!("xRoundup size: {}", size);
    size
}

unsafe extern "C" fn xInit(_ptr: *mut raw::c_void) -> raw::c_int {
    // NOP
    0
}

pub unsafe fn set_sqlite_allocator() -> Result<i32, i32> {
    match sffi::sqlite3_config(
        sffi::SQLITE_CONFIG_MALLOC,
        SQLITE_ALLOCATOR,
    ) {
        sffi::SQLITE_OK => Ok(sffi::SQLITE_OK),
        err => Err(err),
    }
}
