# RediSQL PRO

This document explains the architecture and principle of working of RediSQL PRO.

You can purchase RediSQL PRO, along with support [**here.**][signup]

## Main difference

The PRO version comes without telemetrics.

The community version requires RediSQL to periodically send telemetrics information to our server otherwise RediSQL shut itself down.
This is not require by the PRO version.

## Telemetrics data

The data send to our servers are definitely not critical data, indeed we ship the result of the `REDISQL.STATISTICS` commands.
Moreover we took all the precaution to make sure that our user are not impacted by possible errors on our side replicating all pieces of our infrastructure. [More details about the technical implementation on our blog.][redisql-telemetrics]

[More details behind our motivation in our blog post.][release-1.0.0]

[release-1.0.0]: http://redbeardlab.com/2019/04/09/redisql-release-1-0-0-change-in-strategy-and-business-model/
[redisql-telemetrics]: http://redbeardlab.com/2019/04/09/telemetrics-for-redisql/
[query]: references.md#redisqlquery
[query_statement]: references.md#redisqlquery_statement
[pro_motivations]: pro_motivations.md
[redis_persistence]: https://redis.io/topics/persistence
[signup]: https://plasso.com/s/epp4GbsJdp-redisql/signup/
[redis_cluster]: https://redis.io/topics/cluster-tutorial
[redis_replication]: https://redis.io/topics/replication
