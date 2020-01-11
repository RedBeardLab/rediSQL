use std::error;
use std::error::Error;
use std::fmt;

use crate::sqlite as sql;

pub trait RediSQLErrorTrait: fmt::Display + error::Error {}

pub struct RediSQLError {
    #[allow(dead_code)]
    code: u32,
    debug: String,
    error_description: String,
}

impl RediSQLError {
    pub fn new(debug: String, error_description: String) -> Self {
        RediSQLError {
            code: 0,
            debug,
            error_description,
        }
    }
    pub fn timeout() -> Self {
        RediSQLError::new(
            "Timeout expired.".to_string(),
            "It was impossible to return the whole result before the timeout expired.".to_string(),
            )
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
            code: 0,
            debug: format!("{}", err),
            error_description: err.description().to_owned(),
        }
    }
}
