# RediSQL PRO

This document explains the architecture and principle of working of RediSQL PRO.

You can purchase RediSQL PRO, along with support [**here.**][signup]

Motivation and details about the cost are [described here][pro_motivations].

## Main difference

The PRO version offers two main features: **non blocking command** and **replication**

### Non-blocking command

Most command in RediSQL, all but `REDISQL.CREATE_DB`, are blocking command.

This means that Redis block the client, pass the command to a background thread that actually executed it and finally the result is returned to client unblocking it.

This works great in most cases, the main thread of Redis is free of doing other work (like answering standard redis command), there is no difference from the client point of view and your machine can use more than the single thread of redis to work for you.

However, sometimes you want to have non-blocking commands.

The blocking command will be executed in the main redis thread, this means that no other works will be done by Redis while executing your command.

We could expect the non-blocking command to be slightly faster (smaller latency) than the blocking one since there is no need for coordination between threads.

Finally, non-blocking commands are necessary for replication.

Non-blocking commands are invoke adding the `.NOW` suffix.

As an example, instead of `REDISQL.EXEC` that is a blocking command you can use `REDISQL.EXEC.NOW` to use the non-blocking version.

#### When to use non-blocking commands

Non-blocking command takes the priority over blocking one.

Said so, generally, we are expecting users to use mostly the blocking commands.

However, if you need a very quick insert or a very quick lookup, then you should use the non-blocking version.

It is a bad idea to use non blocking commands for slow statements/query.

This because while you are executing a non-blocking command the main redis thread cannot do anything else, this means it cannot answer other redis commands.

### Replication

Redis offers two main methods for persisting data on disk so that in case of power failure of disastrous failure your data are reasonably safe.

RediSQL implement RDB persistency on the community version and AOF replication on the PRO version.

For the details of this two method, I suggest to read the Redis Documentation [on this page][redis_persistence].

The mechanism behind AOF replication is exactly the same behind cluster replication used by redis. The same **bytes** used for AOF replication are also used for cluster replication, just send over different sockets.

For details about cluster replication you can consult the official Redis Documentation on [cluster][redis_cluster] and on [replication][redis_replication]

The PRO version, indeed, implements both AOF and cluster replication.

#### Effective use of Replication

In order to use replication effectively, you should understand a few simple concepts.

If a command is replicated it means that it could be re-executed.

It is **vital** to replicate commands that change the data you are storing, however, is pointless and wasteful to replicate commands that do not apply any change to the data.

You definitely want to replicate every INSERTs, UPDATEs or DELETEs while you should avoid replicating SELECTs.

Replicated commands are usually executed either when you are re-loading your dataset after some sort of failures or in slaves/replica with a train of other replicated commands is coming right after.

Consider what happens if you replicate a big SELECT. RediSQL is going to execute it and it is going to take some time, this while your application is waiting for redis to restart or when a train of replicated commands are piling up in the slaves/replicas buffers. And all this just for discard the result of the SELECT itself.

In order to avoid this effect is a good idea to use the query commands whenever possible ([`REDISQL.QUERY`][query] and [`REDISQL.QUERY_STATEMENT`][query_statement]), this command **do not** replicate and are marked as `readonly` which means that can be executed also on slaves/replicas providing interesting primitives of load balancing. (Eg. You could write on the master and read on the slaves.)

[query]: references.md#redisqlquery
[query_statement]: references.md#redisqlquery_statement
[pro_motivations]: pro_motivations.md
[redis_persistence]: https://redis.io/topics/persistence
[signup]: https://plasso.com/s/epp4GbsJdp-redisql/signup/
[redis_cluster]: https://redis.io/topics/cluster-tutorial
[redis_replication]: https://redis.io/topics/replication
