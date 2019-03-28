# RediSQL: Fast, in memory, SQL. With Batteries included.

RediSQL is the Fast, in-memory, SQL engine with batteries included.

It provides several benefits for your application.

1. Fast, it can top up to 130k inserts per *second*.
2. Familiar, it works with standard SQL, no weird dialect or syntax to learn.
3. Easy to Operate, it is based on Redis, just start the standard Redis binary and and pass the RediSQL binary.
4. Easy to Use, being based on Redis there are already bindings for any language.

## Use cases

There are several use cases for RediSQL.

### RediSQL to store Transient Data

RediSQL is wonderful for transient data.
With supports for lightweights DBs, you can just store all the data that are important now, and trash them all together as soon as they are not necessary anymore.

### RediSQL as main database

The product is stable, it does not lose data.
Moreover RediSQL supports all the persistency features of Redis, hence RDB and AOF are both fully supported.

## Much more to explore...

There are a lot of features in RediSQL that are worth exploring more. Here are short explanations of those features.

#### Lightweight DBs

RediSQL provides you with lightweight in-memory databases.
It could completely shift your architecture. For example, you could create a new isolated database each day, one for each application tenant, or even one per user.

#### On disk storage

While RediSQL focuses on in-memory database, it can also store data in a regular file. Of course this makes operations slower, but it allows RediSQL to reach a level of data persistency on par with incumbent databases such as Postgres or MySQL.

#### Copy of DBs

With the concept of lightweight databases, it becomes necessary to have a way to duplicate your database. 
Indeed it is possible to copy a database into another one. 
This allows several interesting patterns. 
For example, you could copy an in-memory database into a file-backed database and then ship the file for storage or other analysis. 
Another pattern would be to copy a heavy read database into another for load balancing reasons.
Moreover you may use the copy function to keep a "base-database" to grow when new data comes in.

#### Directly expose the DB to users

With the ability to create several lightweight databases, and the capability to copy those database, you could directly expose the databases to the end users instead of exposing an API that you would need to maintain.
Just load all the data that the user may need into a RediSQL database, document the tables that are available, and give access to it to the users. 
They will write their own API without waiting on you.

#### Stream and cache query results

RediSQL can also store results of queries into a Redis Streams.
This allows different clients to consume part of the result, or to delay the consuption of a result. 
Moreover, it allows caching the result of expensive queries as Redis Streams to consume them over and over again.

#### Complete JSON support

JSON is the de-facto standard for sharing data between applications. RediSQL exploits the JSON1 module of SQLite to bring that capability to easy and quickly manage JSON data inside SQL statements and tables.
In RediSQL you are able to manipulate JSON in every standard way.

#### Full text search support

RediSQL fully supports also the FTS{3,4,5} engine from SQLite, giving you a full text engine at your fingertip.
You will be able to manage and search for data.

## Getting started

RediSQL is a Redis module, hence you will need a modern version of [Redis (> 5.0)][redis-download] and the RediSQL .so file.

You can obtain RediSQL directly [from our store.](https://payhip.com/RediSQL)

Alternative you can download the module from [github release.](https://github.com/RedBeardLab/rediSQL/releases)

Finally you could compile the source yourself, simpy with `cargo build --release` from the root of the project.

To start RediSQL you simply pass it as paramenter to the redis binary.

```
./redis-server --loadmodule /path/to/RediSQL.so 
```

At this point you have your standard redis instance working as you would expect plus all the RediSQL interface.

All the commands are documented in [the references.][api]

## Docker image

Moreover, also a Docker image is provide. Is sufficient to run the image `siscia/redisql`.

Note - keep running both

```
$docker run -it --net host siscia/redisql //Server

$docker run -it --net host siscia/redisql redis-cli //Client

```

This will start a RediSQL instance and allow you to work directly with RediSQL.

## Tutorials and walkthrought

We create a few tutorial to guide you on using RediSQL with Node.js, Go(lang) and Python:

- [Using RediSQL with Python](http://redisql.redbeardlab.com/rediSQL/blog/python/using-redisql-with-python/)
- [Using RediSQL with Golang](http://redisql.redbeardlab.com/rediSQL/blog/golang/using-redisql-with-golang/)
- [Using RediSQL with Node.js](http://redisql.redbeardlab.com/rediSQL/blog/node/using-redisql-with-node/)

Please open an issue and request a tutorial for any other language you are intereted in.

The fastest way to explore RediSQL is using the `redis-cli`.

```
$ ~/redis-4.0-rc1/src/redis-cli 
127.0.0.1:6379> 
127.0.0.1:6379> SET A 3
OK
127.0.0.1:6379> GET A
"3"
# Great, still the old good redis we know, but now with extra commands.
127.0.0.1:6379> REDISQL.CREATE_DB DB
OK
# Start creating a table on the default DB
127.0.0.1:6379> REDISQL.EXEC DB "CREATE TABLE foo(A INT, B TEXT);"
DONE
# Insert some data into the table
127.0.0.1:6379> REDISQL.EXEC DB "INSERT INTO foo VALUES(3, 'bar');"
OK
# Retrieve the data you just inserted
127.0.0.1:6379> REDISQL.EXEC DB "SELECT * FROM foo;"
1) 1) (integer) 3
   2) "bar"
# Of course you can make multiple tables
127.0.0.1:6379> REDISQL.EXEC DB "CREATE TABLE baz(C INT, B TEXT);"
OK
127.0.0.1:6379> REDISQL.EXEC DB "INSERT INTO baz VALUES(3, 'aaa');"
OK
127.0.0.1:6379> REDISQL.EXEC DB "INSERT INTO baz VALUES(3, 'bbb');"
OK
127.0.0.1:6379> REDISQL.EXEC DB "INSERT INTO baz VALUES(3, 'ccc');"
OK
# And of course you can use joins
127.0.0.1:6379> REDISQL.EXEC DB "SELECT * FROM foo, baz WHERE foo.A = baz.C;"

1) 1) (integer) 3
   2) "bar"
   3) (integer) 3
   4) "aaa"
2) 1) (integer) 3
   2) "bar"
   3) (integer) 3
   4) "bbb"
3) 1) (integer) 3
   2) "bar"
   3) (integer) 3
   4) "ccc"
127.0.0.1:6379> 
```
## Documentation

The complete API are explained in the official documentation that you can access here: [API References][api]

## Contributing

I am going to accept pull request here on github.

## License

This software is licensed under the AGPL-v3, it is possible to purchase more permissive licenses.

<RediSQL, SQL steroids for Redis.>
Copyright (C) 2019  Simone Mosciatti

[api]: http://redisql.redbeardlab.com/rediSQL/references/
[redis-download]: https://redis.io/download
