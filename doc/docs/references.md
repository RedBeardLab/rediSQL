# References

This document explains all the API that RediSQL provide to the users.

For each command, it exposes first the name and then the syntax and finally a brief explanation of what is going on inside the code.

Where is possible, it provides also an estimate of the complexity but since we are talking about databases not all queries have the same time and spatial complexity.

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

#### REDISQL.EXEC[.NOW] db_key "statement"

This command takes as input a Redis key created with [`REDISQL.CREATE_DB`][create_db] and a statement string.

Internally it transform the string into a [sqlite statement][sqlite_stmt] using [sqlite3_prepare_v2][sqlite_prepare], execute it against the database, [sqlite3_step][sqlite_step], and finally returns the results to the client.

The compilation of the string into a statement and its execution happens in a different thread from the one used by Redis and so this command has a minimum impact on the overall Redis performance, however, it does block the client.

This command is quite useful to execute [PRAGMA Statements][sqlite_pragma], for normal operations against the database is suggested to use `STATEMENTS`.

Also, remember that there is only a single thread for database, execution of multiple `REDISQL.EXEC` against the same database will result in a serialization of the executions, one will be executed before the others.

If you only need to query the database without modifying the data is a better idea to use [`REDISQL.QUERY`][query].

**Complexity**: It depends entirely on the statement string. The use of a single thread for database is been chosen after several tests where the single thread configuration was faster than a multi-thread one. This is true in a write-intensive application and in a mixed write/read application.

**See also**:

1. [SQLite `sqlite3_prepare_v2`][sqlite_prepare]
2. [SQLite `statement` aka `sqlite3_stmt`][sqlite_stmt]
3. [SQLite `sqlite3_step`][sqlite_step]
4. [SQLite `PRAGMA`s][sqlite_pragma]
5. [Redis Blocking Command][Redis Blocking Command]

## REDISQL.QUERY

#### REDISQL.QUERY[.NOW] db_key "statement"

This command behaves similarly to [`REDISQL.EXEC`][exec] but it imposes an additional constraint on the statement it executes.

It only executes the statement if it is a read-only operation, otherwise, it returns an error.

A read-only operation is defined by the result of calling [`sqlite3_stmt_readonly`][sqlite_readonly] on the compiled statement.

The statement is executed if and only if [`sqlite3_stmt_readonly`][sqlite_readonly] returns true.

If you need to execute the same query over and over it is a good idea to create a statement and use [`REDISQL.QUERY_STATEMENT`][query_statement].

**Complexity**: Similar to [`REDISQL.EXEC`][exec], however, if a statement is not read-only it is aborted immediately and it does return an appropriate error.

**See also**:

1. [SQLite `sqlite3_prepare_v2`][sqlite_prepare]
2. [SQLite `statement` aka `sqlite3_stmt`][sqlite_stmt]
3. [SQLite `sqlite3_step`][sqlite_step]
4. [SQLite `PRAGMA`s][sqlite_pragma]
5. [Redis Blocking Command][Redis Blocking Command] 
6. [`REDISQL.EXEC`][exec]
7. [SQLite `sqlite3_stmt_readonly`][sqlite_readonly]
8. [`REDISQL.QUERY_STATEMENT`][query_statement] 

## REDISQL.CREATE_STATEMENT

#### REDISQL.CREATE_STATEMENT[.NOW] db_key stmt_identifier "statement"

This command compiles a statement string into a [sqlite statement][sqlite_stmt] and associate such statement to an identifier.

Using this command you can insert parameters using the special symbol `?NNN`, those parameters will be bind to the statements when you are executing the statement itself.

For now only the `?NNN` syntax is supported, where `N` is a digit (Ex. `?1`, `?2`, `?3` ...)

This command does not execute anything against the database, but simply store the sqlite statements into a dictionary associated with the identifier provided (`stmt_identifier`). Then it stores the information regarding the statement in the metadata table in order to provide a simple way to restore also the statements.

The statement is associated with a database, a statement created for one database cannot be used for another database, you need to create a different one. This allows a simple and fast way to provide persistence.

You can execute the statement with [`REDISQL.EXEC_STATEMENT`][exec_statement].

You cannot overwrite a statement using this command.

If you need to change the implementation of a statement you have two options:

1. Delete the statement using [`REDISQL.DELETE_STATEMENT`][delete_statement] and the create a new one.
2. Use [`REDISQL.UPDATE_STATEMENT`][update_statement]

Suppose that a service needs a particular statement to be defined in order to work, this safety measure allows the users to simply go ahead, try to create it, and in case catch the error.

Also, this command is not blocking, meaning that all the work happens in a separate thread respect the redis one.

Please keep in mind that the parameters should be named in order and that there should not be any gap.

```SQL
INSERT INTO foo VALUES(?1, ?2, ?3); -- this one is fine and we work as you expect

INSERT INTO foo VALUES(?1, ?123, ?564); -- this one will be more problematic, and you should avoid it
```

Keep in mind that SQLite start to count the bounding parameters from 1 and not from 0, using `?0` is an error.

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

#### REDISQL.EXEC_STATEMENT[.NOW] db_key stmt_identifier [binding_parameters ...]

This command binds all the parameters to the statement created using [`REDISQL.CREATE_STATEMENT`][create_statement] and identified by `stmt_identifier`. Then the module executes the statement against the database associated to `db_key`.

For each parameter in the query of the form `?nnn` the engine will look for the `nnn-th` binding_parameters.
So if the statements is from the following query:

``` SQL
INSERT INTO foo VALUES(?1, ?2, ?3);
```

You will only need to provide 3 parameters and they will be bound, in order to `?1`, `?2` and `?3`.

If your statements looks like this:

``` SQL
INSERT INTO foo VALUES(?1, ?123, ?564);
```

You will need to provide 564 parameters and only the first, the 123-rd and the 564-th will be considered.

SQLite starts to count the binding parameters from 0, not from 1. Using `?0` is an error.

Redis works using a text protocol, all the arguments are encoded as text, hence the module is forced to use the procedure `sqlite3_bind_text`, however, SQLite is smart enough to recognize numbers and treat them correctly. Numbers will be treated as numbers and text will be treated as text.

Finally, once completed the binding part the statement is executed and its result is returned to the client.

This command as well is not blocking, all the work happens in a different thread from the one of Redis.

If you need to query your database, without modifying the data is a better idea to use [`REDISQL.QUERY_STATEMENT`][query_statement].
 

**Complexity**: The complexity to retrieve and to bind the parameters is roughly constant for any practical purpose, however, the overall complexity will be dominated by the time to execute the query.

**See also**:

1. [SQLite `statement` aka `sqlite3_stmt`][sqlite_stmt]
2. [SQLite bindings, `sqlite3_bind_text`][sqlite_binding]
3. [`REDISQL.CREATE_STATEMENT`][create_statement]
4. [Redis Blocking Command][Redis Blocking Command]
5. [`REDISQL.QUERY_STATEMENT`][query_statement] 


## REDISQL.QUERY_STATEMENT

#### REDISQL.QUERY_STATEMENT[.NOW] db_key stmt_identifier [binding_parameters ...]

This command behaves similarly to [`REDISQL.EXEC_STATEMENT`][exec_statement] however it does impose an additional constraint.

It executes the statement if it is a read-only operation, otherwise, it returns an error.

A read-only operation is defined by the result of calling [`sqlite3_stmt_readonly`][sqlite_readonly] on the compiled statement.

The statement is executed if and only if [`sqlite3_stmt_readonly`][sqlite_readonly] returns true.

The result of [`sqlite3_stmt_readonly`][sqlite_readonly] is cached.

If you don't want to create a statement to run a query just once you can use [`REDISQL.QUERY`][query].

**Complexity**: Similar to [`REDISQL.EXEC_STATEMENT`][exec_statement], however, if a statement is not read-only it is aborted immediately and it does return an appropriate error.

**See also**:

1. [SQLite `sqlite3_prepare_v2`][sqlite_prepare]
2. [SQLite `statement` aka `sqlite3_stmt`][sqlite_stmt]
3. [SQLite `sqlite3_step`][sqlite_step]
4. [SQLite `PRAGMA`s][sqlite_pragma]
5. [Redis Blocking Command][Redis Blocking Command] 
6. [`REDISQL.EXEC_STATEMENT`][exec_statement]
7. [SQLite `sqlite3_stmt_readonly`][sqlite_readonly]
8. [`REDISQL.QUERY`][query] 


## REDISQL.DELETE_STATEMENT

#### REDISQL.DELETE_STATEMENT[.NOW] db_key stmt_identifier

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

#### REDISQL.UPDATE_STATEMENT[.NOW] db_key stmt_identifier "statement"

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


# Virtual Tables

What follows is not a RediSQL command but an SQLite virtual table introduced by the module.

Virtual tables behave similarly to normal tables but have some limitations, for a deeper explanation please visit the [official SQLite documentation about virtual tables.][virtual_table]

At the moment the module provides a single read-only virtual table: `REDISQL_TABLES_BRUTE_HASH`.

## REDISQL_TABLES_BRUTE_HASH

This virtual table allows you to query [Redis Hashes][redis_hash] that follow a similar pattern.

A redis hash is composed by a key, that identifies the structure in the whole database, and several sub-keys that map to different string fields.

This structure can easily be mapped to a standard table, where the key identifies the row and the sub-keys the columns.

Redis does not impose any limitation to the format of the hash key, however, in order to use the virtual table you need to follow a specific syntax that happens to be the de-facto standard for hash keys.

The key must be in the following format `$tableName:$id` where `$id` must be an integer. There are no limitations on the sub-keys.

```
127.0.0.1:6379> HSET cats:1 name romeo location rome hungry 3
(integer) 3
127.0.0.1:6379> HSET cats:2 name garfield location london hungry 10
(integer) 3
127.0.0.1:6379> HSET cats:3 name "simon's cat" location "simon's house" hungry 8
(integer) 3
```

In this examples we have a table of cats, each with a name, a location, and a hungry level.

Redis is perfect if we want to know how hungry is `romeo` or where is located `garfield`.

However is a little more difficult to answer query like: who is the hungriest cat? Are there any cats in London? 

Of course, the use of different data structures could alleviate these issues but then there will be the necessity to keep the several data structures in sync one with the other.

Another alternative can be the use of the `REDISQL_TABLE_BRUTE_HASH` virtual table.

```
127.0.0.1:6379> REDISQL.EXEC DB "CREATE VIRTUAL TABLE funny_cats USING REDISQL_TABLES_BRUTE_HASH(cats, name, location, hungry);"
1) DONE
2) (integer) 0
127.0.0.1:6379> REDISQL.EXEC DB "SELECT * FROM funny_cats"
1) 1) "cats:2"
   2) "garfield"
   3) "london"
   4) "10"
2) 1) "cats:1"
   2) "romeo"
   3) "rome"
   4) "3"
3) 1) "cats:3"
   2) "simon's cat"
   3) "simon's house"
   4) "8"
```

This virtual table allows querying the redis hashes using a more convenient SQL syntax. It does require a constant amount of space but it operates in linear time with the respect of the elements in the "hash table".

The syntax of the virtual table is quite simple, `REDISQL_TABLES_BRUTE_HASH(cats, name, location, hungry)`, as first we need the $tableName, so the key of every row without the `:$id` part. 
Then the columns of the table. Please note that you do **not** provide the type of the column in the declaration.

Is not necessary that every key defines all the columns (sub-keys), if a key does not have a specific sub-key, it will simply be returned as (nil).

This virtual table is a read-only virtual table, it means that -- at the moment -- you can only `select` from this table, so you cannot `insert`, `update` or `delete` from this table.

Another limitation is that Redis Hashes can store only strings, not integers or floats. This implies that by default we will return only strings when you query a table, of course, you could cast them to integers or float via SQLite.

```
127.0.0.1:6379> REDISQL.EXEC DB "SELECT name, location, CAST(hungry AS INTEGER) FROM cats"
1) 1) "garfield"
   2) "london"
   3) (integer) 10
2) 1) "romeo"
   2) "rome"
   3) (integer) 3
3) 1) "simon's cat"
   2) "simon's house"
   3) (integer) 8
```

This specific virtual table works by continuously querying Redis itself.

When you execute a `SELECT` against it, the first step is to [`SCAN`][scan] all the possible keys, for each key then we retrieve the associated values in each sub-key using [`HGET`][hget] and finally we return the result.

**Complexity**.

This implementation comes with several trade-offs.

The space complexity is constant and negligible, no data is duplicated and are necessary only few bytes for the SQLite data structures.

The time complexity for a query is linear `O(m*n)` where `m` is the number of rows and `n` is the number of columns.

This virtual table does not support `INSERT`, `UPDATE` or `DELETE`.


**See also**:

1. [SQLite virtual tables][virtual_table]
2. [Redis Hashes][redis_hash]
3. [`SCAN`][scan]
4. [`HGET`][hget]

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
[sqlite_readonly]: https://www.sqlite.org/c3ref/stmt_readonly.html
[query]: #redisqlquery
[query_statement]: #redisqlquery_statement
[virtual_table]: https://sqlite.org/vtab.html
[redis_hash]: https://redis.io/topics/data-types#hashes
[scan]: https://redis.io/commands/scan
[hget]: https://redis.io/commands/hget
