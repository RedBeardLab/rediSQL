# RediSQL for analytics

RediSQL is a module for Redis that embed a completely functional SQLite database. 

RediSQL enables new paradigm where is possible to have several smaller decentralized databases instead of a single giant one.

In this blog post, we are going to explore how RediSQL can be used for storing analytics data.

Redis is always been used for storing fast data and so it is an extremely interesting software for analytics solution.

We are now going to describe the problem, explore some data structures that may help and finally sketch a possible solution using RediSQL.

At the end of the article, there is actual python code that you can run.

## Problem

Suppose you are interested in following the user around your website, and you will like to know what buttons they click, what events they trigger, what form the focus on and so on and so forth.

All these events are quite simple to catch using javascript and client-side code, but then you need to store them in your database to analyze them further and extract new information and value for your business.

However, you would prefer to avoid to put too much pressure on your main database that is already busy storing all the essential information for the business.

## Data Structure

One of the advantages of using SQL is the possibility to use and declare the shape of your data.

For this specific problem, our data are quite simple.
We want to store a user identifier (it may be its alias, nickname, ID in the main database or even something else), the IP address of the user, the timestamp when the event was triggered and finally the event itself.

We are going to represent the identifier, the IP address and the timestamp as strings.
Yes, unfortunately, SQLite does not provide a time type, to use a string is quite a reasonable choice, another one could be to use integers and to save the timestamp as Unix epoch of the event.

### Events

Representing the events may be a little complex and it really depends on your use case.
Suppose you are just listening to specific events like "Sales", "Register", "Login" or "Submit form" you could simply store them as strings.

However you can be a little more sophisticated as well, and associate to every Sales some other data like "amount", "shipping cost" or "total elements sold" or again improve the "Submit form" with information about the web page, like the URL of the page or if it was the A or the B version of your A/B test. And so on and so forth.

### JSON or Tables

If your events are quite static and you already know what you are going to store the best approach is to use tables.

An idea could be to use this representation for the table `Events`:

``` SQL
| event_id | user_id | ip_address | timestamp |
|----------|---------|------------|-----------|
```

And then different tables for each type of events, like:

`Sales`:

``` SQL
| event_id | amount  | shipping_cost | total_elements_sold |
|----------|---------|---------------|---------------------|
```

`Submits`:

``` SQL
| event_id | url_page | A/B_version |
|----------|----------|-------------|
```

Where `event_id` is a Primary key to the table `Events` and a Foreign key on the table `Sales` and `Submits`.

This approach works really well, the shape of your data will be always known and it will be fast, however, you actually need to know what you are saving in your DB and change the structure of the table is quite complex.

A different approach will be to store directly JSON in your table.

The new schema will be only a single table, `Events`:

``` SQL
| event_id | user_id | ip_address | timestamp | data |
|----------|---------|------------|-----------|------|
```

The column data will be of type `text` and it will store anything you want if encoded in JSON.

Of course, it is also possible to run any kind of computation on the JSON data including filters and selection.

Using JSON you gain a lot of flexibility but you are not sure anymore of the shape of your data and if you are not careful it may cause some headaches.

## Solution Sketch

In this section we are going to get through a possible implementation of the above solution, we are going to use the JSON variant since I believe that not everybody knew that SQLite could handle JSON so well.

I am assuming you already know how to get a Redis instance and how to load a module into it, if not make sure to check out [the readme of the project][set_up]

We are going to automate as much as possible in this tutorial, in this way your analytic script will just run.

The very first thing to do is to get a working connection to your Redis instance, any Redis binding should make this process quite simple, here an example in python.

```python
import redis
r = redis.StrictRedis(host='localhost', port=6379, db=0)
```

Now that you have established a connection the next step is to create a RediSQL database, RediSQL can manage multiple, completely independent databases, each associated with a Redis key, for this simple example we are going to use only one database that, with a lot of fantasy, `DB`.

```python
ok = r.execute_command("REDISQL.CREATE_DB", "DB")
assert ok == "OK"
```

Now that we have created our database we can go ahead and create the table that will contain our data. We are going to create the table if and only if it does not exists yet.

```python
done = r.execute_command("REDISQL.EXEC", "DB", 
                         """CREATE TABLE
                            IF NOT EXISTS 
                            Events(
                                event_id INTEGER PRIMARY KEY,
                                user_id STRING,
                                ip_address STRING,
                                timestamp STRING,
                                data JSON
                            );""")
assert done == ["DONE", 0]
```

Setting the type of `event_id` as `INTEGER PRIMARY KEY` is synonymous with `ROWID` which is an autoincrement fields that do not need to be set during insert.

At this point, the only thing left to do is to listen for events in your code and write them into the database.

The simplest, and insecure, way to write the data is to use the `EXEC` function like so:

```python
# import datetime
user_id = "user_1234"
ip_address = "a.simple.ip.address"
now = datetime.datetime.now()
data = {"type" : "sales", "total": 1999, "shipping_address" : "..."}
statement = """INSERT INTO Events (user_id, ip_address, timestamp, data) 
               VALUES(\"{}\", \"{}\", \"{}\", \"{}\")""" \
               .format(user_id, ip_address, now, data)
done = r.execute_command("REDISQL.EXEC", "DB", statement)
assert done == ["DONE", 1]
```

As you may have guessed already the return value of `REDISQL.EXEC` is a list of two elements, the string `DONE` and the integer representing the number of rows modified (inserted, deleted or updated).

However, this way of inserting data into the database is not optimal, especially if the same operation will be performed several times. And also because it is vulnerable to SQL injections attacks.

The better and safer way to do this kind of operation is to define **statements**.

```python
ok = r.execute_command("REDISQL.CREATE_STATEMENT", "DB", "insert_event",
                       """INSERT INTO Events 
                          (user_id, ip_address, timestamp, data) 
                          VALUES(?1, ?2, ?3, ?4)""")
assert ok == "OK"
```

Once a statement is defined you can execute it using the following commands.

```python
# import datetime
user_id = "user_1234"
ip_address = "a.simple.ip.address"
now = datetime.datetime.now()
data = {"type" : "sales", "total": 1999, "shipping_address" : "..."}
done = r.execute_command("REDISQL.EXEC_STATEMENT", "DB", "insert_event", user_id, ip_address, now, data)
assert done == ["DONE", 1]
```

The use of statements brings some benefits.

* It reduces code deduplication in your code base
* It puts a name on a particular procedure, decoupling the implementation and the goal
* It allows different microservices to invoke the always the exact same procedure
* It is faster to execute

### Use the JSON1 SQLite module

Now that we have covered how to execute SQL against RediSQL let me quickly introduce you to the JSON1 syntax provide by SQLite.

The ones that follow are plain SQL statements that you can execute `REDISQL.EXEC` against the database or that you can embed into a statement.

The most interesting function provide is `json_extract`.

``` SQL
SELECT user_id, json_extract(data, '$.total')
FROM Events
WHERE json_extract(data, '$.type') = "sales";
```

This query will look inside the field `type` of the JSON stored into the columns `data` if this fields contains the string "sales" it will return the user who bought something and total of the sale. 

`json_extract` works also on array using a simple syntax: `$.array[2]` (eg. extract the third element of the array)

## Move the data

Running the above script will be extremely fast, I am talking about 10ks inserts per second fast.

However, it is so fast for a variety of reason but maybe the most important is that it keeps all the data in memory and does not write them on disk.

This can be just fine for some application (think about storing data that become useless in few days time) or it can be a big issue for some other use case, luckily there is a very simple solution.

The simplest thing to do when you decide to dump the data in your persistent storage is just to query them all and push them, in batch, to your persistent system. Moving the data in all together will allow having an extremely high throughput and it will take a fraction of the time than if you moved just a row at the time.

A quite simple practice is to simply dump all the content of your database in a CSV file and then let your RDBMS load it.
 
This operation is quite simple and it can be done like so.

```python
# get the data
values = r.execute_command("REDISQL.EXEC", "DB", "SELECT * FROM Events;")
# iterate throught the list writing on file
with open('csv_file', 'w') as csv_file:
    # write the csv header
    csv_file.write("event_id,user_id,ip_address,timestamp,data\n")
    for row in values:
        # create a single string with all the fields separated by a comma
        elements = ",".join(row) + "\n"
        # write the result on the csv_file
        csv_file.write(elements)
```

Now you can use tools like PostgreSQL COPY to load all the data into your database.

This solution is not perfect and in a distributed setting with several concurrent workers it may result in some data duplication, however, we are getting ahead of ourselves and this topic will go far ahead of the scope of this post.

## Recap

In this blog post, we explored how to write a quite sophisticated analytics infrastructure using nothing more than RediSQL.

Adding this tool to your existing infrastructure should be quite simple and painless while it provides a simple way to do powerful things in a fast and reliable way.

Is worth to remember that RediSQL already provide RDB persistency so you already have some interesting level of safeness embed into this architecture.


[redis_download]: https://redis.io/download
[redisql_download]: https://github.com/RedBeardLab/rediSQL/releases
[set_up]: https://github.com/RedBeardLab/rediSQL#getting-start
