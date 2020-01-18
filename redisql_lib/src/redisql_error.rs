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

/**
 * Codes list:
 * 1   - The name of the database to use was not provide in the command
 * 2   - Provide the PATH option but not the PATH to use
 * 3   - Provide as input both CAN_EXISTS and MUST_CREATE, this is a contradiction
 * 4   - Request to create a new database (using MUST_CREATE flag) but one database with the same
 *   name already exists.
 * 5   - Trying to work with a Key that does not belong to RediSQL, it could be a standard redis type
 *   or a type from another redis module
 * 6   - Error in opening the database connection
 * 7   - Error to store the key into redis
 * 8   - Unknow error in saving the key
 * 9   - Provided QUERY keyword without providing a query to run
 * 10  - Provided STATEMENT keywork without providing a statement to run
 * 11  - Provided INTO keywork without providing the stream to use
 * 12  - Provided both QUERY and STATEMENT keywords
 * 13  - Provided QUERY twice
 * 14  - Provided STATEMENT twice
 */
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
    pub fn with_code(
        code: u32,
        debug: String,
        error_description: String,
    ) -> Self {
        RediSQLError {
            code,
            debug,
            error_description,
        }
    }
    pub fn no_database_name() -> Self {
        RediSQLError::with_code(
            1,
            "You didn't provide a database name".to_string(),
            "No database name provide".to_string(),
        )
    }
    pub fn both_statement_and_query() -> Self {
        RediSQLError::with_code(
            12,
            "The EXEC runs either a QUERY `OR` a STATEMENT, but you try to provide one STATEMENT `AND` one QUERY to run".to_string(),
            "Provided both STATEMENT and QUERY keywords".to_string(),
        )
    }
    pub fn no_redisql_key() -> Self {
        RediSQLError::with_code(15, "The command takes as input a KEY, or DATABASE. The one that you provide is a RediSQL database".to_string(), "The KEY provided is not managed by RediSQL".to_string())
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
