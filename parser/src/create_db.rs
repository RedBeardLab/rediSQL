use redisql_lib::redisql_error::RediSQLError;

use crate::common::CommandV2;

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
            name,
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
