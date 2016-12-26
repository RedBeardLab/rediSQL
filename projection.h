
#include "redismodule.h"

int Project(RedisModuleCtx *ctx, RedisModuleString **argv, int argc);

int RemoveProjection(RedisModuleCtx*, RedisModuleString**, int);
int RediSQL_Set(RedisModuleCtx*, RedisModuleString**, int);
int RediSQL_Get(RedisModuleCtx*, RedisModuleString**, int);

void FreeProjection(void *Projection);

