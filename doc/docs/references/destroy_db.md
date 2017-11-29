# DEL 

This command is a generic command from Redis.

It eliminates keys from Redis itself, as well if the key is a RediSQL database create with REDISQL.CREATE_DB it will eliminate the SQLite database, stop the thread loop and clean up everything left.

If the database is backed by a file the file will be close.

Internally it call `sqlite3_close`.

