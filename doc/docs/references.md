# References

This document explains all the API that RediSQL provide to the users.

For each command, it exposes first the name and then the syntax and finally a brief explanation of what is going on inside the code.

Where is possible it provides also an estimate of the complexity since we are talking about databases not all queries have the same time and spatial complexity.

Finally, if it is appropriate the document also provides several references to external material that the interested reader can use to understand better the dynamics of every and each command.

## REDISQL.CREATE_DB

#### REDISQL.CREATE_DB db_key [path]

This command creates a new DB and associates it with the key.

The path argument is optional and, if provided is the file that SQLite will use.
It can be an existing SQLite file or it can be a not existing file.

If the file actually exists and if it is a regular SQLite file that database will be used.
If the file does not exist a new file will be created.

If the path is not provided it will open an in-memory database. Not providing a path is equivalent to provide the special string `:memory:` as path argument.

After opening the database it inserts metadata into it and then starts a thread loop.

**Complexity**: O(1), it means constant, it does not necessarily mean _fast_. However is fast enough for any use case facing human users (eg create a new database for every user logging in a website.)

**See also**: 

1. [SQLite `sqlite3_open_v2`][sqlite3_open]

## DEL

#### DEL db_key [key ...]

This command is a generic command from Redis.

It eliminates keys from Redis itself, as well if the key is a RediSQL database create with [`REDISQL.CREATE_DB`][create_db] it will eliminate the SQLite database, stop the thread loop and clean up everything left.

If the database is backed by a file the file will be close.

**Complexity**: DEL is O(N) on the number of keys, if you are only eliminating the key associated with the SQLite database will be constant, O(1).

**See also**: 

1. [SQLite `sqlite3_close`][sqlite3_close]
2. [Redis `DEL`][Redis DEL]

## REDISQL.EXEC

#### REDISQL.EXEC db_key "statement"

This command takes as input a Redis key created with [`REDISQL.CREATE_DB`][create_db] and a statement string.

Internally it transform the string into a [sqlite statement][sqlite_stmt] using [sqlite3_prepare_v2][sqlite_prepare], execute it against the database, [sqlite3_step][sqlite_step], and finally returns the results to the client.

The compilation of the string into a statement and its execution happens in a different thread from the one used by Redis and so this command has a minimum impact on the overall Redis performance, however, it does block the client.

This command is quite useful to execute [PRAGMA Statements][sqlite_pragma], for normal operations against the database is suggested to use `STATEMENTS`.

Also, remember that there is only a single thread for database, execution of multiple `REDISQL.EXEC` against the same database will result in a serialization of the executions, one will be executed before the others.

Finally is important to have in mind how `sqlite3_prepare_v2` works. The functions compile only the first statement, (everything between the start of the string and the first semicolon `;`) and not the whole string. This means that executing commands like:

``` SQL
BEGIN TRANSACTION; -- here first semicolon, and first statement
INSERT INTO ... ;
SELECT n FROM ...;
CASE 
  WHEN n >= 4 THEN ABORT
  ELSE COMMIT
END;
```

Will **NOT**, let me repeat, it will **not** work as expected, the first statement is just `BEGIN TRANSACTION;` and only that one will be executed.

Please note that we are seriously considering to change this behavior in order to make transactions and other queries work in an expected way.

**Complexity**: It depends entirely on the statement string. The use of a single thread for database is been chosen after several tests where the single thread configuration was faster than a multi-thread one. This is true in a write-intensive application and in a mixed write/read application.

**See also**:

1. [SQLite `sqlite3_prepare_v2`][sqlite_prepare]
2. [SQLite `statement` aka `sqlite3_stmt`][sqlite_stmt]
3. [SQLite `sqlite3_step`][sqlite_step]
4. [SQLite `PRAGMA`s][sqlite_pragma]
5. [Redis Blocking Command][Redis Blocking Command]


## REDISQL.CREATE_STATEMENT

#### REDISQL.CREATE_STATEMENT db_key stmt_identifier "statement"

This command compiles a statement string into a [sqlite statement][sqlite_stmt] and associate such statement to an identifier.

The same limitation of [REDISQL.EXEC][exec] regarding multiple statements in the same string. Only the first SQL statement (from the beginning to the first semicolon `;`) get compiled into a sqlite statement.

Using this command you can insert parameters using the special symbol `?`, those parameters will be bind to the statements when you are executing the statement itself.

For now only the `?` syntax is supported.

This command does not execute anything against the database, but simply store the sqlite statements into a dictionary associated with the identifier provided (`stmt_identifier`). Then it stores the information regarding the statement in the metadata table in order to provide a simple way to restore also the statements.

The statement is associated with a database, a statement created for one database cannot be used for another database, you need to create a different one. This allows a simple and fast way to provide persistence.

You can execute the statement with [`REDISQL.EXEC_STATEMENT`][exec_statement].

You cannot overwrite a statement using this command.

If you need to change the implementation of a statement you have two options:
1. Delete the statement using [`REDISQL.DELETE_STATEMENT`][delete_statement] and the create a new one.
2. Use [`REDISQL.UPDATE_STATEMENT`][update_statement]

Suppose that a service needs a particular statement to be defined in order to work, this safety measure allows the users to simply go ahead, try to create it, and in case catch the error.

Also, this command is not blocking, meaning that all the work happens in a separate thread respect the redis one.

**Complexity**: If we assume that the time necessary to compile a string into a sqlite statement is constant, overall the complexity is O(1), again constant, not necessarily _fast_.

**See also**:

1. [SQLite `sqlite3_prepare_v2`][sqlite_prepare]
2. [SQLite `statement` aka `sqlite3_stmt`][sqlite_stmt]
3. [SQLite bindings, `sqlite3_bind_text`][sqlite_binding]
4. [`REDISQL.EXEC_STATEMENT`][exec_statement]
5. [`REDISQL.DELETE_STATEMENT`][delete_statement]
6. [`REDISQL.UPDATE_STATEMENT`][update_statement]
7. [Redis Blocking Command][Redis Blocking Command]

## REDISQL.EXEC_STATEMENT

#### REDISQL.EXEC_STATEMENT db_key stmt_identifier [binding_parameters ...]

This command binds all the parameters to the statement created using [`REDISQL.CREATE_STATEMENT`][create_statement] and identified by `stmt_identifier`. Then the module executes the statement against the database associated to `db_key`.

The number of parameters must be coherent with the number expected by the statement. If there were 3 `?` you need to provide 3 binding parameters.
The bindings are associated in order to the statement.

Redis works using a text protocol, all the arguments are encoded as text, hence the module is forced to use the procedure `sqlite3_bind_text`, however, SQLite is smart enough to recognize numbers and treat them correctly. Numbers will be treated as numbers and text will be treated as text.

Finally, once completed the binding part the statement is executed and its result is returned to the client.

This command as well is not blocking, all the work happens in a different thread from the one of Redis.

**Complexity**: The complexity to retrieve and to bind the parameters is roughly constant for any practical purpose, however, the overall complexity will be dominated by the time to execute the query.

**See also**:

1. [SQLite `statement` aka `sqlite3_stmt`][sqlite_stmt]
2. [SQLite bindings, `sqlite3_bind_text`][sqlite_binding]
3. [`REDISQL.CREATE_STATEMENT`][create_statement]
4. [Redis Blocking Command][Redis Blocking Command]

## REDISQL.DELETE_STATEMENT

#### REDISQL.DELETE_STATEMENT db_key stmt_identifier

This command eliminates a statement from the database.

It first looks it up into the internal hash table, if it finds the statement the command removes it from the internal hash table and then remove it from an internal SQLite table.

Also, this command is not blocking and work in a different thread from the main Redis one.

**Complexity**: The complexity is constant and it can be considered _fast_ for most practical application.

**See also**:

1. [SQLite `statement` aka `sqlite3_stmt`][sqlite_stmt]
2. [`REDISQL.CREATE_STATEMENT`][create_statement]
3. [`REDISQL.EXEC_STATEMENT`][exec_statement]
4. [`REDISQL.UPDATE_STATEMENT`][update_statement]
5. [Redis Blocking Command][Redis Blocking Command]

## REDISQL.UPDATE_STATEMENT

#### REDISQL.UPDATE_STATEMENT db_key stmt_identifier "statement"

The command update and _existing_ statement changing its internal implementation to the one provide as string.

If the statement does not exist the command will fail and return an error, again this is a safety measure, you must be completely aware that you are changing the implementation of a statement and updating a not existing statement or creating an existing one will result in an error.

Internally the command starts checking if the statement is already defined, then it tries to compile the string into a [`sqlite3_stmt`][sqlite_stmt] and if everything went right it finally updates the metadata table and finally returns to the client.

This command is not blocking as well.

**Complexity**: The complexity is constant and it can be considered _fast_ for most practical application.

**See also**:

1. [SQLite `statement` aka `sqlite3_stmt`][sqlite_stmt]
2. [`REDISQL.CREATE_STATEMENT`][create_statement]
3. [`REDISQL.EXEC_STATEMENT`][exec_statement]
4. [`REDISQL.DELETE_STATEMENT`][delete_statement]
5. [Redis Blocking Command][Redis Blocking Command]


[sqlite3_close]: https://sqlite.org/c3ref/close.html
[Redis DEL]: https://redis.io/commands/del
[sqlite3_open]: https://sqlite.org/c3ref/open.html
[sqlite_prepare]: https://sqlite.org/c3ref/prepare.html
[sqlite_stmt]: https://sqlite.org/c3ref/stmt.html
[sqlite_step]: https://sqlite.org/c3ref/step.html
[sqlite_pragma]: https://sqlite.org/pragma.html 
[Redis Blocking Command]: https://redis.io/topics/modules-blocking-ops
[exec_statement]: #redisqlexec_statement
[delete_statement]: #redisqldelete_statement
[sqlite_binding]: https://sqlite.org/c3ref/bind_blob.html
[create_statement]: #redisqlcreate_statement
[exec]: #redisqlexec
[create_db]: #redisqlcreate_db
[update_statement]: #redisqlupdate_statement
