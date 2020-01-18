use redisql_lib::redis as r;
use redisql_lib::redis::Command;
use redisql_lib::redis::RedisKey;
use redisql_lib::redis_type::BlockedClient;
use redisql_lib::redis_type::Context;
use redisql_lib::redisql_error::RediSQLError;

use crate::common::CommandV2;

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

impl Exec<'static> {
    pub fn get_command(
        self,
        timeout: std::time::Instant,
        client: BlockedClient,
    ) -> Command {
        match self.to_execute {
            Some(ToExecute::Query(q)) => match self.read_only {
                true => todo!(),
                false => Command::Exec {
                    query: q,
                    timeout,
                    client,
                },
            },
            Some(ToExecute::Statement(_)) => todo!(),
            None => todo!(),
        }
    }
    pub fn is_now(&self) -> bool {
        self.now
    }
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
