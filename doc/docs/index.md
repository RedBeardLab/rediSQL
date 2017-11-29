# RediSQL

RediSQL is a redis module that embed a fully functional SQLite database.

## Motivation

The main motivation behind the project is to provide a quick and hands-off environment to store structured data.

It also turns out that RediSQL is a great way to cache your content and data in a more structured way.

## Susteinable Open Source

The project is based on the idea of susteinable Open Source.

The project provides two versions, an open source one, which is enough for most simple projects, and a PRO version that provide features required from companies and enterprises.

# Overview

In this section we are going to explore the main concepts in the module.

## Databases

RediSQL provide the concept of database.

It is possible to create a new database with the command `REDISQL.CREATE_DB`.

The database get associate with a Redis key and so it is possible to have multiple SQL databases in a single Redis instance.

Also it is possible to use in-memory database, wich is the default, or databases backed by a real file. In-memory databases are generally a little faster but they are limited by the ammount of memory your server has. Database backed by files are a little slower but they can grow basically indefinitely.

## Exec

`REDISQL.EXEC` is the command that let you execute command against a SQL database.

It is usefull when you are testing the module or when you are changing the settings of the databases throught SQLite `PRAGMA`s.

However I would not suggest to use them in production since there are better tools like `Statements`.

## Statements

Queries and stataments can be precompile and stores inside the Redis key in order to provide a faster execution and more agility in your application.

When you execute a SQLite query, the text is compiled to a binary code, this binary code is then executed against the database and the result provide as answer.
The phase of compilation can be quite expensive, but if you always execute the same statements (think about `inserts`), it can be avoided.

When you use `REDISQL.CREATE_STATEMENT` your statement get compiled, then when you execute it using `REDISQL.EXECUTE_STATEMENT` it is not re-compiled but we use the pre-compiled one. It seems a trivial change but it will really speed up some workload.

Statements can also be used as an interface for different application using the same RediSQL instance.

Once you define the interface of the statement and its behavior, then you are free to change it's implementation  while maintain all the legacy code working.
This is quite useful esepcially if you have several services using the same RediSQL instance.

# Persistency

The module in the community version implements only RDB. However the PRO version provide also AOF and replication.

## RDB

The module implements RDB persistency.

When Redis starts to save the RDB file the status of the database get serialized and written, along with all the other information, in the RDB file.

## AOF

AOF replication is provided only in the PRO edition.

At the moment all the commands are replicated, this is quite a waste and we are moving to replicate only the commands that actually modify the codebase.

With AOF replication you also get instance replication that allows to replicate the same dataset into different redis instances.
