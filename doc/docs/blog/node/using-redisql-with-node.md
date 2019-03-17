
# Using RediSQL with Node.js

This tutorial will help you to get start to use RediSQL with Node.js

In this tutorial we will scrape the content of Hacker News using their [API documented here][hn-api].

To communicate with Redis we will use the popular [node_redis](https://github.com/NodeRedis/node_redis) library wich is the most suitable to communicate with Redis Modules and RediSQL. Other libraries can be used as well, but some more work will be necessary.

To follow this tutorial you will need a modern (> v4.0) instance of Redis running RediSQL.
You can obtain RediSQL from [our shop](https://payhip.com/b/Ri4d) or from the [github releases](https://github.com/RedBeardLab/rediSQL/releases).

To load RediSQL is sufficient to pass it as argument to the redis-server: `./redis-server --loadmodule /path/to/redisql.so`

The whole code show in this example is reachable [here.](https://github.com/RedBeardLab/rediSQL/blob/master/doc/docs/blog/node/hn.js)


## Dependencies

The first step is always to load the dependencies, for this little script we will need just 3 dependencies:

1. `util` for formatting strings and promisify functions
2. `axios` for making HTTP requests
3. `redis` for actually talking with RediSQL

```js
const util = require('util');
const axios = require('axios');
const redis = require('redis');
```

## Connect to Redis and make it asycn/await ready.

RediSQL works on a normal Redis instance, hence we need to create a connection to our Redis.

In this tutorial we are going to use the async/await syntax that is not natively supported by `node_redis` but that it can easily added using `promisify`.

We first create a new connection to Redis, just one that is enough, and then we promisify the `send_command` method in order to give us the nice async/await syntax.

```js
const client = redis.createClient();
const send_command = util.promisify(client.send_command).bind(client); 
```

## Polling HN API to get the newest items

The API of HN provides an endpoint to retrieve the ID of the latest element added to the website.
The ID are auto-incremental, hence if the ID = `n` exists, it means that also the ID = `n - 1` exists as well.

However is necessary a little of cautions, indeed, is possible that some items are not yet available if they exists, in such case they return the string `null` instead of a JSON object.
We take care of this with a simple loop and an `if`.

```js
const getMaxItem = async () => {
        let response = await axios.get("https://hacker-news.firebaseio.com/v0/maxitem.json");
        return response.data;
};

const getItem = async id => {
        let url = util.format("https://hacker-news.firebaseio.com/v0/item/%s.json", id);
        while (true) {
                let response = await axios.get(url);
                let data = response.data;
                if (data != "null") {
                        return data;
                }
        }
};
```

## The RediSQL structure

Now that we have removed all the boilerplate let's get down to business. We need to:

1. Create our database in RediSQL using the `REDISQL.CREATE_DB` command.
2. Create the table that will contains our data using the `REDISQL.EXEC` command.
3. Create the statement to actually insert the data into the database using the `REDISQL.CREATE_STATEMENT` command. 

We will store in our table the `id` of the item we are adding, the author of the item, when the item was posted on HN and finally the whole item as a JSON structure.

The third step is not strictly needed, but it provide defense against SQL-injections, is more performant, and it is just a cleaner way to do it. 
The alternative would be to just use `REDISQL.EXEC` every time we want to add a new row to the database.

All those steps are done in the `setUp` functions.


```js
const setUp = async () => {
        await send_command("REDISQL.CREATE_DB", ["HN"]).catch( err => console.log(err) );
        table = "CREATE TABLE IF NOT EXISTS hn(id integer primary key, author text, time int, item text);"
        await send_command("REDISQL.EXEC", ['HN', table]).catch( err => console.log(err) );
        stmt = "INSERT INTO hn VALUES(" + 
		"json_extract(json(?1),'$.id')," +
		"json_extract(json(?1),'$.by')," +
		"json_extract(json(?1),'$.time')," +
		"json(?1));";
        await send_command("REDISQL.CREATE_STATEMENT", ['HN', 'insert_item', stmt])
                .catch( err => console.log(err) );
};
```

Note how nicely the `JSON1` modules help us. 
It extracts for us the `id` of the item we want to add, the author of the item (the `by` field) and the time when the item was created.
Without it we would be force to do that operation ourselves and pass more parameters to the statement.

## Insert data into the database

Finally let's make a simple function that will gets an item from HN and stores it into our database executing the statement we just created.

```js
const storeItem = async id => {
        let item = await getItem(id);
        console.log(item);
        await send_command("REDISQL.EXEC_STATEMENT", ['HN', 'insert_item', JSON.stringify(item)])
                .catch( err => console.log(err) );
}
```

## Running it

Now that we have all our piece in order we can just combine them together.

We will start by creating the database, table and statements.

Then a simple infinite loop will simply keep pooling the API.
As soon as it detects new items available, it download them, and store those into the database.

We also include a small throttling mechanism to avoid hammering the API, which is not needed using the `sleep` function.

```js
const sleep = (waitTimeInMs) => new Promise(resolve => setTimeout(resolve, waitTimeInMs));

(async () => {
        setUp();
        let maxItem = await getMaxItem();
        while (true) {
                let newMaxItem = await getMaxItem();
                for (; maxItem < newMaxItem; maxItem += 1) {
                        storeItem(maxItem);
                }
                await sleep(5 * 1000);
        }
})()
```

## Concluding

In this small example (less than 60 lines) we show how simple and easy is to get started with RediSQL. We just set up the database and table once, along with the statement and that's it.
A much quicker set up, especially for testing, than using classical databases as `Postgres` or `MySQL`.

Indeed is sufficient to set up the database following always the same steps:

1. Create the database with `REDISQL.CREATE_DB $db_name`
2. Create the schema inside your database `REDISQL.EXEC $db_name "CREATE TABLE ... etc"` and, if you want, the statements: `REDISSQL.CREATE_STATEMENT $db_name $statement_name "INSERT INTO ... etc"`
3. Start inserting data, with or without a statement: `REDISQL.EXEC_STATEMENT` or simply `REDISQL.EXEC`

Finally to query back the data is possible to use:

- queries `REDISQL.QUERY $db_name "SELECT * FROM ..."`,
- statements `REDISQL.CREATE_STATEMENT $db_name $query_name "SELECT * FROM ... WHERE foo = ?1"` and the `REDISQL.QUERY_STATEMENTS $db_name $query_name "paramenters"`
- simple exec `REDISQL.EXEC $db_name "SELECT * FROM ... etc"`

Feel free to explore our [references documentation][ref] to understand better what capabilities RediSQL provides you.

The complete code of this example is [available here.](https://github.com/RedBeardLab/rediSQL/blob/master/doc/docs/blog/node/hn.js)

If you wish to see a similar tutorial for a different language, [open an issue on github.](https://github.com/RedBeardLab/rediSQL/issues/new)

[hn-api]: https://github.com/HackerNews/API
[json1]: https://www.sqlite.org/json1.html
[ref]: ../../references
