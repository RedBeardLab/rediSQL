# RediSQL: Fast, in memory, SQL. With Batteries included.

RediSQL is the Fast, in-memory, SQL engine with batteries included.

It provides several benefits for your application.

1. Fast, it can top up to 130k inserts per *seconds*.
2. Familiar, it works with standard SQL, no weird dialect or syntax to learn.
3. Easy to Operate, it is based on Redis, just start the standard Redis binary and and pass the RediSQL binary.
4. Easy to Use, being based on Redis there are already bindings for any language.

## Use cases

There are several use cases for RediSQL.

### RediSQL to store Transient Data

RediSQL is wonderful for transient data.
With supports for lightweights DBs you can just store all the data that are important now, and trash them all together as soon as they are not necessary anymore.

### RediSQL as main database

The product is stable, it does not loose data and you can store the data into regular files to gain in persistency.
Moreover RediSQL support all the persistency features of Redis, hence RDB and AOF are both fully supported.

## Much more to explore...

There are a lot of features in RediSQL that is worth to explore more, here a short excerpt of those features, please follow the link on the main website.

#### Lightweight DBs

RediSQL provides you with lightweight in memory databases.
It could completely shift your architecture, indeed you could thing to create a new isolated database each day, or for every tenant of your application or even for each of your users.

#### On disk storage

While RediSQL focus on in-memory database, it can also store data in a regular file. Of course this makes operations slower, but it allow to reach a level of persistency and safeness of data on pair with regular databases as Postgres or MySQL.

#### Copy of DBs

With the concept of lightweight databases it become necessary to have a way to duplicate your database. 
Indeed it is possible to copy a database into another one. 
This allow several interesting patterns. 
For example you could copy an in-memory database into a file-backed database and then ship the file for storage or other analysis. 
Another pattern would be to copy a heavy read database into another for load balancing reasons.
Moreover you may use the copy function to keep a "base-database" to grow when new data comes in.

#### Directly expose the DB to users

With the ability to create several lightweight databases and the capability to copy those database, you could directly expose directly the databases to the final users instead of exposing a set of API that you would need to maintain.
Just load all the data that the user may need into a RediSQL database, document the tables that are available, and give access to it to the users. 
They will write their own API without waiting on you.

#### Stream and cache query results

RediSQL allow to store the results of queries into a Redis Stream.
This allow different clients to consume part of the result, or to delay the consuption of a result. 
Moreover it allow to cache the result of expensive queries into Redis Streams and consume them over and over again.

#### Complete JSON support

JSON is the de-facto standard to share data between application, indeed RediSQL exploit the JSON1 module of SQLite to bring you the capability to easy and quickly manage JSON data inside SQL statements and tables.
In RediSQL you are able to manipulate JSON in all the possible way.

#### Full text search support

RediSQL fully supports also the FTS{3,4,5} engine in SQLite giving you a full text engine at your fingertip.
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

## Tutorials and walkthrought

We create a few tutorial to guide you on using RediSQL with Node.js, Go(lang) and Python:

- [Using RediSQL with Python](http://redbeardlab.tech/rediSQL/blog/python/using-redisql-with-python/)
- [Using RediSQL with Golang](http://redbeardlab.tech/rediSQL/blog/golang/using-redisql-with-golang/)
- [Using RediSQL with Node.js](http://redbeardlab.tech/rediSQL/blog/node/using-redisql-with-node/)

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





## OLD

RediSQL is a module for Redis that embed a completely functional SQLite database.

RediSQL enables new paradigm where is possible to have several smaller decentralized databases instead of a single giant one.

_With great powers comes great responsability_ (cit. Uncle Ben)

# Documentation

This readme will provide you with the basics, however for deeper documentation you should look here: [redbeardlab.tech/rediSQL/](http://redbeardlab.tech/rediSQL/)

## Motivation

I love the agility provided by Redis, however, several times, I wished I had a little more structure in my in-memory database.

Even basic SQL is very powerful and years upon years of experience on several SQL implementations have brought us a very mature product that we can now exploit with confidence.

Between all the SQL implementation, the one that best fitted the need for this module is definitely SQLite, for its velocity, portability, simplicity, and capability to work in memory.

## Getting start

There are three main way to get RediSQL.

The first way is to download the public release directly from [this link](https://payhip.com/RediSQL).

These will provide you with the community version that is free, however, you have the possibility to leave a donation (20€ would be the suggested ammount). Indeed, is not necessary to pay anything for the community edition and feel free to just input 0€.

Another option is to download the module from [github release](https://github.com/RedBeardLab/rediSQL/releases)

With the `.so` you can start redis passing the object as argument like so:

```
./redis-server --loadmodule librediSQL.so 
```

Please note that you need to run redis > 4.0 to use modules and RediSQL is not an exception.

The last way is to compile the module yourself:

## Tutorials:

We provide tutorials to get started with different languages, if you need help for a particular programming language please open an issue.

- [Using RediSQL with Python](http://redbeardlab.tech/rediSQL/blog/python/using-redisql-with-python/)
- [Using RediSQL with Golang](http://redbeardlab.tech/rediSQL/blog/golang/using-redisql-with-golang/)
- [Using RediSQL with Node.js](http://redbeardlab.tech/rediSQL/blog/node/using-redisql-with-node/)

## Compiling and contributing

If you want to compile the module yourself or contribute to the project you can simply clone the repo

```
$ git clone http://github.com/RedBeardLab/rediSQL/
Cloning into 'rediSQL'...
remote: Counting objects: 1404, done.
remote: Total 1404 (delta 0), reused 0 (delta 0), pack-reused 1404
Receiving objects: 100% (1404/1404), 7.28 MiB | 487.00 KiB/s, done.
Resolving deltas: 100% (513/513), done.
Checking connectivity... done.
```

Then move inside the directory and compile the module:

```
$ cargo build --release 
```

At this point, you should have the `.so` inside the `target/release/` directory.

Now launch Redis with the module load will looks similarly to this:

```
$ ~/redis-4.0-rc1/src/redis-server --loadmodule ./target/release/librediSQL.so 
6833:M 15 Dec 16:25:53.195 * Increased maximum number of open files to 10032 (it was originally set to 1024).
                _._                                                  
           _.-``__ ''-._                                             
      _.-``    `.  `_.  ''-._           Redis 3.9.101 (00000000/0) 64 bit
  .-`` .-```.  ```\/    _.,_ ''-._                                   
 (    '      ,       .-`  | `,    )     Running in standalone mode
 |`-._`-...-` __...-.``-._|'` _.-'|     Port: 6379
 |    `-._   `._    /     _.-'    |     PID: 6833
  `-._    `-._  `-./  _.-'    _.-'                                   
 |`-._`-._    `-.__.-'    _.-'_.-'|                                  
 |    `-._`-._        _.-'_.-'    |           http://redis.io        
  `-._    `-._`-.__.-'_.-'    _.-'                                   
 |`-._`-._    `-.__.-'    _.-'_.-'|                                  
 |    `-._`-._        _.-'_.-'    |                                  
  `-._    `-._`-.__.-'_.-'    _.-'                                   
      `-._    `-.__.-'    _.-'                                       
          `-._        _.-'                                           
              `-.__.-'                                               

6833:M 15 Dec 16:25:53.197 * Module 'rediSQL__' loaded from ./rediSQL.so
6833:M 15 Dec 16:25:53.197 * The server is now ready to accept connections on port 6379
```

## Walkthrough

After starting redis with the module rediSQL it will be just the redis you learn to love:

```
$ ~/redis-4.0-rc1/src/redis-cli 
127.0.0.1:6379> 
127.0.0.1:6379> SET A 3
OK
127.0.0.1:6379> GET A
"3"
```

But you will also able to use all the API described below:

```
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

Also the `LIKE` operator is included:

```
127.0.0.1:6379> REDISQL.EXEC DB "CREATE TABLE text_search(t TEXT);"
OK
127.0.0.1:6379> REDISQL.EXEC DB "INSERT INTO text_search VALUES('hello');"
OK
127.0.0.1:6379> REDISQL.EXEC DB "INSERT INTO text_search VALUES('banana');"
OK
127.0.0.1:6379> REDISQL.EXEC DB "INSERT INTO text_search VALUES('apple');"
OK
127.0.0.1:6379> 
127.0.0.1:6379> REDISQL.EXEC DB "SELECT * FROM text_search WHERE t LIKE 'h_llo';"
1) 1) "hello"
127.0.0.1:6379> REDISQL.EXEC DB "SELECT * FROM text_search WHERE t LIKE '%anana';"
1) 1) "banana"
127.0.0.1:6379> REDISQL.EXEC DB "INSERT INTO text_search VALUES('anana');"
OK
127.0.0.1:6379> REDISQL.EXEC DB "SELECT * FROM text_search;"
1) 1) "hello"
2) 1) "banana"
3) 1) "apple"
4) 1) "anana"
127.0.0.1:6379> REDISQL.EXEC DB "SELECT * FROM text_search WHERE t LIKE 'a%';"
1) 1) "apple"
2) 1) "anana"
``` 

Now you can create tables, insert data on those tables, make queries, remove elements, everything.

# PRO version

The PRO version is available [here](https://plasso.com/s/epp4GbsJdp-redisql/signup/) it cost 990€ / years and of course you have 14 days money back if you are not satisfied with the product.

The PRO version provides two main capabilities and **dedicated support** from the creators of RediSQL.
    
### Non blocking commands

All (but `REDISQL.CREATE_DB`) commands in the module blocks the clients and execute the computation in a different thread. 

This means that the redis engine is free to serve other clients and it doesn't freeze on long select and it can use more CPU power. 

Blocking is the more sensible choice in the general case, however, in some case you may need `non blocking` command.

The PRO version provide you with that.

In the PRO version is enough to add the suffix `.NOW` to any command to invoke the blocking version.

### AOF & Replication

AOF and Replication in Redis works with the same underneath implementation.

It is quite complex to implement and usefull mostly to companies where redis is a critical piece of infrastructure.

For these reasone the AOF and the Replication are provide in the PRO version.

## About the cost

We set up the cost to make it a bargain for any company that actually use the product.

If you consider the ammount of time necessary to replicate this features:
    
    1. Understand the code base of RediSQL
    2. Understand how Redis itself works
    3. Understand how SQLite works
    4. Implement the features
    5. Document it
    6. Test it
    7. Maintaint it

And you multiply for the hour cost of an engineer (~100€/hour).

You will see that RediSQL PRO will pay itself in a little more that 1 day of work.

All this without considering the dedicated support that comes with the plan.

Moreover, up to our knowledge we are the only one to provide a similar product (SQL, in memory inside a cache engine).


## API

The complete API are explained in the official documentation that you can access here: [API References][api]

## Contributing

I am going to accept pull request here on github.

## OpenSource and the necessity of real support and charge for my time.

[How to Charge for your Open Source](http://www.mikeperham.com/2015/11/23/how-to-charge-for-your-open-source/) by Mike Perham brings good arguments on the necessity to charge for work done by developers, even in the Open Source world.

I myself have started a lot of Open Source project that, eventually, are all dead because I wasn't able to dedicate the right amount of time to them.

I am hoping to find the necessary funds to keep maintain this project.

I am starting with only an Open Source version and then move to an enterprise version adding the necessary features.


## License

This software is licensed under the AGPL-v3, it is possible to purchase more permissive licenses.

<RediSQL, SQL steroids for Redis.>
Copyright (C) 2019  Simone Mosciatti

[api]: http://redbeardlab.tech/rediSQL/references/

