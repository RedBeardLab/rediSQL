# Release 0.6.0 of RediSQL, SQL steroids for Redis

#### RediSQL, Redis on SQL steroids.

RediSQL is a Redis module that provides full SQL capabilities to Redis, it is the simplest and fastest way to get an SQL database up and running, without incurring in difficult operational issues and it can scale quite well with your business.

The fastest introduction to RediSQL is [our homepage](https://redisql.com)

**tl;dr** This release does not introduce new commands, but it provides a SQLite virtual table implementation that allows making SQL queries against Redis Hashes.
The release is important because set the foundation to write more complex commands or SQLite functions.
Possible ideas could be SQLite functions that append to a list or to a stream, these functions could be used inside triggers to generate an event log of all the operation that happened to a particular table.

## Virtual Table

Inside RediSQL is now possible to use the virtual table: [REDISQL_TABLES_BRUTE_HASH][redisql_table_brute_hash].

This virtual table allows to only query Redis hashes that follow a common structure.

The understood structure is:

```
HSET $tableName:$id $col1 $val1 $col2 $val2 ... $colN $valN
```

Where the `$col`s are constant in the hashes and, of course, the `$val`s change from row to row.

In order to create a [REDISQL_TABLES_BRUTE_HASH][redisql_table_brute_hash] the syntax is the following:

```
CREATE VIRTUAL TABLE funny_cats USING REDISQL_TABLES_BRUTE_HASH($tableName, $col1, $col2, ..., $colN);
```

Please note that the first parameter of the virtual table is not, as we could expect, the first column of the table, but is the name of hashes that we want to use as table, of course without specifying any `$id`.

Also note that is pointless to provide a type to the columns since Redis does store only strings inside the hashes, hence you will get only strings from the virtual table as well.

What you can do to get numbers, integer or floats, is to exploit the [`CAST`][sqlite_cast] capabilities of SQLite.

You can find examples of this feature in [the documentation.][redisql_table_brute_hash]

Let me make clear that this virtual table does **not** implements updates, inserts or deletes, at the moment you can only query this type of virtual tables.

The implementation of update and inserts and deletes should not pose significant challenges.

## Importance of this release

This release is extremely important for architectural reasons inside the module itself.

In order to implement the above virtual table was necessary to keep a pointer to an internal structure of Redis that actually allow calling any Redis command from inside a module.

Including this pointer into the RediSQL structures make possible to call arbitrary Redis commands.

This opens the gate to quite interesting features, as an example, imagine to be able to call `LPUSH` or `XADD` inside a trigger.

This will allow to log every operation you are doing against your dataset. You could replay them later in a different instance of RediSQL or maybe also against a different database.

You could write all you operation very fast in memory using RediSQL and when you have enough of them write them to disk against PostgreSQL, MySQL or any other database.

## End

As always you can find all the public releases on the [github page][releases], you can openly access the same public release on the [open page of our shop][plaso_open] or you can buy the complete PRO package [signing up in the shop][plaso_signup].

Remember that signing up for the PRO product also provide you free support from us, the creator of the project, so that we can point you to the right direction and suggest the best use cases for our product.

[redisql_table_brute_hash]: ../references.md#redisql_tables_brute_hash
[sqlite_cast]: https://www.sqlite.org/lang_expr.html#castexpr
[releases]: https://github.com/RedBeardLab/rediSQL/releases/tag/v0.5.0
[plaso_open]: https://plasso.com/s/epp4GbsJdp-redisql/
[plaso_signup]: https://plasso.com/s/epp4GbsJdp-redisql/signup/
