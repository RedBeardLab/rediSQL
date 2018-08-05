# Release 0.5.0 of RediSQL, SQL steroids for Redis

#### RediSQL, Redis on SQL steroids.

RediSQL is a Redis module that provides full SQL capabilities to Redis, it is the simplest and fastest way to get an SQL database up and running, without incurring in difficult operational issues and it can scale quite well with your business.

The fastest introduction to RediSQL is [our homepage](https://redisql.com)

**tl;dr;** This release does not introduce any new features but it does improve the performances significantly. 
Moreover, we are releasing for multiple platforms, notably for ARMv7 (Rasberry PI), for CentOS7 (Linux AMI on AWS) and of course for Linux generic. 
Finally, we introduce also the TRIAL executable which can be freely downloaded and used, it provides all the functionality of the PRO version but it is limited for evaluation purposes, after ~2 hours it will shut down itself.

## Performance Improvement ~20%

We are registering an improvement in performance of roughly 20% with a similar load.

Of course, performance inside an SQL database varies a lot depending on the query you are executing.

In our evaluation, we are focusing on a simple query that inserts or updates values in the database.

We decided to focus on `insert`s because is the simplest write operation, hence it cannot be distributed to different instances, and the performance of the single instance will limit the performance of the overall system.

However, `insert` operations, need to allocate new memory, the allocator used by SQLite is very efficient but we still wanted to see what would happen without the need of allocation, hence we tested also `update`s

We are seeing an improvement of roughly the 20% in `insert`/`upsert` throughput.

The increase in throughput is driven by the switch to a zero-copy architecture.

In Rust, the language in which RediSQL is written is usual to start dealing with lifetime issues simply by copying memory, this, of course, comes with a penalty in performances.

However, as long as this performance penalty is not an issue is better to just leave as it is and work on the more important parts of the project.
For one of our user it was a problem and so we decide to fix it by bringing performance improvements to all. [More about this story here.][performance]

## Releases

Rust produce in general static linked objects, so everything you need is already inside your object and you do not depends on any external library that must be installed in your host system.

This has several advantages, as long as the architecture of your host is the correct one your executable will most likely run.

There is an exception to that, `libc` given its ubiquity and size is compiled dynamically, so your object will need it to be present in your host machine. Which usually is not an issue.

Unfortunately is some machine `libc` is present in an older version that the one we are expecting, so the module will not be able to run.

Very old systems have this issues as well as CentOS 7 and the Linux AMI on AWS.

Unfortunately, cross-compile for a different version of glibc is not as simple as it may seem, but we finally manage :)

## Trial

Finally, we decide to provide open access to the PRO version for evaluation purposes.

Hence we created a third release that is called TRIAL.

The releases are exactly the same as the PRO one, except that it shuts itself down after ~2 hours.

It is supposed to let you test the PRO version before to commit to buy it, still, you have 14 days of money back guarantee if you don't like the product.

Clearly, the TRIAL version is not supposed to be used in production.

## End

As always you can find all the public releases on the [github page][releases], you can openly access the same public release on the [open page of our shop][plaso_open] or you can buy the complete PRO package [signing up in the shop][plaso_signup].

Remember that signing up for the PRO product also provide you free support from us, the creator of the project, so that we can point you to the right direction and suggest the best use cases for our product.

[releases]: https://github.com/RedBeardLab/rediSQL/releases/tag/v0.5.0
[plaso_open]: https://plasso.com/s/epp4GbsJdp-redisql/
[plaso_signup]: https://plasso.com/s/epp4GbsJdp-redisql/signup/
[performance]: blog/performances.md
