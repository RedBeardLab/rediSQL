# RediSQL

[RediSQL][github] is a Redis module that embeds a fully functional SQLite database.

At the best of our knowledge is the only system that provides SQL capabilities while being very fast so to be used as a cache, simple to integrate with any programming language, since it can be used by every redis client, and with very very low maintenance.

Moreover, for small, not critical services, it can also be used as the main database, it can store data not only in memory but also on file and it can also use the same persistence mechanisms of redis itself.

## Obtain

There are **two version** of the software, a "community", completely open source version and a PRO version that comes with **more features and support plan.**

Both versions can be [**obtained in the store.**][store]

For the community version, you can just download it, we ask for a small donation if you can support the project but feel free to just input 0€ and download it.

For the PRO version you need to [**sign up here**][signup], after you signed up you will be able to download the software.

A detailed coverage of the PRO version [is provided here][pro]

The motivations of why we decided to sell a PRO version and about its price are [here][pro_motivations] and we hope that you agree on our points.

## Motivation

The main motivation behind the project is to provide a quick and hands-off environment to store structured data.

It also turns out that RediSQL is a great way to cache your content and data in a more structured way.

The main history and motivation of the project are explained [in this page.][motivations]

# Overview

In this section, we are going to explore the main concepts in the module.

There is another section of the website, [the reference][ref], that explore every single command that the module provides giving a deeper explanation of every detail.

## Databases

RediSQL provides the concept of database.

It is possible to create a new database with the command [`REDISQL.CREATE_DB`][r:create_db].

The database is associated with a Redis key and so it is possible to have multiple SQL databases in a single Redis instance.

Also, it is possible to use in-memory database, which is the default, or databases backed by a real file. In-memory databases are generally a little faster but they are limited by the amount of memory your server has. Database backed by files are a little slower but they can grow basically indefinitely.

## Exec

[`REDISQL.EXEC`][r:exec] is the command that let you execute command against a SQL database.

It is useful when you are testing the module or when you are changing the settings of the databases through SQLite `PRAGMA`s.

However, I would not suggest to use them in production since there are better tools like `Statements`.

## Statements

Queries and statements can be precompiled and stores inside the Redis key in order to provide a faster execution and more agility in your application.

When you execute an SQLite query, the text is compiled to a binary code, this binary code is then executed against the database and the result provide an answer.
The phase of compilation can be quite expensive, but if you always execute the same statements (think about `inserts`), it can be avoided.

When you use [`REDISQL.CREATE_STATEMENT`][r:create_statement] your statement is compiled, then when you execute it using [`REDISQL.EXEC_STATEMENT`][r:exec_statement] it is not re-compiled but we use the pre-compiled one. It seems a trivial change but it will really speed up some workload.

Statements can also be used as an interface for different applications using the same RediSQL instance.

Once you define the interface of the statement and its behaviour, then you are free to change it's implementation while maintaining all the legacy code working.
This is quite useful especially if you have several services using the same RediSQL instance.

## Query

In most databases there are statements that modify the data and queries that simply read.

Of course, just reading, is usually a faster and simpler operation than modify the data. In order to take advantages of this, we provide a different command [`REDISQL.QUERY`][r:query] and [`REDISQL.QUERY_STATEMENT`][r:query_statement] that constraint you to don't modify the data.

These commands allow you to have slaves/replicas serves query and to balance some load off the master node for better speed and reliability.

# Persistency

The module in the community version implements only RDB. However, the PRO version provides also AOF and replication.

## RDB

The module implements RDB persistency.

When Redis starts to save the RDB file the status of the database get serialized and written, along with all the other information, in the RDB file.

## AOF

AOF replication is provided only in the PRO edition.

All the commands are replicated, but the read-only ones.

With AOF replication you also get instance replication that allows replicating the same dataset into different Redis instances in a master/slave fashion.

# PRO

The PRO edition is based on the Open Source one, however, it provides one more class of commands that are necessary for business or where rediSQL is a critical piece of the infrastructure.

Every command, but `REDISQL.CREATE_DB`, blocks the clients and it is executed in the background by a different thread.

With the PRO version, we also provide the `.NOW` commands that are executed immediately without blocking the client.

Every command in the PRO version provides the `.NOW` variant, but please refer to the [reference][ref].

Moreover, the PRO version also provides AOF replication, that, indeed, necessitate of commands that don't block the clients.

More information about the PRO version are available [here.][pro]

[github]: https://github.com/RedBeardLab/rediSQL
[ref]: references.md
[r:create_db]: references.md#redisqlcreate_db
[r:exec]: references.md#redisqlexec
[r:create_statement]: references.md#redisqlcreate_statement
[r:exec_statement]: references.md#redisqlexec_statement
[r:query]: references.md#redisqlquery
[r:query_statement]: references.md#redisqlquery_statement
[motivations]: motivations.md
[store]: https://plasso.com/s/epp4GbsJdp-redisql
[signup]: https://plasso.com/s/epp4GbsJdp-redisql/signup/
[pro]: pro.md
[pro_motivations]: pro_motivations.md
