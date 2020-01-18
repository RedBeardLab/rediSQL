use redisql_lib::redis as r;
use redisql_lib::redis::RedisKey;
use redisql_lib::redis_type::Context;

use redisql_lib::redisql_error::RediSQLError;

pub trait CommandV2<'s> {
    fn parse(args: Vec<&'s str>) -> Result<Self, RediSQLError>
    where
        Self: std::marker::Sized;
    fn database(&self) -> &str;
    fn key(&self, ctx: &Context) -> RedisKey {
        let key_name = self.database();
        let key_name = r::rm::RMString::new(ctx, key_name);
        let key = r::rm::OpenKey(
            ctx,
            &key_name,
            r::rm::ffi::REDISMODULE_WRITE,
        );
        RedisKey { key }
    }
}
