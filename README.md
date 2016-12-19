# RediSQL

RediSQL is a redis module that embeded SQLite.

_With great powers comes great responsability_ (cit. Uncle Ben)

## Motivation

I love the agility provided by Redis, however, several times, I wished I had a little more structure in my in-memory database.

Even basic SQL is very powerful and years upon years of experience on several SQL implementation have bring us very mature product that we can now exploit with confidence.

Between all the SQL implementation, the one that best fitted the need for this module is definitely SQLite, for its velocity, portability, simplicity and capability to work in memory.

## OpenSource and the necessity of real support and charge for my time.

[How to Charge for your Open Source](http://www.mikeperham.com/2015/11/23/how-to-charge-for-your-open-source/) by Mike Perham brings good arguments on the necessity to charge for work done by developers, even in the Open Source world.

I myself have started a lot of Open Source project that, eventually, are all dead because I wasn't able to dedicate the right amount of time to them.

I am hoping to find the necessary funds to keep maintain this project.

I am starting with only an Open Source version, and then move to an enterprise version adding the necessary features.

## Usage

You can get started simply downloading the git repo:

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
$ cd rediSQL/
$ make sqlite 
gcc -fPIC -c -o sqlite3.o sqlite3.c
$ make
gcc -c -Wpedantic -fPIC -IRedisModulesSDK/ -Bstatic rediSQL.c -o rediSQL.o
ld -o rediSQL.so rediSQL.o sqlite3.o RedisModulesSDK/rmutil/librmutil.a -shared -lc
```

At this point you can launch your redis instance loading the module:

```
$ ~/redis-4.0-rc1/src/redis-server --loadmodule ./rediSQL.so file_path.sqlite
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

## Default DB

When you start a redis instance loading the module, the module itself create a SQLite default database, this DB will be the one used when you do not specify what database to use.

If you load the module passing as argument a valid path name, rediSQL will open such database or it will create a new one.

If you do not pass any argument while loading the module, rediSQL will still create a new database but it will be an in-memory one.

## API

### REDISQL.SQLITE_VERSION

This function reply with the version of SQLite that is actually in use.

```
127.0.0.1:6379> REDISQL.SQLITE_VERSION
3.15.1                               
```

### REDISQL.CREATE_DB key [file_path]

This function will create a new SQLite database that will be bound to `key`.

If you do not provide a `file_path` the new database will be an in-memory one, if you provide a file_path, that would be where your database will reside.

```
127.0.0.1:6379> REDISQL.CREATE_DB user /tmp/user.sqlite
OK                                                    
$ ls /tmp/ | grep user 
user.sqlite
```

### REDISQL.EXEC [key] statement

This command will execute the statement against the database bound to `key`, if you do not specify a key the statement will be executed against the standard DB.

```
$ ./redis-cli
127.0.0.1:6379> REDISQL.CREATE_DB user /tmp/user.sqlite
OK

$ ll /tmp/ | grep user 
-rw-r--r--  1 simo simo    0 dic 18 12:55 user.sqlite 

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

$ ll /tmp/ | grep user 
-rw-r--r--  1 simo simo 8192 dic 18 12:56 user.sqlite
```

### REDISQL.DELETE_DB key

This function will remove the database bound to `key`, if the database is on disk you will find all your data saved, if the database is in-memory your data will be lost.

This function is equivalent to `DELL key` however it won't let you delete keys that are not DBs.

```
127.0.0.1:6379> REDISQL.DELETE_DB user
OK                                   
127.0.0.1:6379> REDISQL.EXEC user "SELECT * FROM user;" 
(error) WRONGTYPE Operation against a key holding the wrong kind of value  
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

But you will also able to use all the API described above

```
127.0.0.1:6379> 
# Start creating a table on the default DB
127.0.0.1:6379> REDISQL.EXEC "CREATE TABLE foo(A INT, B TEXT);"
OK
# Insert some data into the table
127.0.0.1:6379> REDISQL.EXEC "INSERT INTO foo VALUES(3, 'bar');"
OK
# Retrieve the data you just inserted
127.0.0.1:6379> REDISQL.EXEC "SELECT * FROM foo;"
1) 1) (integer) 3
   2) "bar"
# Of course you can make multiple tables
127.0.0.1:6379> REDISQL.EXEC "CREATE TABLE baz(C INT, B TEXT);"
OK
127.0.0.1:6379> REDISQL.EXEC "INSERT INTO baz VALUES(3, 'aaa');"
OK
127.0.0.1:6379> REDISQL.EXEC "INSERT INTO baz VALUES(3, 'bbb');"
OK
127.0.0.1:6379> REDISQL.EXEC "INSERT INTO baz VALUES(3, 'ccc');"
OK
# And of course you can use joins
127.0.0.1:6379> REDISQL.EXEC "SELECT * FROM foo, baz WHERE foo.A = baz.C;"

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
127.0.0.1:6379> REDISQL.EXEC "CREATE TABLE text_search(t TEXT);"
OK
127.0.0.1:6379> REDISQL.EXEC "INSERT INTO text_search VALUES('hello');"
OK
127.0.0.1:6379> REDISQL.EXEC "INSERT INTO text_search VALUES('banana');"
OK
127.0.0.1:6379> REDISQL.EXEC "INSERT INTO text_search VALUES('apple');"
OK
127.0.0.1:6379> 
127.0.0.1:6379> REDISQL.EXEC "SELECT * FROM text_search WHERE t LIKE 'h_llo';"
1) 1) "hello"
127.0.0.1:6379> REDISQL.EXEC "SELECT * FROM text_search WHERE t LIKE '%anana';"
1) 1) "banana"
127.0.0.1:6379> REDISQL.EXEC "INSERT INTO text_search VALUES('anana');"
OK
127.0.0.1:6379> REDISQL.EXEC "SELECT * FROM text_search;"
1) 1) "hello"
2) 1) "banana"
3) 1) "apple"
4) 1) "anana"
127.0.0.1:6379> REDISQL.EXEC "SELECT * FROM text_search WHERE t LIKE 'a%';"
1) 1) "apple"
2) 1) "anana"
``` 

And, of course, also errors are managed:

```
127.0.0.1:6379> REDISQL.EXEC "INSERT INTO baz VALUES("aaa", "bbb");"
Invalid argument(s)
127.0.0.1:6379> 
127.0.0.1:6379> REDISQL.EXEC "CREATE TABLE baz(f INT, k TEXT);"
(error) ERR - table baz already exists | Query: CREATE TABLE baz(f INT, k TEXT);
```

Now you can create tables, insert data on those tables, make queries, remove elements, everything.

Finally all the above features can be applied to the default DB or to a databases bound to a specif key. 

## Benchmark

Benchmarks are a little tricky, there are a lot of factor that may alter them, especially in this particular case.

However just to have an idea of the order of magnitude of insert per second I wrote a [little test](https://github.com/RedBeardLab/rediSQL/blob/381e3796ad31c231719380afb92352c54b244b8c/test/performance/rediSQL_bench.py).

On my machine I got 1000 insert (each one in its own transaction) in 0.6 seconds which let me claim 1000 / 0.6 => 1600 insert per second.

I believe that there are A LOT of possibilities to improvements this numbers but I also thinks that they are good enough for most workload.

If you need something faster, please take the time to open an issues and describe your use case.

## RoadMap

We would like to move following the necessity of the community, so ideas and use cases are extremely welcome.

We do have already a couple of ideas: 

1. Introducing concurrency and non-blocking queries. 

2. Stream all the statements that modify the data, everything but `SELECT`s.

3. A cache system to store the result of the more complex select.

But please share your thoughts.

## Limit

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

However when the dimension of the dataset start to approach the terabyte you may be better of looking for other alternatives.

Of course if you use SQLite as in memory database the limiting factor will be the memory of your machine.

## Limit of the module

Right now there are some limit on the module implementation, these limitation are because my lack of time.

#### Single thread

Right now the module is single thread and it use the same thread of Redis.

This means that during long computation, mainly BIG `SELECT`s, your redis instance will be un-responsive serving the data.

This limitation will be definitely remove in a couple of week in Open Source version and more aggresively in the PRO version

Fortunately I already have a working prototype and I only need to time to refine the design and make simple to add futher improvement to the module.

#### Persistency

No form of persistency are provide, right now, by the module.

This means that your in memory database is completely transient.

However, if your database is save on disk also in case of catastrophic failure, each committed statemet will be present on the SQLite file.

Also in this case, fortunately, I am already working on a feasible design to permit both AOF and RDB persistency in both OO and PRO version.


#### Sharding

Right now the module is intended to be used on a sigle machine.

SQL doesn't works flawless in case of tuple distributed in several different host, for a variety of reason.

The module is not going to distribute tuple over different host, however it may distributed databases.

So, each database must be in one and only one host. Multiple databases could be distributed in different host.

## Alpha code

This is extremelly alpha code, there will be definitely some rough edges and some plain bugs.

I really appreciate if you take your time to report those bugs.

A lot of possible functionalities are not yet implemented. By now it is suppose to work on a single redis instance In a future it will be possible to distributed some functionalities of the modules.

## Contributing

I am going to accept pull request here on github.

However since I am going to sell premium version of this products I must ask to every contributer to assign all the rights of the contribution to me.

A pull request template is in place.

## Need incentives

I am not sure, myself, that this module should exist at all. If you find this little module useful or you think that it has some potential, please star the project or open issues requiring functionalities.

## License

This software is licensed under the AGPL-v3, it is possible to buy more permissive licenses.

<RediSQL, SQL capabilities to redis.>
Copyright (C) 2016  Simone Mosciatti


