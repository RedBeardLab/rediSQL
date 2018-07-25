# Doubling the performances of RediSQL, SQL steroids for Redis.

#### RediSQL provides SQL steroids for Redis

RediSQL is a redis module that embeds SQLite to provide full SQL capabilities to redis.

The fastest introduction to RediSQL is [our homepage](/)

tl;dr; We double the performance of RediSQL switching to a zero-copy use of the input arguments.

## Background

During the development of RediSQL we always kept in mind performance, but only up to a degree.

As Donal Knuth says "premature optimization is the root of all evil", but he also added, "Yet we should not pass up our opportunities in that critical 3%."

For us not pass up opportunities was more about using the correct data structure and algorithm when and where they make sense.

Overall the performances were quite good, on my old personal machine I could get 30k/inserts per second in a very simple table (few numeric value and small texts).

This number is really small compared to Redis that can `SET` keys up to 100k/set per second, but we are also doing more work and we have always considered this figures good enough, at least as long as nobody complains.

## The complains

*Fortunately* somebody [complains about the insertion rates][github_issues].

His requirements were a little different from the assumption that we had when testing the performance. He needed to insert a lot of data ~100kB in each row. And to do it fast.

First tests were not so good, showing just 2000 insertions per second. Definitely not good.

## Looking for the causes

Rust, the system language in which RediSQL is built, is peculiar in the management of memory.

The type system must be sure that every reference that you are using is valid and that you will never deference stuff out of your memory space.

Of course, this gives you a lot of safety but introduces quite a bit of complexity.

Fortunately, there are ways to manage this complexity: is possible to trade complexity for performances or, the other way around, performances for complexity.

High complexity with high performance means that you are using references (pointers) everywhere it is possible, so you never copy memory around if it is not necessary, you pass a pointer to that memory location and the type system assure you that it is safe to deference such pointer.

Low complexity with low performance implies to copy areas of memory multiple times so that the type system is always happy. Instead of using a pointer to a piece of memory I will just copy everything I need and pass it around functions.

Since the product was new, it made sense for us to get as low complexity as possible in such a way that it is possible to move faster understanding what our clients need and what we want to build.

Until now.

## The problem

The biggest problem we identified was that we were copying all the input parameter, so when a client was sending as a request like `REDISQL.EXEC_STATEMENT DB insert 1 2 3` we were copying 6 (insert) + 3 (1, 2, 3) = 9 bytes of memory.

(Redis string are a little particular since they don't have a null '\0' terminator and carry along their size.)

Nobody will complain about copying just ~10 bytes of memory, especially using slab allocator it is still some work but it is quite fast to do.

However, when we start to ingest 100k bytes this will really impact negatively the performance.

## Need for Speed

We could get away with just copying and freeing few bytes, especially using jemalloc, however as soon as the payload start increasing in size copying it every time started to impact negatively the performance of the module.

Was time to buy some performances paying our share price in complexity.

The process was a little complex because the data lived outside the Rust code, inside Redis itself.
It was necessary to use `unsafe` code that in normal Rust you can avoid, but finally, we were able to see the Redis string as a slice (array) of chars and get a reference to it to pass around.

## Result

Before to release the code I obviously tested it. I used a c4.8xlarge from AWS.

That machine before the patch could ingest 4000 request / second each of ~100k bytes. After the patch, it was able to get 8000 requests/second.

Finally, the latency was pretty much the same with the 99.9 percentile at ~50 milliseconds.

Xin, the gentleman who brought the issues to our attention, went even further setting several parameters available in `redis-benchmark`, notably the number of pipelined requests, and he reached 17000 requests per second.

Being conservative we can claim that our works made redisql twice (from 4000 req/s to 8000 req/s) as fast at ingesting big amount of data.

It is definitely necessary more testing to see the impact on different workloads, smaller payload. Fortunately, our preliminary results seem quite good.

## Wrapping up

We showed how was possible to get a lot of performance out of Rust and RediSQL switching to a zero copy approach.

We also showed that our solution is very efficient, being able to ingest 800 MB / s of data without any sort of tuning.

## Looking for beta tester

We are looking also for tester for the PRO version of RediSQL, if you would like to test the PRO module without paying anything this is your occasion.
You can either open an issues on github or write me (siscia) directly (email on github).

[github_issues]: https://github.com/RedBeardLab/rediSQL/issues/29#issuecomment-382199622
