#include "sqlite3.h"
#include "redismodule.h"

extern RedisModuleType *RediSQL_SQLiteDB;

static RedisModuleType *RediSQL_Projection;

typedef struct {
  char *name;
  sqlite3 *connection;
} Phy_RediSQL_SQLiteDB;

