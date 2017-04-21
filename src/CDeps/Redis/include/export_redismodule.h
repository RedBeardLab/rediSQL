#include "redismodule.h"

extern RedisModuleType *DBType;

// RedisModule_Init is defined as a static function and so won't be exported as
// a symbol. Export a version under a slightly different name so that we can
// get access to it from Rust.
int Export_RedisModule_Init(RedisModuleCtx *ctx, const char *name, int ver, int apiver);

RedisModuleKey* Export_RedisModule_OpenKey(RedisModuleCtx *ctx, RedisModuleString *keyname, int mode);

