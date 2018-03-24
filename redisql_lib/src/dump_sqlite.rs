use std::os::raw;
use std::mem;
use std::iter;
use std::sync::{Arc, Mutex};
use std::ffi::CString;

use sqlite::RawConnection;
use sqlite::ffi;
use sqlite::SQLiteConnection;

use redis as r;
use redis::ffi::{RedisModuleString, RedisModuleIO};

use redis::{LoopData, execute_query, get_dbkey_from_name,
            reply_with_error_from_key_type, RedisReply};


struct DumpIterator<'a> {
    fd: raw::c_int,
    buffer: [u8; 4096],
    iterator: Box<DoubleEndedIterator<Item = &'a str>>,
    first_chunk: &'a str,
}

impl<'a> DumpIterator<'a> {
    fn new(conn: &Arc<Mutex<RawConnection>>) -> DumpIterator {
        let db = conn.lock().unwrap();
        let buffer: [u8; 4096] = unsafe { mem::zeroed() };
        let fd = unsafe { ffi::start((*db).get_db()) };
        let iterator = Box::new(iter::empty());
        let first_chunk = "";
        DumpIterator {
            fd,
            buffer,
            iterator,
            first_chunk,
        }
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
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        match self.iterator.next() {
            Some(s) => Some(s),
            None => {
                let read_bytes =
                    unsafe {
                        ffi::read_from_pipe(self.fd,
                                            self.buffer
                                                .as_mut_ptr() as
                                            *mut raw::c_void,
                                            4096)
                    };
                match read_bytes {
                    n if n > 0 => unsafe {
                        let whole_string = String::from_utf8_unchecked(self.buffer.split_at(n as usize).0.clone().to_vec());
                        let iterator =
                            whole_string.split('\n').to_vec().iter();

                        self.iterator = Box::new(iterator);

                        let result = match self.iterator.next() {
                            None => {
                                match self.first_chunk {
                                    "" => None,
                                    s => Some(s),
                                }
                            }
                            Some(s) => {
                                let mut buffer = self.first_chunk
                                    .to_owned();
                                buffer.push_str(s);
                                Some(self.first_chunk)
                            }
                        };

                        self.first_chunk =
                            self.iterator.next_back().unwrap_or("");
                        result
                    },
                    _ => None,
                }
            }
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

    let exec = CString::new("REDISQL.EXEC.NOW").unwrap();
    let iter = DumpIterator::new(&db);
    let cmd_specifier = CString::new("sc").unwrap();
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

#[allow(non_snake_case)]
pub extern "C" fn ExecNow(ctx: *mut r::ffi::RedisModuleCtx,
                          argv: *mut *mut r::ffi::RedisModuleString,
                          argc: ::std::os::raw::c_int)
                          -> i32 {
    let (_context, argvector) = r::create_argument(ctx, argv, argc);
    match argvector.len() {
        3 => {
            match get_dbkey_from_name(ctx, argvector[1].clone()) {
                Err(key_type) => {
                    reply_with_error_from_key_type(ctx, key_type)
                }
                Ok(dbkey) => {
                    let db = dbkey.loop_data.get_db();
                    mem::forget(dbkey);
                    let result = execute_query(&db,
                                               argvector[2].clone());
                    match result {
                        Ok(res) => res.reply(ctx),
                        Err(res) => {
                            let err = res;
                            err.reply(ctx)
                        }
                    }
                }
            }
        }
        _ => {
            let error = CString::new("Wrong number of arguments, it \
                                      accepts 3")
                    .unwrap();
            unsafe {
                r::ffi::RedisModule_ReplyWithError
                    .unwrap()(ctx, error.as_ptr())
            }

        }
    }
}
