# RediSQL

RediSQL is a redis module that embeded SQLite.

_With great powers comes great responsability_ (cit. Uncle Ben)

Redis is born as a NoSQL database, this same module is already bordeline with the respect of the **common sense** that we, as community, should have. 

Use but do not abuse RediSQL please.

## Motivation

I love the agility provided by Redis, however, several times, I wished I had a little more structure in my in-memory database.

Even basic SQL is very powerful and years upon years of experience on several SQL implementation have bring us very mature product that we can now exploit with confidence.

Between all the SQL implementation, the one that best fitted the need for this module is definitely SQLite, for its velocity, portability, simplicity and capability to work in memory.

## OpenSource and the necessity of real support and charge for my time.

[How to Charge for your Open Source](http://www.mikeperham.com/2015/11/23/how-to-charge-for-your-open-source/) by Mike Perham brings well argumented thoughts on the necessity to charge for work done by developers, even in the Open Source world.

I have started, myself, a lot of Open Source project that, eventually, are all death because I wasn't able to dedicate the right ammount of time to them.

I am hoping to find the necessary funds to keep maintain this project.

I am starting with only an Open Source version, and then move to an enterprise version adding the necessary features.

## Usage

From the root of the project you should be able to compile and link the module, all with a simple:
``` bash
make
```

Now a rediSQL.so object should have been generate, and you simply load the module inside Redis.

Note that you need the last unstable redis branch in order of modules to work. 

```bash
./redis-server --loadmodule ~/rediSQL/rediSQL.so
```

Now, your Redis instance has loaded the module and it is capable of accept SQL queries and execution.

```bash
./redis-cli

> REDISQL.EXEC "YOUR_QUERY_HERE"
```

Now you can create tables, insert data on those tables, make queries, remove elements, everything.

## Bug and Error

This is extremelly alpha code, there will be definitely some rough edges and some plain bugs, I really appreciate if you take your time to report those bugs.
