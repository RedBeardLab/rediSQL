# Release 0.8.0 of RediSQL, SQL steroids for Redis

#### RediSQL, Redis on SQL steroids.

RediSQL is a Redis module that provides full SQL capabilities to Redis, it is the simplest and fastest way to get an SQL database up and running, without incurring in difficult operational issues and it can scale quite well with your business.

The fastest introduction to RediSQL is [our homepage](https://redisql.com)

**tl;dr** This release introduce two new commands [`REDISQL.QUERY.INTO[.NOW]`][query_into] and [`REDISQL.QUERY_STATEMENT.INTO[.NOW]`][query_statement_into]. 
The new commands behave similary to `REDISQL.QUERY` and `REDISQL.QUERY_STATEMENT` but they [`XADD`][redis_xadd] the results to a [Redis Stream][redis_streams_intro] passed as first argument.


## Motivation

Being able to write the result of a query into a stream opens several possibilities.
First off all allow to easily cache the result of expensive queries.
Then, it separate the creation of a result with its consuption which is a very important step forward especially for big results.

Indeed, while the computation of a query is not done by the main redis thread but it is off-load to another thread to allow redis to keep serving the client. Returning the result must be done in the main Redis thread. 
Hence a long result can take a lot of time to be returned to the client and in that time Redis cannot serve other requests.
Writing the result into a stream make it much more efficient use of the main Redis thread time.

Moreover, on the other side of the network, a small consumer might not expect a big result and could be overlwhelmed by the size.

In standard databases this problem is usually solved using cursors, however Redis itself does not provide this facility.
Redis provide lists, but they are simply flat list and can store only strings, it would be complex to create the cursors on top of them.

The streams however are a better fit. While also them can store only strings, they store them into entries, which are small key-values objects.
Each entry represent a row of our result set.
Where we encode the column name and type into the key, and we use the value field to store the actual value of the column.

An example will be easier to follow.

## How to use

An example of `REDISQL.QUERY.INTO` is the following:

```
REDISQL.QUERY.INTO result_stream DB "SELECT foo, bar FROM baz WHERE n > 42"
```

The command will execute the query `SELECT foo, bar FROM baz WHERE n > 42` agains the database `DB` and it will `XADD` each row of the result to the stream `result_stream`.

If the result is empty, the command will return `["DONE", 0]` to the Redis client.

If the result is not empty, the command will return, to the Redis client, the name of the stream used (hence `result_stream` in this example) along with the first ID added and the last ID added and the size of the cursor (the number of entries added to the stream.)

In the following example we start by creating a database, then we create a new table `foo` in the database, and we store 4 rows into the table.

Then we use the new `REDISQL.QUERY.NOW` command to store the result of the query `SELECT * FROM foo` agains the database `DB` in the stream `{DB}:all_foo`.

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

The first thing to notice is that the stream entity contains both the type of the column and it's name as well. The format is `$column_type:$column_name`. This is necessary because stream support only strings.

In the example above the string `int:a` means that, for this row, the column `a` is of type `int`. Usually the type of a column is constant, however, it may be null, in that case it would be something like: `null:b`.

Another interesting thing to notice is the name of the stream used which can look peculiar. Indeed it is the same name of the database `DB`, between curly braces `{DB}` and then a useful identifier `{DB}:all_foo`. The name of the stream can be any name, so is not important to use this schema, however this schema is useful if you use redis cluster.

Indeed, both keys, the target stream `{DB}:all_foo` and the source database `DB`, need to be on the same redis cluster node. Since redis use the part of the key between curly bracket to decide in which node a key should resize, this schema allow us to make sure that this invariant is always respected.

Moreover this schema is also quite nice, allowing with a glance to know what stream refer to what database. But again, it is not necessary at all.


[redis_xadd]: https://redis.io/commands/xadd
[redis_streams_intro]: https://redis.io/topics/streams-intro
[query_into]: ../references.md#redisqlqueryinto
[query_statement_into]: ../references.md#redisqlquery_statementinto
