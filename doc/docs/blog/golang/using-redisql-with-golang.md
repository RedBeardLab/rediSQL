
# Using RediSQL with Go(lang)

This tutorial will help you to get start to use RediSQL with Go(lang).

In this tutorial we will scrape the content of Hacker News using their [API documented here][hn-api].

To communicate with Redis we will use the popular [radix](https://github.com/mediocregopher/radix) library wich is the most suitable to communicate with Redis Modules and RediSQL. Other libraries can be used as well, but some more work will be necessary.

To follow this tutorial you will need a modern (> v4.0) instance of Redis running RediSQL.
You can obtain RediSQL from [our shop](https://payhip.com/b/Ri4d) or from the [github releases](https://github.com/RedBeardLab/rediSQL/releases).

To load RediSQL is sufficient to pass it as argument to the redis-server: `./redis-server --loadmodule /path/to/redisql.so`

The whole code show in this example is reachable [here](https://github.com/RedBeardLab/rediSQL/blob/master/doc/docs/blog/golang/main.go)

## Creating a Redis Pooled Connection

Using `radix` is quite simple to use a pooled connection to Redis, indeed `radix` provide the `NewPool` function that creates a pool of connection to Redis. Then it is possible to use that pool as a client and have the library allocate a client for us.

```golang
redis, err := radix.NewPool("tcp", "localhost:6379", 10)
if err != nil {
	fmt.Println(err)
        return
}
```

## Setting up the database

In order to work with RediSQL is necessary to do a small setup. The first step is always to create a database, then we create the tables inside the databases and finally the different statements if they are necessary.

It is always a good idea to use statements instead of building query by hand. but it is not mandatory. The use of statements eliminate the risk of SQL injection and it is more performant, since the query is parsed only once and not every time it get executed.

In our case we create a simple database, `HN`.

```golang
r.Do(radix.Cmd(nil, "REDISQL.CREATE_DB", "HN"))
```

Then we create a single table `hn` inside the `HN` database.

```golang
table := "CREATE TABLE IF NOT EXISTS hn(id integer primary key, author text, time int, item text);"
r.Do(radix.Cmd(nil, "REDISQL.EXEC", "HN", table))
```

The table will contains the `id` of each item we are getting from HN, along with the author of the item, when the item was posted and finally the last field will contains the whole item as a `json` string.

The last step is to create a statement to easily insert the data inside our table.

```golang
stmt := `INSERT INTO hn VALUES(
	json_extract(json(?1),'$.id'), 
	json_extract(json(?1),'$.by'), 
	json_extract(json(?1),'$.time'), 
	json(?1));`
r.Do(radix.Cmd(nil, "REDISQL.CREATE_STATEMENT", "HN", "insert_item", stmt))
```

The statement is a little complex. It exploit the [JSON1][json1] sqlite extension to extract the necessary fields from a JSON string. In particular we extract the `id`, the `by` (author) and the `time` fields.

After that those fields are extracted from the JSON string we store all of them into the database along with the whole item.


## The loop

After the database is ready to accept data, we start to poll the HN API in order to fetch the newest item. 
Each item is then pushed into the `itemIds` channel that is later consumed.

```golang
itemIds := make(chan int, 10)

go func() {
	oldMaxItemId := getMaxItem()
	for {
		newMaxItemId := getMaxItem()
		for ; oldMaxItemId < newMaxItemId; oldMaxItemId++ {
			itemIds <- oldMaxItemId
		}
		time.Sleep(5 * time.Second)
	}
}()
```

The `getMaxItem()` function is implemented as:

```golang
func getMaxItem() int {
	resp, _ := http.Get("https://hacker-news.firebaseio.com/v0/maxitem.json")
	defer resp.Body.Close()
	body, _ := ioutil.ReadAll(resp.Body)
	n, _ := strconv.Atoi(string(body))
	return n
}
```

Finally the main loop iterate through the `itemIds` channel.
For each new item we use again the HN API to get the content of the items and then we store it into RediSQL.



```golang
for itemId := range itemIds {
	go func() {
		item := getItem(itemId)
		err := redis.Do(radix.Cmd(nil, "REDISQL.EXEC_STATEMENT", "HN", "insert_item", item))
		if err != nil {
			fmt.Println(err)
		}
	}()
}
```

The `getitem()` functions implement a trivial error recovery strategy. 
Indeed, after some trial and error, was clear that, sometimes, the element `n` is not ready yet even if the element `n+1` was published as `maxitem` and the API returns the simple string "null", if that is the case we simply repeat the call.

```golang
func getItem(id int) string {
	for {
		url := fmt.Sprintf("https://hacker-news.firebaseio.com/v0/item/%d.json", id)
		resp, _ := http.Get(url)
		defer resp.Body.Close()
		body, _ := ioutil.ReadAll(resp.Body)
		result := string(body)
		if result != "null" {
			return result
		}
	}
}

```

## Concluding

This small example (less than 80 lines) show how simple is to quickly get value from RediSQL.
Indeed is sufficient to set up the database following always the same steps:

1. Create the database with `REDISQL.CREATE_DB $db_name`
2. Create the schema inside your database `REDISQL.EXEC $db_name "CREATE TABLE ... etc"` and, if you want, the statements: `REDISSQL.CREATE_STATEMENT $db_name $statement_name "INSERT INTO ... etc"`
3. Start inserting data, with or without a statement: `REDISQL.EXEC_STATEMENT` or simply `REDISQL.EXEC`

Finally to query back the data is possible to use:

- queries `REDISQL.QUERY $db_name "SELECT * FROM ..."`,
- statements `REDISQL.CREATE_STATEMENT $db_name $query_name "SELECT * FROM ... WHERE foo = ?1"` and the `REDISQL.QUERY_STATEMENTS $db_name $query_name "paramenters"`
- simple exec `REDISQL.EXEC $db_name "SELECT * FROM ... etc"`

Feel free to explore our [references documentation][ref] to understand better what capabilities RediSQL provides you.

The complete code of this example is [available here.](https://github.com/RedBeardLab/rediSQL/blob/master/doc/docs/blog/golang/main.go)



[hn-api]: https://github.com/HackerNews/API
[json1]: https://www.sqlite.org/json1.html
[ref]: ../../../references
