# FAQs 

## Common answer and mistake quickly solved

### ERR - Error the key is empty

You try to execute a command against a database like

```
REDISQL.EXEC DB-EXAMPLE "SELECT 1;"
```

and RediSQL returns the error: `ERR - Error the key is empty`.

Most likely the dabatase `DB-EXAMPLE` does not exists. To fix the problem you can simply create first the database with

```
REDISQL.CREATE_DB DB-EXAMPLE
```

During development is quite convenient to just delete everything from RediSQL, so it may happens that you encounter this error.
A possible solution is to always invoke the `REDISQL.CREATE_DB` command, if the database is not there, it will be created, if the database is already there an error will be raise. As long as you are in a development environment just ignore the error.

### READONLY You can't write against a read only replica.

Redis and RediSQL supports **replication**. You can have the same database in different redis instance, on different processes and potentially on different machine.
This means that you can read data from different instances in parallel, greatly improving reading performances.
However you cannot write in parallel to different instances, otherwise we wouldn't know what data is "real". You can write only to the master instance.

By policy the `REDISQL.EXEC` command allow you to read and write and (due to Redis limitation) you cannot use this command on replicas. The `REDISQL.EXEC` command works only on the master node. This is true even if the query that you are trying to execute is an very simple read only query like `SELECT 1;`, you cannot `REDISQL.EXEC` against a replica node.

In order to read from the replicas, you can use the `REDISQL.QUERY` family of commands. This command is allowed to only read data, without modifying the database, hence you can use it also in the replica instances. Moreover it is a good idea to use it also against the master instance whenever is possible.

If you try to execute the `REDISQL.EXEC` command against a replica you will get the error `READONLY You can't write against a read only replica`. To query the replicas use the `REDISQL.QUERY` command.
