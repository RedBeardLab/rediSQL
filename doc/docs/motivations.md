# Motivation

This document explains the motivations behind this redis module.

## My personal use case

As a lot of different open source projects, this module is born out of a personal issue that I was trying to solve.

I was developing a very simple application using a microservice architecture, each service needed to be stopped and updated at will so it was mandatory to store all the state in an external application.

Redis was perfect for this use case since it is very simple to operate, you could get away simply setting your level of persistence, extremely stable, very performant and there are bindings ready for basically any programming language.

However, the application started to grow in terms of complexity and soon I realized that having a small SQL engine would have saved me a lot of complexity in my code while delivering better performances.

At that time I had only the following options:

1. Keep all the state in Redis, implementing by hand, or using some external library, whatever SQL-like transformation I needed.
2. Bring in another piece inside my architecture, namely an SQL database.

For some project it may be worth to immediately include an external dependency in the form of a database, but it brings up the cost of operating the infrastructure.

Operating a database is quite complex, operating it in any organization costs in terms of human resources or, if you use managed services, directly in terms of money.

Also, since all my state was kept only in Redis, introducing another "source of truth" would have complicated the code base.

My project definitely didn't need the whole computing power of Postgresql or of MySQL, I didn't need the burden of operating it and definitely I wasn't in the condition to pay for managed services.

## Why RediSQL

The goal of the module is to create a third alternative to the two mentioned above.

I wanted this alternative to be as low maintenance as possible, keep a great level of security on the persistency of the data stored and to be easily deployed in most architectures.

SQLite easily checks both the low maintenance and the high level of persistency requirements. Redis is already deployed in most architectures, either as a cache layer or as a database.

Finally, merging the two project was just made possible by the introduction of the Redis modules.

Hence, RediSQL was born.

## Possible uses

RediSQL has been thought to be used as an in-memory SQL database, shared between multiple (micro-)services.

However, RediSQL inherits the persistency capabilities of Redis, supporting RDB and AOF, and of SQLite, with the possibility to write directly on disk.

Moreover, it basically never uses the main thread of Redis, hence it will not affect the performance of Redis itself.

This makes RediSQL a reasonable solution to store and persist data in a small to a medium modern project. 



