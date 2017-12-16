use std::fmt;
use std::error;
use std::error::Error;


use sqlite as sql;
use redis;

pub trait RediSQLErrorTrait: fmt::Display + error::Error {}

pub struct RediSQLError {
    debug: String,
    error_description: String,
}

impl RediSQLError {
    pub fn new(debug: String,
               error_description: String)
               -> RediSQLError {
        RediSQLError {
            debug,
            error_description,
        }
    }
}

impl fmt::Debug for RediSQLError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.debug)
    }
}

impl fmt::Display for RediSQLError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.error_description)
    }
}

impl error::Error for RediSQLError {
    fn description(&self) -> &str {
        self.error_description.as_str()
    }
}

impl From<sql::SQLite3Error> for RediSQLError {
    fn from(err: sql::SQLite3Error) -> RediSQLError {
        RediSQLError {
            debug: format!("{}", err),
            error_description: err.description().to_owned(),
        }
    }
}

impl From<redis::RedisError> for RediSQLError {
    fn from(err: redis::RedisError) -> RediSQLError {
        RediSQLError {
            debug: format!("{}", err),
            error_description: err.description().to_owned(),
        }
    }
}
