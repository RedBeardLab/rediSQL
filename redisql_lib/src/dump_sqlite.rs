use std;
use std::os::raw;
use std::mem;
use std::sync::{Arc, Mutex, MutexGuard};
use std::ffi::CString;

use sqlite::RawConnection;
use sqlite::ffi;
use sqlite::SQLiteConnection;

use redis as r;
use redis::ffi::{RedisModuleString, RedisModuleIO};

use redis::Loop;
use redis::LoopData;



struct DumpIterator<'a> {
    fd: raw::c_int,
    db: MutexGuard<'a, RawConnection>,
    buffer: [u8; 4096],
}

impl<'a> DumpIterator<'a> {
    fn new(conn: &'a Arc<Mutex<RawConnection>>) -> DumpIterator<'a> {
        let db = conn.lock().unwrap();
        let buffer: [u8; 4096] = unsafe { mem::zeroed() };
        let fd = unsafe { ffi::start((*db).get_db()) };
        DumpIterator { fd, db, buffer }
    }
}

impl<'a> Drop for DumpIterator<'a> {
    fn drop(&mut self) {
        unsafe {
            ffi::close_read_pipe(self.fd);
        }
    }
}

impl<'a> Iterator for DumpIterator<'a> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        let read_bytes = unsafe {
            ffi::read_from_pipe(self.fd,
                                self.buffer.as_mut_ptr() as
                                *mut raw::c_void,
                                4096)
        };
        match read_bytes {
            n if n > 0 => unsafe {
                Some(String::from_utf8_unchecked(self.buffer.split_at(n as usize).0.clone().to_vec()))
            },
            _ => None,
        }
    }
}

#[allow(non_snake_case)]
pub extern "C" fn WriteAOF(aof: *mut RedisModuleIO,
                           key: *mut RedisModuleString,
                           value: *mut raw::c_void) {
    let dbkey: Box<r::DBKey> =
        unsafe { Box::from_raw(value as *mut r::DBKey) };

    let db = dbkey.loop_data.get_db().clone();

    let create_db = CString::new("REDISQL.CREATE_DB").unwrap();
    let create_specifier = CString::new("s").unwrap();
    unsafe {
        r::ffi::RedisModule_EmitAOF.unwrap()(aof,
                                             create_db.as_ptr(),
                                             create_specifier
                                                 .as_ptr(),
                                             key);
    }

    let exec = CString::new("REDISQL.EXEC").unwrap();
    let iter = DumpIterator::new(&db);
    let cmd_specifier = CString::new("sc").unwrap();
    let cmd_key = unsafe {
        r::ffi::RedisModule_StringPtrLen
            .unwrap()(key, std::ptr::null_mut())
    };
    for s in iter {
        for line in s.split("\n")
                .filter(|l| !l.is_empty())
                .map(|l| CString::new(l).unwrap()) {
            unsafe {
                r::ffi::RedisModule_EmitAOF
                    .unwrap()(aof,
                              exec.as_ptr(),
                              cmd_specifier.as_ptr(),
                              key,
                              line.as_ptr());
            }
        }

    }
}
