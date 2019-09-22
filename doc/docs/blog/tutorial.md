# Tutorial for RediSQL

If you want to start exploring RediSQL, this tutorial will walk you through the most important commands to know and how to use them.

The tutorial can be followed also connecting on our demo machine @ demo.redisql.com:

```
redis-cli -h demo.redisql.com
```

If you connect to the demo machine, please remember that it is a shared machine and other users can be using it at the same time. Also note that we clean up the machine every hour.

<iframe width="560" height="315" src="https://www.youtube.com/embed/YRusC-AIq_g" frameborder="0" allow="accelerometer; autoplay; encrypted-media; gyroscope; picture-in-picture" allowfullscreen></iframe><Paste>

## Create your database

The first fundamental concept in RediSQL is the concept of database.

One database is a SQLite database that is associated with a Redis key, creating a new database also means to spawn a new thread that will be the one that actually does the work leaving the main Redis thread available to answer standard Redis queries.

To create a new RediSQL database just run:

```
REDISQL.CREATE_DB TUTORIAL

OK
```

This will create a new in-memory database associated with the name `TUTORIAL`.

(If you receive an error at this stage, most likely someone else is following the tutorial, no worries, instead of `TUTORIAL` simply use another name).

## First interaction

Now that we have create a new database, we can interact with it.

The simplest command to use is the `REDISQL.EXEC` command that will execute one query against our newly created database, try:

```
REDISQL.EXEC TUTORIAL "SELECT 1;"

1) 1) (integer) 1
```

This command should simply return 1.

What we can do, is to create a simple SQL table.

```
REDISQL.EXEC TUTORIAL "CREATE TABLE first_table(a integer, b string);"

1) DONE
2) (integer) 0
```

Congrats, you have just created a new table.

## Insert values

After we create our first table, we can insert some value into it.

```
REDISQL.EXEC TUTORIAL "INSERT INTO first_table VALUES(1, 'the first row'), (2, 'the second row');"

1) DONE
2) (integer) 2
```

This command will return 2, the number of rows inserted.

## Query values

Now we can try to select some values from our table, as an example:

```
REDISQL.EXEC TUTORIAL "SELECT * FROM first_table;"

1) 1) (integer) 1
   2) "the first row"
2) 1) (integer) 2
   2) "the second row"
```

Another way to select values is to use the `QUERY` command, this command works only if the query you send is a read-only query.

```
REDISQL.QUERY TUTORIAL "SELECT * FROM first_table WHERE a = 1;"

1) 1) (integer) 1
   2) "the first row"
```

## Statements

Writing this commands all over again get tedious, moreover, it is inefficient since the SQL engine needs to parse and compile the SQL query every time. The solution, in this case, is the use of statements.

```
REDISQL.CREATE_STATEMENT TUTORIAL filter_on_a "SELECT * FROM first_table WHERE a = ?1;"


OK
```

The statements can both query (as above) or insert rows:

```
REDISQL.CREATE_STATEMENT TUTORIAL new_row "INSERT INTO first_table VALUES(?1, 'insert_from_statement');"

OK
```

Statements can be invoked using:

```
REDISQL.EXEC_STATEMENT TUTORIAL new_row 5

1) DONE
2) (integer) 1
```

Or if they are read-only statements:

```
REDISQL.QUERY_STATEMENT TUTORIAL filter_on_a 5

1) 1) (integer) 5
   2) "insert_from_statement"
```

## Push to streams

Sometimes you may want to store the result of your queries in Redis itself, to consume it little by little, or for caching. In such cases, the solution is to push the results into a Redis stream.

```
REDISQL.QUERY.INTO tutorial_stream TUTORIAL "SELECT * FROM first_table;"

1) 1) "tutorial_stream"
   2) "1569174107783-0"
   3) "1569174107783-2"
   4) (integer) 3
```

This will push the result of the query into the stream `tutorial_stream` and it will return:
1. The name of the stream
2. The first ID
3. The last ID
4. The total number of elements added

In order to retrieve the values from the Redis stream, you can use the standard Redis API:

```
XRANGE tutorial_stream - +

1) 1) "1569174107783-0"
   2) 1) "int:a"
      2) "1"
      3) "text:b"
      4) "the first row"
2) 1) "1569174107783-1"
   2) 1) "int:a"
      2) "2"
      3) "text:b"
      4) "the second row"
3) 1) "1569174107783-2"
   2) 1) "int:a"
      2) "5"
      3) "text:b"
      4) "insert_from_statement"
```

# End

I hope that this tutorial was helpful :)

If you have any question, feel free to contact me simone@redbeardlab.com or on github: [RedBeardLab/rediSQL](https://github.com/RedBeardLab/rediSQL)
