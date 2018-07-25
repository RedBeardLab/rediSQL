# JSON on Redis via RediSQL, SQL steroids for Redis

#### RediSQL, Redis on SQL steroids.

RediSQL is a redis module that embeds SQLite to provide full SQL capabilities to redis.

The fastest introduction to RediSQL is [our homepage](/)

tl;dr; We build a **JSON as a Service** in less than 500 lines of javascript + RediSQL, you can check out the [whole source file here.][jaas]

## JSON as a Service in 500 lines of code + RediSQL

While building web services is common to have the need to store some un-structured or semi-structured data (aka **JSON**) somewhere.

Unfortunately, it is not always so easy.

If you are using a SQL database, think about postgres, you need to add a column or even a table to your database, you need to decide how to encode the data and to be sure that your drivers work correctly with this new type.

If you are already using some kind of NoSQL database you may be a little luckier, but nevertheless, you should be sure that your database support all the operation you may need on JSON and still you need your team on-board to add a new collection/fields/column to your actual database.

## A faster solution

A faster solution could be to use RediSQL exploiting the [JSON1][JSON1] extension.

SQLite provides several interesting extensions and one of our favourites is JSON1 that allow an efficient and fast manipulation of JSON data, all inside a full SQL engine.

We include this extension by default in RediSQL, so, if you are using RediSQL, you already have all the necessary function.

## Desiderata

In this example we assume that you are sharing your JSON data store between different part of the system, hence you need some form of hierarchy.

We will create JSON object that will have names and then each object will live inside a namespace, the couple `(namespace, name)` will be unique while name could be repeated inside different namespaces.

Then we will like to have a simple RediSQL interface like this one:

```
127.0.0.1:6379> REDISQL.EXEC_STATEMENT DB create_namespace noises
1) DONE
2) (integer) 1
127.0.0.1:6379> REDISQL.EXEC_STATEMENT DB upsert_object noises animals '{"cat": "meeow", "dog": "woof", "goldfish": "..."}'
1) DONE
2) (integer) 1
127.0.0.1:6379> REDISQL.EXEC_STATEMENT DB upsert_object noises humans '{"extrovert": "blablabla", "introverse": "bla", "programmer": "tap tap tap"}'
1) DONE
2) (integer) 1
127.0.0.1:6379> REDISQL.EXEC_STATEMENT DB extract noises humans $.extrovert
1) 1) "blablabla"
127.0.0.1:6379> REDISQL.EXEC_STATEMENT DB extract noises animals $.dog
1) 1) "woof"
127.0.0.1:6379> 
```

Of course, we would also like to navigate complex JSONs and to add fields and values at will.

```
127.0.0.1:6379> REDISQL.EXEC_STATEMENT DB create_namespace foo
1) DONE
2) (integer) 1
127.0.0.1:6379> REDISQL.EXEC_STATEMENT DB upsert_object foo bar '{"a": {"quite": ["", {"complex": {"json": [1, 2, "object"]}}, ""]}}'
1) DONE
2) (integer) 1
127.0.0.1:6379> REDISQL.EXEC_STATEMENT DB extract foo bar $.a.quite[1].complex.json[2]
1) 1) "object"
127.0.0.1:6379> REDISQL.EXEC_STATEMENT DB set foo bar $.a.quite[1].complex.json[3] '["even", "more", "complex"]'
1) DONE
2) (integer) 1
127.0.0.1:6379> REDISQL.EXEC_STATEMENT DB extract foo bar $.a.quite[1].complex.json[3] 
1) 1) "[\"even\",\"more\",\"complex\"]"
127.0.0.1:6379> REDISQL.EXEC_STATEMENT DB extract foo bar $.a.quite[1].complex.json[3][0]
1) 1) "even"
127.0.0.1:6379> REDISQL.EXEC_STATEMENT DB extract foo bar $.a.quite[1].complex.json[3][1]
1) 1) "more"
127.0.0.1:6379> REDISQL.EXEC_STATEMENT DB extract foo bar $.a.quite[1].complex.json[3][2]
1) 1) "complex"
```

In this specific example we are only showing JSON, but keep in mind that the JSON field can be stored alongside the regular SQL fields as an extra field to hold any kind of unstructured data.

Also please note how these APIs are quite pleasant to work with, they seem almost native to redis and thanks to redis-module we have the possibility to simply create more powerful commands. 

## Implementation

Now that we know what we are trying to achieve let's proceed to the implementation that you will see is quite simple.

Usually is a good idea to start from the data structure, and in our simple, but powerful, example, we need only a single table:

``` SQL
CREATE TABLE IF NOT EXISTS namespace ( 
    namespace TEXT PRIMARY KEY 
); 

CREATE TABLE IF NOT EXISTS json_data (
    namespace STRING,
    object_name STRING,
    data JSON,
    PRIMARY KEY (namespace, object_name),
    FOREIGN KEY(namespace) REFERENCES namespace(namespace) ON UPDATE CASCADE ON DELETE CASCADE
);
```

We could have done everything with just `json_data` and without `namespace`, but that would force to have the namespace that always contains at least a single object and everything would become more complex.

Now that we have our data structure we can proceed with the procedures that I have show you above:

``` SQL
-- create_namespace
INSERT INTO namespace VALUES(?1);

-- upsert_object
INSERT OR REPLACE 
        INTO json_data (namespace, object_name, data)
        VALUES (?1, ?2, json(?3))

-- get_object
SELECT data 
        FROM json_data 
        WHERE namespace = ?1 AND
        object_name = ?2;

-- extract
SELECT json_extract(data, ?3) 
        FROM json_data 
        WHERE namespace = ?1 AND 
        object_name = ?2;

-- set
UPDATE json_data 
        SET data = json_set(data, ?3, json(?4))
        WHERE namespace = ?1 AND 
        object_name = ?2 AND
        data != json_set(data, ?3, json(?4)); -- if no modification, don't change the object
```

In order to actually create those table here is an example on RediSQL that is more difficult to read but simpler to just copy and paste into the redis-cli.

```
127.0.0.1:6379> REDISQL.CREATE_DB DB
OK
127.0.0.1:6379> REDISQL.EXEC DB "CREATE TABLE IF NOT EXISTS namespace (namespace TEXT PRIMARY KEY);" 
1) DONE
2) (integer) 0
127.0.0.1:6379> REDISQL.EXEC DB "CREATE TABLE IF NOT EXISTS json_data (namespace STRING, object_name STRING, data JSON, PRIMARY KEY (namespace, object_name));"
1) DONE
2) (integer) 0
127.0.0.1:6379> REDISQL.CREATE_STATEMENT DB create_namespace "INSERT INTO namespace VALUES(?1);"
OK
127.0.0.1:6379> REDISQL.CREATE_STATEMENT DB upsert_object "INSERT OR REPLACE INTO json_data (namespace, object_name, data) VALUES (?1, ?2, json(?3))"
OK
127.0.0.1:6379> REDISQL.CREATE_STATEMENT DB get_object "SELECT data FROM json_data WHERE namespace = ?1 AND object_name = ?2;"
OK
127.0.0.1:6379> REDISQL.CREATE_STATEMENT DB extract "SELECT json_extract(data, ?3) FROM json_data WHERE namespace = ?1 AND object_name = ?2;"
OK
127.0.0.1:6379> REDISQL.CREATE_STATEMENT DB set "UPDATE json_data SET data = json_set(data, ?3, json(?4)) WHERE namespace = ?1 AND object_name = ?2 AND data != json_set(data, ?3, json(?4));"
OK
```

Of course, there are a lot more commands in JSON1 API to use and explore, so I will simply [leave you the reference][JSON1].

I also prepare a simple node application which exposes this exact same interface via REST API, it is a single, ~500 LOC, file that you can find [here][jaas]

Feel free to use the node application as a blueprint for your next project.

## Recap

In this brief tutorial, we have shown how quickly and easily is possible to build a fairly complex JSON store using Redis and RediSQL.

Of course, similar structures, procedure and ideas can be used inside bigger structures and data table yielding powerful primitives for your application capable of sustain quite reasonable load without incurring in any extra operational cost.

## Question?

Of course, if you have any question on RediSQL either open a public issue or write me, siscia, a private email.

Cheers,

;)

[JSON1]: https://www.sqlite.org/json1.html
[jaas]: https://github.com/RedBeardLab/JaaS/blob/master/index.js
