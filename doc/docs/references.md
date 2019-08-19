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

## REDISQL.QUERY.INTO

#### REDISQL.QUERY.INTO[.NOW] stream_name db_key "query"

This command is similar to [`REDISQL.QUERY`][query] but instead of returning the result of the query, it append each row to the [stream][redis_streams_intro] `stream_name` passed as first argument. 

The query must be a read-only one, exactly as [`REDISQL.QUERY`][query].

The command executes [`XADD`][redis_xadd] to the stream, hence if the stream does not exists a new one is created. On the other hand, if the stream already exists the rows are simply appended.

The command itself is eager, hence it compute the whole result, append it into the stream, and then it returns. Once the command returns, the whole result set is already in the Redis stream.

The return value of the command depends on the result of the query:

1. If the result of the query is empty, it simply returns `["DONE", 0]`, exactly like [`REDISQL.QUERY`][query].
2. If at least one row is returnend by the query the command returns the name of the stream where it appended the resulting rows, which is exactly the one passed as input, the first and the last ID added to the stream and the total number of entries added to the stream.

The stream will use autogeneratated IDs.

Each entry in a stream is a set of field-value (key-value) pairs. The field (key) will be the type of the row and its name separated by a colon. It cpuld be something like `int:users` or `text:user_name` or even `real:x_coordinate`.

The value will simply store the value of the column untouched. 

```
127.0.0.1:6379> REDISQL.CREATE_DB DB
OK
127.0.0.1:6379> REDISQL.EXEC DB "CREATE TABLE foo(a int, b int);"
1) DONE
2) (integer) 0
127.0.0.1:6379> REDISQL.EXEC DB "INSERT INTO foo(a) VALUES(1)"
1) DONE
2) (integer) 1
127.0.0.1:6379> REDISQL.EXEC DB "INSERT INTO foo VALUES(3, 4)"
1) DONE
2) (integer) 1
127.0.0.1:6379> REDISQL.EXEC DB "INSERT INTO foo VALUES(5, 6)"
1) DONE
2) (integer) 1
127.0.0.1:6379> REDISQL.EXEC DB "INSERT INTO foo VALUES(10, 19)"
1) DONE
2) (integer) 1
127.0.0.1:6379> REDISQL.QUERY.INTO {DB}:all_foo DB "SELECT * FROM foo"
1) 1) "{DB}:all_foo"
   2) "1549811093979-0"
   3) "1549811093979-3"
   4) (integer) 4
127.0.0.1:6379> XRANGE {DB}:all_foo - +
1) 1) "1549811093979-0"
   2) 1) "int:a"
      2) "1"
      3) "null:b"
      4) "(null)"
2) 1) "1549811093979-1"
   2) 1) "int:a"
      2) "3"
      3) "int:b"
      4) "4"
3) 1) "1549811093979-2"
   2) 1) "int:a"
      2) "5"
      3) "int:b"
      4) "6"
4) 1) "1549811093979-3"
   2) 1) "int:a"
      2) "10"
      3) "int:b"
      4) "19"
```


Using a standard Redis Stream all the standard consideration applies.

1. The stream is not deleted by RediSQL, hence it can definitely be used for caching, on the other hand too many streams will use memory.
2. The stream use a standard Redis key, in a cluster environment you should be sure that the database that is executing the query and the stream that will accomodate the result are on the same cluster node. 
This can be accomplished easily by forcing the stream name to hash to the same cluster node of the database, it is sufficiento to use a `stream_name` composed as such `{db_key}:what:ever:here`. Redis will hash only the part between the `{` and `}` in order to compute the cluster node.
3. The result can be consumed using the standard [Redis streams commands][redis_stream_commands], two good starting points are [`XREAD`][redis_xread] and [`XRANGE`][redis_xrange].

**Complexity**: The complexity of the command is `O(n)` where `n` is the amount of row returned by the query.

**See also**:

1. [`REDISQL.QUERY`][query] 
2. [`REDISQL.QUERY_STATEMENT.INTO`][query_statement_into] 
3. [Redis Streams Intro][redis_streams_intro]
4. [Redis Streams Commands][redis_stream_commands]
5. [`XADD`][redis_xadd]
6. [`XREAD`][redis_xread]
7. [`XRANGE`][redis_xrange]

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


## REDISQL.QUERY_STATEMENT.INTO

#### REDISQL.QUERY_STATEMENT.INTO[.NOW] stream_name db_key stmt_identifier [binding_parameters ...]

This command behave like [`REDISQL.QUERY.INTO`][query_into] but instead of a query it takes as input a read-only statement and its binding paramenters.

**Complexity**: The complexity of the command is `O(n)` where `n` is the amount of row returned by the query.

**See also**:

1. [`REDISQL.QUERY.INTO`][query_into] 
2. [`REDISQL.QUERY_STATEMENT`][query_statement] 
3. [Redis Streams Intro][redis_streams_intro]
4. [Redis Streams Commands][redis_stream_commands]
5. [`XADD`][redis_xadd]
6. [`XREAD`][redis_xread]
7. [`XRANGE`][redis_xrange]



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

## REDISQL.COPY

#### REDISQL.COPY[.NOW] db_key_source db_key_destination 

The command copies the source database into the destination database.

The content of the destination databases is completely ignored and lost.

It is not important if the databases are stored in memory or backed by disk, the `COPY` command will work nevertheless.

This command is useful to:

1. Create backups of databases
2. Load data from a slow, disk based, databases into a fast in-memory one
3. To persist data from a in-memory database into a disk based database
4. Initialize a database with a predefined status

Usually the destination database is an empty database just created, while the source one is a databases where we have been working for a while.

This command use the [backup API][backup_api] of sqlite.

**Complexity**: The complexity is linear on the number of page (dimension) of the source database, beware it can be "slow" if the source database is big, during the copy the `source` database is busy and it cannot serve other queries. 

**See also**:

1. [Backup API][backup_api]


## REDISQL.STATISTICS

#### REDISQL.STATISTICS

The command print the internal statistics of RediSQL.

There are 3 counter associated to each command. 
The first one for counting the number of times the command is been invoked.
The second (`OK` counter) keep tracks of how many times the command returned successfully.
The third (`ERR` counter) memorize the amount of times the command returned an error. 

The counters are implemented as atomic counters, they don't use locks nor introduces any notiaceble slowdown to the application.

```
127.0.0.1:6379> REDISQL.STATISTICS
 1) 1) "CREATE_DB"
    2) (integer) 1
 2) 1) "CREATE_DB OK"
    2) (integer) 1
 3) 1) "CREATE_DB ERR"
    2) (integer) 0
 4) 1) "EXEC"
    2) (integer) 4
 5) 1) "EXEC OK"
    2) (integer) 4
 6) 1) "EXEC ERR"
    2) (integer) 0
 7) 1) "QUERY"
    2) (integer) 0
 8) 1) "QUERY OK"
    2) (integer) 0
 9) 1) "QUERY ERR"
    2) (integer) 0
10) 1) "QUERY.INTO"
    2) (integer) 0
11) 1) "QUERY.INTO OK"
    2) (integer) 0
12) 1) "QUERY.INTO ERR"
    2) (integer) 0
13) 1) "CREATE_STATEMENT"
    2) (integer) 3
14) 1) "CREATE_STATEMENT OK"
    2) (integer) 1
15) 1) "CREATE_STATEMENT ERR"
    2) (integer) 2
16) 1) "EXEC_STATEMENT"
    2) (integer) 2
17) 1) "EXEC_STATEMENT OK"
    2) (integer) 2
18) 1) "EXEC_STATEMENT ERR"
    2) (integer) 0
19) 1) "UPDATE_STATEMENT"
    2) (integer) 2
20) 1) "UPDATE_STATEMENT OK"
    2) (integer) 1
21) 1) "UPDATE_STATEMENT ERR"
    2) (integer) 1
22) 1) "DELETE_STATEMENT"
    2) (integer) 0
23) 1) "DELETE_STATEMENT OK"
    2) (integer) 0
24) 1) "DELETE_STATEMENT ERR"
    2) (integer) 0
25) 1) "QUERY_STATEMENT"
    2) (integer) 0
26) 1) "QUERY_STATEMENT OK"
    2) (integer) 0
27) 1) "QUERY_STATEMENT ERR"
    2) (integer) 0
28) 1) "QUERY_STATEMENT.INTO"
    2) (integer) 0
29) 1) "QUERY_STATEMENT.INTO OK"
    2) (integer) 0
30) 1) "QUERY_STATEMENT.INTO ERR"
    2) (integer) 0
31) 1) "COPY"
    2) (integer) 0
32) 1) "COPY OK"
    2) (integer) 0
33) 1) "COPY ERR"
    2) (integer) 0
```



**Complexity**: The complexity is constant.

## REDISQL.COPY

#### REDISQL.COPY[.NOW] db_key_source db_key_destination 

The command copies the source database into the destination database.

The content of the destination databases is completely ignored and lost.

It is not important if the databases are stored in memory or backed by disk, the `COPY` command will work nevertheless.

This command is useful to:

1. Create backups of databases
2. Load data from a slow, disk based, databases into a fast in-memory one
3. To persist data from a in-memory database into a disk based database
4. Initialize a database with a predefined status

Usually the destination database is an empty database just created, while the source one is a databases where we have been working for a while.

This command use the [backup API][backup_api] of sqlite.

**Complexity**: The complexity is linear on the number of page (dimension) of the source database, beware it can be "slow" if the source database is big, during the copy the `source` database is busy and it cannot serve other queries. 

**See also**:

1. [Backup API][backup_api]

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
[backup_api]: https://www.sqlite.org/backup.html
[redis_streams_intro]: https://redis.io/topics/streams-intro
[redis_stream_commands]: https://redis.io/commands#stream
[redis_xadd]: https://redis.io/commands/xadd
[redis_xread]: https://redis.io/commands/xread
[redis_xrange]: https://redis.io/commands/xrange
[query_into]: #redisqlqueryinto
[query_statement_into]: #redisqlquery_statementinto
