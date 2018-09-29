# Release 0.7.0 of RediSQL, SQL steroids for Redis

#### RediSQL, Redis on SQL steroids.

RediSQL is a Redis module that provides full SQL capabilities to Redis, it is the simplest and fastest way to get an SQL database up and running, without incurring in difficult operational issues and it can scale quite well with your business.

The fastest introduction to RediSQL is [our homepage](https://redisql.com)

**tl;dr** This release introduce a new commands [`REDISQL.COPY`][redisql_copy] that copy the content from a source database into a destination database.


## Motivations

Since from the very first release and the very first user, we have been asked a lot about the possibility to copy SQLite database into disk or into memory.

It is definitely an useful feature, suppose to already have the database and you simply want to make it available to some of your services.

We wait a bit before to incorporate on RediSQL such capabilities, mostly because we weren’t sure about the API to offer.

Finally we decide to pull the trigger and we implemented a new command [`REDISQL.COPY`][redisql_copy].

The [`REDISQL.COPY`][redisql_copy] command takes two parameters as input, a `source` database and a `destination` database and it overwrite the content of the `source` database into the `destination` database.

It is important to understand that the content of the destination database is completely lost after a `REDISQL.COPY`

The [`REDISQL.COPY`][redisql_copy] command takes as input two databases, both of them must be created using the `REDISQL.CREATE_DB` command. 
This API allows several use cases that are quite interesting.

1. Make a backup/copy of your database
2. Split load to multiple threads
3. Move a database from a in-memory database to a disk-based database
4. Move a database from  a disk-based database to a in-memory database


### Few Examples

I will show briefly some examples of those use cases.

Making a backup/copy 

Backups are already provided by the internal of Redis itself, all the database will be copied into the RDB files.
However you may be interested in having just a copy of your database, so that you can archive it in a different way, or just explore it offline.

Suppose you have your database `DB` running with some table and some data:


    > REDISQL.CREATE_DB DB
    OK
    > REDISQL.EXEC DB "CREATE TABLE foo( ... )"
    DONE
    0L
    > REDISQL.EXEC DB "INSERT INTO foo VALUES ( ... )"
    DONE
    1L

Now you will like to transfer that same database into a file, so that you can archive it or analyze it.

The first step would be to create another database backed by a file.


    > REDISQL.CREATE_DB BACKUP "/home/foo/backup.sqlite"
    OK

In this way we have created a new, empty database that is backed by a file.

You will see the small file `home/foo/backup.sqlite`

At this point you just need to make a copy of it.


    > REDISQL.COPY DB BACKUP
    OK

Now the file `/home/foo/backup.sqlite` will contains all the data that were originally on the `DB` database.

### Load a database

Now, suppose that the data you want to serve via RediSQL are already inside a SQLite database, or suppose that you are recovering from a previous backup. However you would like to have the database in memory, since we know the load will be quite high.

Assuming  your database is stored into `/home/foo/recover.sqlite` we start by loading it, and then move it into an in-memory database, and finally we can also delete the database we used for recovering.


    > REDISQL.CREATE_DB TO_RECOVER "/home/foo/recover.sqlite"
    OK
    > REDISQL.CREATE_DB DB
    OK
    > REDISQL.COPY TO_RECOVER DB
    OK
    > DEL TO_RECOVER
    OK

At this point we have only one database `DB` that is an in-memory one and we have used the `TO_RECOVER` database to load the recovering file.

### Spread load

Another quite interesting use case is about load spreading.

Suppose to have a read-only database `DB1` that makes quite complex and long queries, if that start to be a problem we could spread the load into two identical databases.


    > REDISQL.CREATE_DB DB2
    OK
    > REDISQL.COPY DB1 DB2
    OK

Now we have the same dataset in two different database, each one of them with its own thread of execution. This will allow us to round robin between the two databases and achieve smaller latencies.


## End

With this post we showed the newest features of RediSQL.

The product start to be quite stable, more performance test will come in the next release but we don’t plan to touch the API.

If we don’t change the API the next release will be the v1.0.0

As always you can find all the public releases on the [github page][releases], you can openly access the same public release on the [open page of our shop][plaso_open] or you can buy the complete PRO package [signing up in the shop][plaso_signup].

Remember that signing up for the PRO product also provide you free support from us, the creator of the project, so that we can point you to the right direction and suggest the best use cases for our product.

[redisql_copy]: ../references.md#redisqlcopy
[sqlite_cast]: https://www.sqlite.org/lang_expr.html#castexpr
[releases]: https://github.com/RedBeardLab/rediSQL/releases/tag/v0.5.0
[plaso_open]: https://plasso.com/s/epp4GbsJdp-redisql/
[plaso_signup]: https://plasso.com/s/epp4GbsJdp-redisql/signup/
