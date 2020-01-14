use redisql_lib::redis as r;
use redisql_lib::redis::RedisKey;
use redisql_lib::redis_type::Context;

use redisql_lib::redisql_error::RediSQLError;

#[derive(Debug, PartialEq, Clone)]
pub struct CreateDB<'s> {
    name: &'s str,
    path: Option<&'s str>,
    can_exists: bool,
    must_create: bool,
}

impl<'s> CreateDB<'s> {
    pub fn parse(args: Vec<&'s str>) -> Result<Self, RediSQLError> {
        let mut args_iter = args.iter();
        args_iter.next();
        let name = match args_iter.next() {
            Some(name) => name,
            None => return Err(RediSQLError::no_database_name()),
        };
        let mut createdb = CreateDB {
            name: name,
            path: None,
            can_exists: false,
            must_create: false,
        };
        while let Some(arg) = args_iter.next() {
            let mut arg_string = String::from(*arg);
            arg_string.make_ascii_uppercase();
            match arg_string.as_str() {
                "PATH" => {
                    let path = match args_iter.next() {
                        Some(path) => path,
                        None => return Err(RediSQLError::with_code(
                            2,
                            "Provide PATH option but no PATH to use"
                                .to_string(),
                            "No PATH provided".to_string(),
                        )),
                    };
                    createdb.path = Some(path);
                }
                "CAN_EXIST" => {
                    createdb.can_exists = true;
                }
                "MUST_CREATE" => {
                    createdb.must_create = true;
                }
                _ => {}
            }
        }
        if createdb.can_exists && createdb.must_create {
            return Err(RediSQLError::with_code(3, 
                    "Provide both CAN_EXISTS and MUST_CREATE flags, they can't work together".to_string(), 
                    "Provide both CAN_EXISTS and MUST_CREATE".to_string()));
        }
        if !createdb.can_exists && !createdb.must_create {
            createdb.can_exists = true;
        }
        Ok(createdb)
    }
    pub fn key(self, ctx: &Context) -> RedisKey {
        let key_name = r::rm::RMString::new(ctx, self.name);
        let key =         r::rm::OpenKey(
                    ctx,
                    &key_name,
                    r::rm::ffi::REDISMODULE_WRITE,
                );
        RedisKey { key }
    }
}

mod test {

    #[test]
    fn simple_tag() {
        assert!(true);
    }
}
