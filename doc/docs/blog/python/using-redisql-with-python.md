
# Using RediSQL with Python

This tutorial will help you to get start to use RediSQL with Python3.

In this tutorial we will scrape the content of Hacker News using their [API documented here][hn-api].

We will use async python with [`asyncio`][asyncio] to manage the event loop, [`aiohttp`][aiohttp] to retrieve data from a public API and [`aioredis`][aioredis] to communicate with Redis.

To follow this tutorial you will need a modern (> v4.0) instance of Redis running RediSQL.
You can obtain RediSQL from [our shop](https://payhip.com/b/Ri4d) or from the [github releases](https://github.com/RedBeardLab/rediSQL/releases).

To load RediSQL is sufficient to pass it as argument to the redis-server: `./redis-server --loadmodule /path/to/redisql.so`

The whole code show in this example is reachable [here](https://github.com/RedBeardLab/rediSQL/blob/master/doc/docs/blog/python/src/simple.py) while we also created a [more sophisticate example](https://github.com/RedBeardLab/rediSQL/blob/master/doc/docs/blog/python/src/main.py) that stress much more the infrastructure to show that the bottle neck is not RediSQL but python and the network.

## RediSQL and aioredis

Most Redis library implements methods to call the standard Redis command like `SET` or `GET` or `RPOP` and aioredis is not an exception.
This is generally a problem for Redis modules like RediSQL that instead defined their own commands.
Fortunately most libraries usually expose also a lower level method that is used to implement most of the other Redis command.
For what concern `aioredis` the lower level method that we can use is `.execute` that is implemented for both [single connection](https://aioredis.readthedocs.io/en/v1.2.0/api_reference.html#aioredis.RedisConnection.execute) and for a [pool of connections](https://aioredis.readthedocs.io/en/v1.2.0/api_reference.html#aioredis.ConnectionsPool.execute).

Indeed is possible to implement all the other high level command using the low level `.execute` method.

## RediSQL and redis

While in this article we will talk about `aioredis`, another, not asynchronous library for using Redis with python is [`redis` library](https://pypi.org/project/redis/).

In the `redis` library, the low level method is `.execute_command` and not `.execute` as for `aioredis`, other than this difference everything will apply just the same.

## Creating a Redis connection

The very first thing to do is to connect to Redis, in our case we use a connection pool that has the same interface of a simple connection but is backed by a pool of different connections.

Creating the pool can be done like so:

```python
loop = asyncio.get_event_loop()
conn_co = aioredis.create_pool('redis://localhost', minsize=10, maxsize=300, loop = loop)
redis_co = asyncio.gather(*[conn_co])
redis = loop.run_until_complete(redis_co)
redis = redis[0]
```

and now the variable `redis` refer to a `aioredis` pool.

When we will need a new connection, the pool will either give us an idle connection or open a new connection to Redis and give us the new one.

## Setting up RediSQL

Now that we have a pool of connections before to get the data into RediSQL we need to set up RediSQL.
The first step is to create a database in RediSQL, this can be done easily with a call like

```python
await redis.execute("REDISQL.CREATE_DB", "HN")
```

this call will create a new RediSQL database and it will call it `HN`. If the key `HN` already exists the call will return an error.

The next step is to create the structure to hold our data. In our case we will stick to something simple, a single table where we store the identifier of each item (comment or story) from HN, the author of such item, when the item was created and finally we will store the whole item as json structure in a text field.

```python
query = """CREATE TABLE IF NOT EXISTS hn(id integer primary key, author text, time int, item text);"""
await redis.execute("REDISQL.EXEC", "HN", query) 
```

Finally, since we storing data from the open internet inside our database, is wise to create an SQL statement to execute when doing an insert.
The advantage of the statement is that is safe from SQL injections and is usually faster than re-compile the same query each time.

To create a statement we can proceed as following:

```python
statement = "INSERT INTO hn VALUES(?1, ?2, ?3, json(?4));"
await redis.execute("REDISQL.CREATE_STATEMENT", "HN", "insert_item", statement)
```

The last command create a new statement in the `HN` database and associate it with the string `insert_item` so that we can refer to it later.

Also note the use of the `json(?4)` function, this is a function provided by the [JSON1 module][json1] of SQLite and exposed by RediSQL that allow fast and efficient manipulation of json object. 
Using the JSON1 module is possible to have a lot of flexibility even inside a rigid SQL schema.

Is usually wise to wrap those command into a `try: except:` block. Hence the final function will look like this:

```python
async def set_up(redis):
    try:
        await redis.execute("REDISQL.CREATE_DB", "HN")
    except Exception as e:
        print(e)
    
    query = """CREATE TABLE IF NOT EXISTS hn(id integer primary key, author text, time int, item text);"""
    try:
        await redis.execute("REDISQL.EXEC", "HN", query)
    except Exception as e:
        print(e)

    statement = "INSERT INTO hn VALUES(?1, ?2, ?3, json(?4));"
    try:
        await redis.execute("REDISQL.CREATE_STATEMENT", "HN", "insert_item", statement)
    except Exception as e:
        print(e)
```

## Running the loop

Now that we have set up our environment we can go on and start to listen for new items posted on HN.

The API provides a simple endpoint [`maxitem.json`](https://hacker-news.firebaseio.com/v0/maxitem.json) that returns the id of the latest item posted on HN.
When the loop start we get maxitem and we store it into Redis. 
Then, when the maxitem get updated we download each of the items between the `old maxitem` and the `new maxitem`. 

We repeat the loop forever with a sleep to avoid hammering the API endpoint.

```python
async def main(redis, http):
    max_item = await get_max_item(http)
    await redis.execute("SET", "max-item", max_item)
    old_max_item = max_item
    while True:
        # we download the new maxitem
        max_item = await get_max_item(http)
        # if the new maxitem is actually bigger than the old one
        if max_item > old_max_item:
            # for each new item...
            for i in range(old_max_item, max_item):
                # we start a new Task that store the item in our database
                store = store_item(http, redis, str(i))
                asyncio.ensure_future(store)
            await redis.execute("SET", "max-item", max_item)
            old_max_item = max_item
        asyncio.sleep(1)
```

## Storing the data into RediSQL

The last interesting bit is about the `store_item` function that is the one that download the item from the API and store it into RediSQL.


```python
async def store_item(http, redis, item_id):
    item = await get_hn_item(http, item_id)
    await store_on_db(redis, item)

async def get_hn_item(http, item_id):
    while True:
        async with http.get(get_item_url(item_id)) as item:
            if 200 <= item.status < 300:
                data = await item.text()
                item = json.loads(data)
                if item:
                    return item

# In this function we store the item into the RediSQL database
async def store_on_db(redis, item):
    try:
        # execute the statement passing the necessary parameters
        await redis.execute("REDISQL.EXEC_STATEMENT", "HN", "insert_item", 
                item["id"], item.get("by", "_no_author_"), item["time"], json.dumps(item))
    except Exception as e:
        print(e)
        print(item["id"])
```

Downloading the item from the API is a simple HTTP GET request, then we simply check if it returns a successful status code and that it actually returns valid json.

Finally to store the element into RediSQL we execute the statement that we have create before during the set up phase.
Indeed we are executing the command `REDISQL.EXEC_STATEMENT HN insert_item $item_id $item_author $item_time $item`.
This command will find inside the database `HN` the statement `insert_item` that we have previously defined as `INSERT INTO hn VALUES(?1, ?2, ?3, json(?4));`.
Now the item id will be substituted to `?1`, the item author will substitute `?2`, the creation time of the item will take the place of `?3` and the whole json string of the item will substitute `?4`, finally the statement is executed agains RediSQL and its result returned.

If everything went right, we have just added our first row to the database using async python.

# Concluding

In this tutorial we took a rather simple problem and we use it to show how to use RediSQL with async python.

We started by setting up the database, we show how to create a database and tables inside it. Then we also show how to create statements to avoid SQL injections attack and improve the efficiency of repeated queries.

Then we obtain the data from the Hacker News API and we show how to insert the data into RediSQL using the statement that we have just created.

Hopefully this tutorial will be helpful and sufficient to get started, but if you have any question feel free to get in touch or to open an issue.

[asyncio]: https://docs.python.org/3/library/asyncio.html
[aiohttp]: https://github.com/aio-libs/aiohttp/
[aioredis]: https://github.com/aio-libs/aioredis
[hn-api]: https://github.com/HackerNews/API
[json1]: https://www.sqlite.org/json1.html
