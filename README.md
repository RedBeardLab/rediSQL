# RediSQL

RediSQL is a redis module that embeded SQLite.

_With great powers comes great responsability_ (cit. Uncle Ben)

Redis is born as a NoSQL database. This same module is already bordeline with the respect of the **common sense** that we, as community, should have. 

Please, use but do not abuse RediSQL.

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
$ ~/redis-4.0-rc1/src/redis-server --loadmodule ./rediSQL.so 
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

6833:M 15 Dec 16:25:53.196 # WARNING: The TCP backlog setting of 511 cannot be enforced because /proc/sys/net/core/somaxconn is set to the lower value of 128.
6833:M 15 Dec 16:25:53.196 # Server started, Redis version 3.9.101
6833:M 15 Dec 16:25:53.196 # WARNING overcommit_memory is set to 0! Background save may fail under low memory condition. To fix this issue add 'vm.overcommit_memory = 1' to /etc/sysctl.conf and then reboot or run the command 'sysctl vm.overcommit_memory=1' for this to take effect.
6833:M 15 Dec 16:25:53.196 # WARNING you have Transparent Huge Pages (THP) support enabled in your kernel. This will create latency and memory usage issues with Redis. To fix this issue run the command 'echo never > /sys/kernel/mm/transparent_hugepage/enabled' as root, and add it to your /etc/rc.local in order to retain the setting after a reboot. Redis must be restarted after THP is disabled.
6833:M 15 Dec 16:25:53.197 * Module 'rediSQL__' loaded from ./rediSQL.so
6833:M 15 Dec 16:25:53.197 * The server is now ready to accept connections on port 6379
```

Now the redis instance will be just the redis you learn to love:

```
$ ~/redis-4.0-rc1/src/redis-cli 
127.0.0.1:6379> 
127.0.0.1:6379> SET A 3
OK
127.0.0.1:6379> GET A
"3"
```

But it will also able to accept SQL statements:

```
127.0.0.1:6379> 
# Start creating a table
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

Of course also errors are managed:

```
127.0.0.1:6379> REDISQL.EXEC "INSERT INTO baz VALUES("aaa", "bbb");"
Invalid argument(s)
127.0.0.1:6379> 
127.0.0.1:6379> REDISQL.EXEC "CREATE TABLE baz(f INT, k TEXT);"
(error) ERR - table baz already exists | Query: CREATE TABLE baz(f INT, k TEXT);
```


Now you can create tables, insert data on those tables, make queries, remove elements, everything.

## RoadMap

The very first step will be to add data persistency that we believe to be fundamental even if option for some use cases optional.

Then we would like to move following the necessity of the community, so ideas and use cases are extremely welcome.

We do have already a couple of ideas: 

1. Introducing concurrency and non-blocking queries. 

2. Stream all the statements that modify the data, everything but `SELECT`s.

3. A cache system to store the result of the more complex select.

But please share your thoughts.

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


