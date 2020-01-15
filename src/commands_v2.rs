use parser;
use redisql_lib::redis as r;
use redisql_lib::redis::{KeyTypes, RedisReply};
use redisql_lib::redis_type::ReplicateVerbatim;
use redisql_lib::redisql_error::RediSQLError;
use redisql_lib::sqlite::QueryResult;

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
    let command = match parser::CreateDB::parse(argvector) {
        Ok(comm) => comm,
        Err(mut e) => return e.reply(&context),
    };
    let key = command.key(&context);
    match key.key_type() {
        KeyTypes::Empty => {
            // create the actual key and return ok
        }
        KeyTypes::RediSQL => {
            if command.can_exists {
                return (QueryResult::OK {}).reply(&context);
            }
            if command.must_create {
                let mut err = RediSQLError::with_code(
                    4,
                    "Database already exists".to_string(),
                    "A database with the same name already exists but you explicitely asked to create one (using the MUST_CREATE flag).".to_string(),
                );
                return err.reply(&context);
            }
        }
        _ => {
            let mut err = RediSQLError::with_code(
                5,
                "Key does not belong to us".to_string(),
                "You are trying to work with a key that does not contains RediSQL data.".to_string(),
            );
            return err.reply(&context);
        }
    }
    ReplicateVerbatim(&context);
    (QueryResult::OK {}).reply(&context)
}
