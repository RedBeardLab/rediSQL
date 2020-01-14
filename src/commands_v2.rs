use parser;
use redisql_lib::redis as r;
use redisql_lib::redis::{KeyTypes, RedisReply};
use redisql_lib::redis_type::ReplicateVerbatim;
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
    match command.key(&context).key_type() {
        KeyTypes::Empty => {
            // create the actual key and return ok
        }
        KeyTypes::Module => {
            // must_create fails here
            // can exists work just fine
            if unsafe {
                rm::ffi::DBType
                    == rm::ffi::RedisModule_ModuleTypeGetType.unwrap()(
                        safe_key.key,
                    )
            } {
                // must_create fail
                // return ok for can_exists
            } else {
                // error key does not belog to us
            }
        }
        _ => {
            // fail, key does not belong to us
        }
    }
    ReplicateVerbatim(&context);
    (QueryResult::OK {}).reply(&context)
}
