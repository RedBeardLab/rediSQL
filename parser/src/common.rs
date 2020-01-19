use redisql_lib::redis::RedisKey;
use redisql_lib::redis_type::Context;

use redisql_lib::redisql_error::RediSQLError;

pub trait CommandV2<'s> {
    fn parse(args: Vec<&'s str>) -> Result<Self, RediSQLError>
    where
        Self: std::marker::Sized;
    fn database(&self) -> &str;
    fn key(&self, ctx: &Context) -> RedisKey {
        RedisKey::new(self.database(), ctx)
    }
}
