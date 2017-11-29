# RediSQL

RediSQL is a redis module that embeded SQLite.

_With great powers comes great responsability_ (cit. Uncle Ben)

# Documentation

This readme will provide you with the basis, however for deeper documentation you should look here: [siscia.github.io/rediSQL](https://siscia.github.io/rediSQL/)

## Motivation

I love the agility provided by Redis, however, several times, I wished I had a little more structure in my in-memory database.

Even basic SQL is very powerful and years upon years of experience on several SQL implementation have bring us very mature product that we can now exploit with confidence.

Between all the SQL implementation, the one that best fitted the need for this module is definitely SQLite, for its velocity, portability, simplicity and capability to work in memory.

## Getting start

You can download the `.so` directly from github following the [release link](https://github.com/RedBeardLab/rediSQL/releases)

With the `.so` you can start redis passing the object as argument like so:

```
./redis-server --loadmodule librediSQL.so 
```

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

At this point you should have the `.so` inside the `target/release/` directory.

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

## API

### REDISQL.CREATE_DB key

This function will create a new SQLite database that will be bound to `key`.

```
127.0.0.1:6379> REDISQL.CREATE_DB user 
OK                                                    
```

### REDISQL.EXEC key statement

This command will execute the statement against the database bound to `key`. 

```
$ ./redis-cli
127.0.0.1:6379> REDISQL.CREATE_DB user 
OK

$ ./redis-cli
127.0.0.1:6379> REDISQL.EXEC user "CREATE TABLE user(email TEXT, password TEXT)"
OK 
127.0.0.1:6379> REDISQL.EXEC user "INSERT INTO user VALUES('test@test.it','very secret')"
OK
127.0.0.1:6379> REDISQL.EXEC user "INSERT INTO user VALUES('noob@security.io', 'password')"
OK  
127.0.0.1:6379> REDISQL.EXEC user "SELECT * FROM user;"   
1) 1) "test@test.it" 
   2) "very secret" 
2) 1) "noob@security.io" 
   2) "password" 
127.0.0.1:6379> 

```

## Benchmarks

Benchmarks are always tricky and it definitely depends on your use case, however I can provide some number at least for `INSERT` operations.

I ran a simple benchmark where I insert a tuple of 3 integers.

I start establishing a baseline:

```
$ ./redis-benchmark -e -c 50 -n 500000 -r 100000 PING
====== PING ======
  500000 requests completed in 7.30 seconds
  50 parallel clients
  3 bytes payload
  keep alive: 1

99.76% <= 1 milliseconds
100.00% <= 2 milliseconds
100.00% <= 2 milliseconds
68483.77 requests per second
```

Now I ran my benchmark:

```
$ ./redis-benchmark -e -c 50 -n 500000 -r 100000 REDISQL.EXEC A "INSERT INTO test VALUES(__rand_int__,__rand_int__,__rand_int__);"
====== REDISQL.EXEC A INSERT INTO test VALUES(__rand_int__,__rand_int__,__rand_int__); ======
  500000 requests completed in 10.44 seconds
  50 parallel clients
  3 bytes payload
  keep alive: 1

84.19% <= 1 milliseconds
99.38% <= 2 milliseconds
99.92% <= 3 milliseconds
99.93% <= 9 milliseconds
99.94% <= 10 milliseconds
99.95% <= 11 milliseconds
99.95% <= 13 milliseconds
99.95% <= 14 milliseconds
99.96% <= 15 milliseconds
99.96% <= 78 milliseconds
99.96% <= 79 milliseconds
99.97% <= 80 milliseconds
99.97% <= 89 milliseconds
99.97% <= 90 milliseconds
99.98% <= 91 milliseconds
99.98% <= 94 milliseconds
99.98% <= 95 milliseconds
99.99% <= 96 milliseconds
99.99% <= 98 milliseconds
99.99% <= 99 milliseconds
100.00% <= 100 milliseconds
100.00% <= 101 milliseconds
100.00% <= 101 milliseconds
47915.67 requests per second
```

### Result

Overall Redis was able to manage ~70K PING per second and the module ~50K SQL inserts per seconds.

This is a very narrow test and if you care about performance, you should perform your own test; I would love if you could share your result.

Overall there are a lot of opportunity to optimize the SQL.

Also, keep in mind, that I am running those test in an old machine and your numbers may be different.


## Safeness vs. Performance

The tradeoff between safeness and performace is always crucial in all applications.

Since Redis is born as am in-memory database, as default, also the RediSQL modules works with in memory databases, however you can create a standard, file backed, database.

Also, you can adjust all the PRAGMA settings to fit your uses case.


## RoadMap

We would like to move following the necessity of the community, so ideas and use cases are extremely welcome.

We do have already a couple of ideas: 

[x] Introducing concurrency and non-blocking queries. 

[ ] Supports for prepared statements

[ ] A cache system to store the result of the more complex select.

But please share your thoughts.

## Limits

This module is based on SQLite so it has all the SQLite strenghts and limitations.

The appropriate use cases for SQLite are described in [this document](https://sqlite.org/whentouse.html).

With this module we remove the network limitation and so the use of this module is not suggested in only two cases:

#### Many concurrent writers.

SQLite does hold a table level lock on write while "standard" database can hold a row, or even value level lock.

This means that concurrent client will never be able to write at the same time on the same table and one will always need to wait for the other to finish.

At the moment this limiting factor is only secondary with the respect of this module.
Indeed, the module, is not multithread (it will be soon, though), so before to blame SQLite for slow concurrent write you must blame me, the author.

However, it should not be an issues for most uses cases, if your specific use case require a lot of concurent read I would suggest you to still try and benchmark this implementation.

#### BIG dataset

Because its internal SQLite can handle only up to 140TB of data, ideally this will also apply to this same module, supposing you know where to host the database.

However when the dimension of the dataset start to approach a terabyte you may be better of looking for other alternatives.

Of course if you use SQLite as in memory database the limiting factor will be the memory of your machine.

## Single thread

When you create a new database a new thread is started, all the operations on the database will be performed by only that thread.

The choice of using a single thread, in my tests, yielded overall better performaces.

Clearly, if the load is mainly reads, this choice will not be optimal.

## Alpha code

This is alpha code, there will be definitely some rough edges and some plain bugs.

I really appreciate if you take your time to report those bugs.

A lot of possible functionalities are not yet implemented. By now it is suppose to work on a single redis instance In a future it will be possible to distributed some functionalities of the modules.

## Contributing

I am going to accept pull request here on github.

However since I am going to sell premium version of this products I must ask to every contributer to assign all the rights of the contribution to me.

A pull request template is in place.

## Need incentives

I am not sure, myself, that this module should exist at all. If you find this little module useful or you think that it has some potential, please star the project or open issues requiring functionalities.

## OpenSource and the necessity of real support and charge for my time.

[How to Charge for your Open Source](http://www.mikeperham.com/2015/11/23/how-to-charge-for-your-open-source/) by Mike Perham brings good arguments on the necessity to charge for work done by developers, even in the Open Source world.

I myself have started a lot of Open Source project that, eventually, are all dead because I wasn't able to dedicate the right amount of time to them.

I am hoping to find the necessary funds to keep maintain this project.

I am starting with only an Open Source version, and then move to an enterprise version adding the necessary features.


## License

This software is licensed under the AGPL-v3, it is possible to buy more permissive licenses.

<RediSQL, SQL capabilities to redis.>
Copyright (C) 2016  Simone Mosciatti


