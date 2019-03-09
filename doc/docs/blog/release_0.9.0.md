# Release 0.9.0 of RediSQL, SQL steroids for Redis

#### RediSQL, Redis on SQL steroids.

RediSQL is a Redis module that provides full SQL capabilities to Redis, it is the simplest and fastest way to get an SQL database up and running, without incurring in difficult operational issues and it can scale quite well with your business.

The fastest introduction to RediSQL is [our homepage](https://redisql.com)

**tl;dr** This release introduce one simple new command `REDISQL.STATISTICS`.
The new command returns the amount of time each command is been called and how many of those calls were successfully and how many returned errors.
The command does not introduce noticeable slowdowns.

This release is the smallest, however it provide the foundation for the next major releases.

## Motivation

The infrastucture behind the `REDISQL.STATISTICS` commands is needed for the next major release of RediSQL.

Moreover it provides an useful tool for the administrator of the instance allowing them to spot inefficiencies.

## How to use

Just invoke the command without any arguments to get an array of all the counters, extra arguments are ignored for the moment.

After using RediSQL for few commands, the output of `REDISQL.STATISTICS` could be the following.

```
127.0.0.1:6379> REDISQL.STATISTICS
 1) 1) "CREATE_DB"
    2) (integer) 1
 2) 1) "CREATE_DB OK"
    2) (integer) 1
 3) 1) "CREATE_DB ERR"
    2) (integer) 0
 4) 1) "EXEC"
    2) (integer) 4
 5) 1) "EXEC OK"
    2) (integer) 4
 6) 1) "EXEC ERR"
    2) (integer) 0
 7) 1) "QUERY"
    2) (integer) 0
 8) 1) "QUERY OK"
    2) (integer) 0
 9) 1) "QUERY ERR"
    2) (integer) 0
10) 1) "QUERY.INTO"
    2) (integer) 0
11) 1) "QUERY.INTO OK"
    2) (integer) 0
12) 1) "QUERY.INTO ERR"
    2) (integer) 0
13) 1) "CREATE_STATEMENT"
    2) (integer) 3
14) 1) "CREATE_STATEMENT OK"
    2) (integer) 1
15) 1) "CREATE_STATEMENT ERR"
    2) (integer) 2
16) 1) "EXEC_STATEMENT"
    2) (integer) 2
17) 1) "EXEC_STATEMENT OK"
    2) (integer) 2
18) 1) "EXEC_STATEMENT ERR"
    2) (integer) 0
19) 1) "UPDATE_STATEMENT"
    2) (integer) 2
20) 1) "UPDATE_STATEMENT OK"
    2) (integer) 1
21) 1) "UPDATE_STATEMENT ERR"
    2) (integer) 1
22) 1) "DELETE_STATEMENT"
    2) (integer) 0
23) 1) "DELETE_STATEMENT OK"
    2) (integer) 0
24) 1) "DELETE_STATEMENT ERR"
    2) (integer) 0
25) 1) "QUERY_STATEMENT"
    2) (integer) 0
26) 1) "QUERY_STATEMENT OK"
    2) (integer) 0
27) 1) "QUERY_STATEMENT ERR"
    2) (integer) 0
28) 1) "QUERY_STATEMENT.INTO"
    2) (integer) 0
29) 1) "QUERY_STATEMENT.INTO OK"
    2) (integer) 0
30) 1) "QUERY_STATEMENT.INTO ERR"
    2) (integer) 0
31) 1) "COPY"
    2) (integer) 0
32) 1) "COPY OK"
    2) (integer) 0
33) 1) "COPY ERR"
    2) (integer) 0
```

The `CERATE_DB` line means that the `REDISQL.CREATE_DB` command is been invoked once. The `CREATE_DB OK` lines means that the command succeeded once.

Let's analyze the `CREATE_STATEMENT` lines as well.

```
13) 1) "CREATE_STATEMENT"
    2) (integer) 3
```

This line says that the command is been invoked 3 times.

```
14) 1) "CREATE_STATEMENT OK"
    2) (integer) 1
```

The next line specify that the commands completed successfully 1 time out of 3.

```
15) 1) "CREATE_STATEMENT ERR"
    2) (integer) 2
```

The last line confirms that out of the 3 times we invoked the command, 2 of them failed for some reason.

Of course the math need to check out and the sum of successful and erroneous runs should match with the number of invocation.

## Implementation

This command is implemented with atomic counters, they are fast and provide a simple and easy way to manage concurrent access.

We careful tested the performance to make sure that the slowdown introduces by the counter is not noticeable.
