use redisql_lib::redis as r;
use redisql_lib::redis::RedisKey;
use redisql_lib::redis_type::Context;

use redisql_lib::redisql_error::RediSQLError;

#[derive(Debug, PartialEq, Clone)]
pub struct CreateDB<'s> {
    name: &'s str,
    pub path: Option<&'s str>,
    pub can_exists: bool,
}

impl<'s> CommandV2<'s> for CreateDB<'s> {
    fn parse(args: Vec<&'s str>) -> Result<Self, RediSQLError> {
        let mut args_iter = args.iter();
        args_iter.next();
        let name = match args_iter.next() {
            Some(name) => name,
            None => return Err(RediSQLError::no_database_name()),
        };
        let mut createdb = CreateDB {
            name: name,
            path: None,
            can_exists: true,
        };
        let mut can_exists_flag = false;
        let mut must_create_flag = false;
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
                    can_exists_flag = true;
                    createdb.can_exists = true;
                }
                "MUST_CREATE" => {
                    must_create_flag = true;
                    createdb.can_exists = false;
                }
                _ => {}
            }
        }
        if can_exists_flag && must_create_flag {
            return Err(RediSQLError::with_code(3,
                    "Provide both CAN_EXISTS and MUST_CREATE flags, they can't work together".to_string(),
                    "Provide both CAN_EXISTS and MUST_CREATE".to_string()));
        }
        Ok(createdb)
    }

    fn database(&self) -> &str {
        self.name
    }
}

#[derive(Debug, PartialEq, Clone)]
enum ToExecute<'s> {
    Query(&'s str),
    Statement(&'s str),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Exec<'s> {
    database: &'s str,
    connection: Option<&'s str>,
    into: Option<&'s str>,
    read_only: bool,
    now: bool,
    to_execute: Option<ToExecute<'s>>,
    args: Option<Vec<&'s str>>,
}

impl<'s> CommandV2<'s> for Exec<'s> {
    fn parse(args: Vec<&'s str>) -> Result<Self, RediSQLError> {
        let mut args_iter = args.iter();
        args_iter.next();
        let database = match args_iter.next() {
            Some(db) => db,
            None => return Err(RediSQLError::no_database_name()),
        };
        let mut exec = Exec {
            database,
            connection: None,
            into: None,
            read_only: false,
            now: false,
            to_execute: None,
            args: None,
        };
        while let Some(arg) = args_iter.next() {
            let mut arg_string = String::from(*arg);
            arg_string.make_ascii_uppercase();
            match arg_string.as_str() {
                "QUERY" => match exec.to_execute {
                    Some(ToExecute::Statement(_)) => {
                        return Err(
                            RediSQLError::both_statement_and_query(),
                        );
                    }
                    Some(ToExecute::Query(_)) => {
                        return Err(RediSQLError::with_code(
                            13,
                            "Impossible to know which query should be executed".to_string(),
                            "Provided QUERY twice".to_string(),
                        ));
                    }
                    None => {
                        let query = match args_iter.next() {
                            Some(q) => q,
                            None => {
                                return Err(RediSQLError::with_code(
                                    9,
                                    "Provided the QUERY keyword but not the query to execute".to_string(),
                                    "No query provided".to_string(),
                                ))
                            }
                        };
                        exec.to_execute =
                            Some(ToExecute::Query(query));
                    }
                },
                "STATEMENT" => match exec.to_execute {
                    Some(ToExecute::Query(_)) => {
                        return Err(
                            RediSQLError::both_statement_and_query(),
                        );
                    }
                    Some(ToExecute::Statement(_)) => {
                        return Err(RediSQLError::with_code(
                            14,
                            "Impossible to know which statement should be executed".to_string(),
                            "Provided STATEMENT twice".to_string(),
                        ));
                    }
                    None => {
                        let stmt = match args_iter.next() {
                            Some(s) => s,
                            None => {
                                return Err(RediSQLError::with_code(
                                    10,
                                    "Provided the STATEMENT keyword but not the statement to execute".to_string(),
                                    "No statement provided"
                                        .to_string(),
                                ))
                            }
                        };
                        exec.to_execute =
                            Some(ToExecute::Statement(stmt));
                    }
                },
                "READ_ONLY" => exec.read_only = true,
                "NOW" => exec.now = true,
                "INTO" => {
                    let stream = match args_iter.next() {
                        Some(s) => s,
                        None => {
                            return Err(RediSQLError::with_code(
                                11,
                                "Provided the INTO keyword without providing which stream we should use".to_string(),
                                "No stream provided".to_string(),
                            ))
                        }
                    };
                    exec.into = Some(stream);
                }
                "ARGS" => {
                    let (size, _) = args_iter.size_hint();
                    let mut args = Vec::with_capacity(size);
                    while let Some(arg) = args_iter.next() {
                        args.push(*arg);
                    }
                    exec.args = Some(args);
                }
                _ => {}
            }
        }
        Ok(exec)
    }
    fn database(&self) -> &str {
        self.database
    }
}

pub trait CommandV2<'s> {
    fn parse(args: Vec<&'s str>) -> Result<Self, RediSQLError>
    where
        Self: std::marker::Sized;
    fn database(&self) -> &str;
    fn key(&self, ctx: &Context) -> RedisKey {
        let key_name = self.database();
        let key_name = r::rm::RMString::new(ctx, key_name);
        let key = r::rm::OpenKey(
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
