use redisql_lib::redis::Command;
use redisql_lib::redis::ReturnMethod;
use redisql_lib::redis_type::BlockedClient;
use redisql_lib::redis_type::Context;
use redisql_lib::redis_type::RMString;
use redisql_lib::redisql_error::RediSQLError;

use crate::common::CommandV2;
use redisql_lib::redis_type::ffi::RedisModuleString;

#[derive(Debug, PartialEq, Clone)]
pub enum ToExecute<'s> {
    Command(&'s str),
    Statement(&'s str),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Exec<'s> {
    database: &'s str,
    connection: Option<&'s str>,
    into: Option<&'s str>,
    read_only: bool,
    now: bool,
    no_header: bool,
    to_execute: Option<ToExecute<'s>>,
    pub args: Vec<&'s str>,
}

impl Exec<'static> {
    pub fn get_command(
        self,
        timeout: std::time::Instant,
        client: BlockedClient,
    ) -> Command {
        let return_method = self.get_return_method();
        if self.to_execute.is_none() {
            todo!("to_execute not set");
        }
        match (self.to_execute.unwrap(), self.read_only) {
            (ToExecute::Command(q), true) => Command::Query {
                query: q,
                timeout,
                return_method,
                client,
            },

            (ToExecute::Command(q), false) => Command::Exec {
                query: q,
                timeout,
                client,
                return_method,
            },
            (ToExecute::Statement(identifier), true) => {
                Command::QueryStatement {
                    identifier,
                    arguments: self.args,
                    timeout,
                    client,
                    return_method,
                }
            }
            (ToExecute::Statement(identifier), false) => {
                Command::ExecStatement {
                    identifier,
                    arguments: self.args,
                    timeout,
                    client,
                }
            }
        }
    }
    pub fn get_return_method(&self) -> ReturnMethod {
        match (self.read_only, self.into, self.no_header) {
            (true, Some(s), false) => {
                ReturnMethod::Stream { name: s }
            }
            (_, Some(s), _) => ReturnMethod::Stream { name: s },
            (_, _, true) => ReturnMethod::Reply,
            (_, _, false) => ReturnMethod::ReplyWithHeader,
        }
    }
    pub fn is_now(&self) -> bool {
        self.now
    }
    pub fn get_query(&self) -> Option<&str> {
        match self.to_execute {
            Some(ToExecute::Command(q)) => Some(q),
            _ => None,
        }
    }
    pub fn get_to_execute(&self) -> &ToExecute {
        self.to_execute.as_ref().unwrap()
    }
    pub fn is_read_only(&self) -> bool {
        self.read_only
    }
    pub fn make_into_query(&mut self) {
        self.read_only = true;
    }
    pub fn replicate_args(
        &self,
        ctx: &Context,
    ) -> Option<Vec<*mut RedisModuleString>> {
        if self.read_only {
            return None;
        }
        if self.now {
            return None;
        }
        let mut v = Vec::new();
        let to_push = RMString::new(ctx, self.database);
        v.push(to_push.as_ptr());
        std::mem::forget(to_push);
        let to_push = RMString::new(ctx, "NOW");
        v.push(to_push.as_ptr());
        std::mem::forget(to_push);
        let (t, s) = match self.to_execute.as_ref() {
            Some(ToExecute::Command(s)) => ("COMMAND", s),
            Some(ToExecute::Statement(s)) => ("STATEMENT", s),
            None => todo!("Should never happen"),
        };
        let to_push = RMString::new(ctx, t);
        v.push(to_push.as_ptr());
        std::mem::forget(to_push);
        let to_push = RMString::new(ctx, s);
        v.push(to_push.as_ptr());
        std::mem::forget(to_push);
        if self.args.len() > 0 {
            let to_push = RMString::new(ctx, "ARGS");
            v.push(to_push.as_ptr());
            std::mem::forget(to_push);
            for arg in self.args.iter() {
                let to_push = RMString::new(ctx, arg);
                v.push(to_push.as_ptr());
                std::mem::forget(to_push);
            }
        }
        Some(v)
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
            no_header: false,
            to_execute: None,
            args: Vec::new(),
        };
        while let Some(arg) = args_iter.next() {
            let mut arg_string = String::from(*arg);
            arg_string.make_ascii_uppercase();
            match arg_string.as_str() {
                "COMMAND" => match exec.to_execute {
                    Some(ToExecute::Statement(_)) => {
                        return Err(
                            RediSQLError::both_statement_and_query(),
                        );
                    }
                    Some(ToExecute::Command(_)) => {
                        return Err(RediSQLError::with_code(
                            13,
                            "Impossible to know which query should be executed".to_string(),
                            "Provided COMMAND twice".to_string(),
                        ));
                    }
                    None => {
                        let query = match args_iter.next() {
                            Some(q) => q,
                            None => {
                                return Err(RediSQLError::with_code(
                                    9,
                                    "Provided the COMMAND keyword but not the query to execute".to_string(),
                                    "No query provided".to_string(),
                                ))
                            }
                        };
                        exec.to_execute =
                            Some(ToExecute::Command(query));
                    }
                },
                "STATEMENT" => match exec.to_execute {
                    Some(ToExecute::Command(_)) => {
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
                "NO_HEADER" => exec.no_header = true,
                "ARGS" => {
                    let (size, _) = args_iter.size_hint();
                    exec.args.reserve(size);
                    while let Some(arg) = args_iter.next() {
                        exec.args.push(*arg);
                    }
                }
                _ => {}
            }
        }
        if exec.to_execute.is_none() {
            return Err(RediSQLError::with_code(24, "You didn't provide neither `COMMAND` nor `STATEMENT` fields".to_string(), "Command incomplete, no `COMMAND` nor `STATEMENT` fields".to_string()));
        }
        if exec.into.is_some() && exec.no_header {
            return Err(RediSQLError::with_code(16, "Asked a STREAM without the header".to_string(), "The header is part of the stream, does not make sense to provide a stream without header".to_string()));
        }
        if exec.into.is_some() && !exec.read_only {
            return Err(RediSQLError::with_code(17, "STREAM for not READ_ONLY query not supported".to_string(), "Asked a STREAM, but the query is not `READ_ONLY` (flag not set), this is not supported.".to_string()));
        }
        Ok(exec)
    }
    fn database(&self) -> &str {
        self.database
    }
}
