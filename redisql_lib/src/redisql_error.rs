use std::fmt;

use crate::sqlite as sql;

pub trait RediSQLErrorTrait: fmt::Display {}

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
 * 9   - Provided COMMAND keyword without providing a query to run
 * 10  - Provided STATEMENT keywork without providing a statement to run
 * 11  - Provided INTO keywork without providing the stream to use
 * 12  - Provided both COMMAND and STATEMENT keywords
 * 13  - Provided COMMAND twice
 * 14  - Provided STATEMENT twice
 * 15  - Provided key does not exists
 * 16  - Ask stream without heading, which does not make sense
 * 17  - Ask stream for not read_only query, not supported
 * 18  - Statement command without Action (DELETE, UPDATE, NEW, SHOW)
 * 19  - Statement command without name of the statement to operate with
 * 20  - Statement command without necessary query, for UPDATE or NEW
 * 21  - Only Statement NEW support CAN_UPDATE flag
 * 22  - Only Statement UPDATE support CAN_CREATE flag
 * 23  - Command for Statement unknown
 * 24  - Didn't provide neither COMMAND nor STATEMENT
 * 25  - Provide name of the statement to STATEMENT LIST
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
        RediSQLError::with_code(
                5,
                "You are trying to work with a key that does not contains RediSQL data, but contains other data".to_string(),
                "Key does not belong to us".to_string(),
            )
    }
    pub fn empty_key() -> Self {
        RediSQLError::with_code(
                15,
                "You are trying to work with a key that is empty, run REDISQL.CREATE_DB first".to_string(),
                "Key does not exists".to_string(),
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

impl From<sql::SQLite3Error> for RediSQLError {
    fn from(err: sql::SQLite3Error) -> RediSQLError {
        RediSQLError {
            code: err.code,
            debug: format!("{}", err),
            error_description: err.error_message,
        }
    }
}
