use std::sync::mpsc::channel;
use std::thread;
use uuid::Uuid;

use parser;
use parser::CommandV2;
use redisql_lib::redis as r;
use redisql_lib::redis::{KeyTypes, RedisKey, RedisReply};
use redisql_lib::redis_type::ReplicateVerbatim;
use redisql_lib::redisql_error::RediSQLError;
use redisql_lib::sqlite::{get_arc_connection, QueryResult};

#[allow(non_snake_case)]
pub extern "C" fn CreateDB_v2(
    ctx: *mut r::rm::ffi::RedisModuleCtx,
    argv: *mut *mut r::rm::ffi::RedisModuleString,
    argc: ::std::os::raw::c_int,
) -> i32 {
    let context = r::rm::Context::new(ctx);
    let argvector = match r::create_argument(argv, argc) {
        Ok(argvector) => argvector,
        Err(mut error) => {
            return error.reply(&context);
        }
    };
    let command: parser::CreateDB =
        match parser::CommandV2::parse(argvector) {
            Ok(comm) => comm,
            Err(mut e) => return e.reply(&context),
        };
    let key = command.key(&context);
    match key.key_type() {
        KeyTypes::Empty => {
            match create_db_from_path(key, command.path) {
                Ok(mut ok) => {
                    ReplicateVerbatim(&context);
                    return ok.reply(&context);
                }
                Err(mut e) => return e.reply(&context),
            }
        }
        KeyTypes::RediSQL => {
            if command.can_exists {
                return (QueryResult::OK {}).reply(&context);
            } else {
                let mut err = RediSQLError::with_code(
                    4,
                    "Database already exists".to_string(),
                    "A database with the same name already exists but you explicitely asked to create one (using the MUST_CREATE flag).".to_string(),
                );
                return err.reply(&context);
            }
        }
        _ => {
            let mut err = RediSQLError::no_redisql_key();
            return err.reply(&context);
        }
    }
}

fn create_db_from_path(
    key: RedisKey,
    path: Option<&str>,
) -> Result<QueryResult, RediSQLError> {
    let possible_name = format!(
        "file:{}?mode=memory&cache=shared",
        Uuid::new_v4().to_simple()
    );
    let name = match path {
        None | Some(":memory") => &possible_name,
        Some(name) => name,
    };
    let connection = get_arc_connection(name);
    if connection.is_err() {
        let err = RediSQLError::with_code(
            6,
            "Error in opening database connection".to_string(),
            "It was impossible to open a new database connection, maybe we are running out of space, memory, or you request to open a file that we cannot write.".to_string(),
        );
        return Err(err);
    }
    match connection
        .and_then(r::create_metadata_table)
        .and_then(r::enable_foreign_key)
        .and_then(|rc| r::insert_path_metadata(rc, name))
    {
        Err(e) => Err(e.into()),
        Ok(rc) => {
            let (tx, rx) = channel();
            let db = r::DBKey::new_from_arc(tx, rc);
            let mut loop_data = db.loop_data.clone();
            thread::spawn(move || {
                r::listen_and_execute(&mut loop_data, &rx)
            });
            let ptr = Box::into_raw(Box::new(db));
            let type_set = unsafe {
                r::rm::ffi::RedisModule_ModuleTypeSetValue.unwrap()(
                    key.key,
                    r::rm::ffi::DBType,
                    ptr as *mut std::os::raw::c_void,
                )
            };

            match type_set {
                r::rm::ffi::REDISMODULE_OK => Ok(QueryResult::OK {}),
                r::rm::ffi::REDISMODULE_ERR => {
                    let err = RediSQLError::with_code(
                        7,
                        "Error in storing the key into redis"
                            .to_string(),
                        "Error in storing the key into redis"
                            .to_string(),
                    );
                    Err(err)
                }
                _ => {
                    let err = RediSQLError::with_code(
                        8,
                        "Unknow error in saving the key into redis"
                            .to_string(),
                        "Unknow error in saving the key into redis"
                            .to_string(),
                    );
                    Err(err)
                }
            }
        }
    }
}
