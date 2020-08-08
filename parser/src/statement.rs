use redisql_lib::redis::Command;
use redisql_lib::redis::ReturnMethod;
use redisql_lib::redis_type::BlockedClient;
use redisql_lib::redisql_error::RediSQLError;

use crate::common::CommandV2;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Action {
    Delete,
    Update,
    New,
    Show,
    List,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Statement<'s> {
    database: &'s str,
    action: Action,
    stmt_name: Option<&'s str>,
    stmt_query: Option<&'s str>,
    now: bool,
    can_update: bool,
    can_create: bool,
}

impl Statement<'static> {
    pub fn get_command(self, client: BlockedClient) -> Command {
        match self.action {
            Action::Delete => Command::DeleteStatement {
                identifier: self.stmt_name.unwrap(),
                client,
            },
            Action::Update => Command::UpdateStatement {
                identifier: self.stmt_name.unwrap(),
                statement: self.stmt_query.unwrap(),
                can_create: self.can_create,
                client,
            },
            Action::New => Command::CompileStatement {
                identifier: self.stmt_name.unwrap(),
                statement: self.stmt_query.unwrap(),
                can_update: self.can_update,
                client,
            },
            Action::Show => Command::ShowStatement {
                identifier: self.stmt_name.unwrap(),
                return_method: ReturnMethod::ReplyWithHeader,
                client,
            },
            Action::List => Command::ListStatements {
                return_method: ReturnMethod::ReplyWithHeader,
                client,
            },
        }
    }
    pub fn is_now(&self) -> bool {
        self.now
    }
    pub fn get_action(&self) -> Action {
        self.action
    }
    pub fn identifier(&self) -> &'static str {
        self.stmt_name.unwrap()
    }
    pub fn statement(&self) -> &'static str {
        self.stmt_query.unwrap()
    }
    pub fn can_update(&self) -> bool {
        self.can_update
    }
    pub fn can_create(&self) -> bool {
        self.can_create
    }
}

impl<'s> CommandV2<'s> for Statement<'s> {
    fn parse(args: Vec<&'s str>) -> Result<Self, RediSQLError> {
        let mut args_iter = args.iter();
        args_iter.next();
        let database = match args_iter.next() {
            Some(name) => name,
            None => return Err(RediSQLError::no_database_name()),
        };
        let action = match args_iter.next() {
            None => return Err(RediSQLError::with_code(18, "The statement command needs an action, either: DELETE, UPDATE, NEW or SHOW".to_string(), "Statement command without command".to_string())),
            Some(a) => {
                let mut action_str = String::from(*a);
                action_str.make_ascii_uppercase();
                match action_str.as_str() {
                    "DELETE" => Action::Delete,
                    "UPDATE" => Action::Update,
                    "NEW" => Action::New,
                    "SHOW" => Action::Show,
                    "LIST" => Action::List,
                    _ => return Err(RediSQLError::with_code(23,
                            "You provide a command for the statement that is not supported".to_string(),
                            "Statement command unknow".to_string()))
                }
            }
        };
        let stmt_name = match action {
            Action::New
            | Action::Update
            | Action::Delete
            | Action::Show => match args_iter.next() {
                Some(s) => Some(*s),
                None => {
                    return Err(RediSQLError::with_code(19, "You should provide the name of the statement to operate with".to_string(), "Statement command with statement name".to_string()));
                }
            },
            Action::List => None,
        };
        let stmt_query = match action {
            Action::Update | Action::New => match args_iter.next() {
                Some(s) => Some(*s),
                None => return Err(RediSQLError::with_code(20, "Statement actions requires a query to be provided in input".to_string(), "No query provided for the statement".to_string())),
            },
            _ => None,
        };
        let mut command = Statement {
            database,
            action,
            stmt_name,
            stmt_query,
            now: false,
            can_update: false,
            can_create: false,
        };
        for args in args_iter {
            let mut arg_string = String::from(*args);
            arg_string.make_ascii_uppercase();
            match arg_string.as_str() {
                "NOW" => command.now = true,
                "CAN_UPDATE" => command.can_update = true,
                "CAN_CREATE" => command.can_create = true,
                _ => {}
            }
        }
        if command.can_update && command.action != Action::New {
            return Err(RediSQLError::with_code(21,
                    "Flag `CAN_UPDATE` is supported only by STAMENTE NEW action".to_string(),
                    "Unexpected flag `CAN_UPDATE`".to_string()));
        }
        if command.can_create && command.action != Action::Update {
            return Err(RediSQLError::with_code(22,
                    "Flag `CAN_CREATE` is supported only by STATEMEN UPDATE action".to_string(), 
                    "Unexpected flag `CAN_CREATE`".to_string()));
        }
        Ok(command)
    }

    fn database(&self) -> &str {
        self.database
    }
}
