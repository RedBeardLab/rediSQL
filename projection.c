#include <string.h>

#include "rediSQL.h"

#include "redismodule.h"
#include "projection.h"


static RedisModuleType *RediSQL_Projection;

typedef struct {
  int ID;
  char* field;
  char* value;
} Update;

typedef struct {
  Update* WAL;
} Projection;

void FreeProjection(void *Pj){
  Projection *PJ = (Projection*)Pj;
  
  RedisModule_Free(PJ->WAL);
}

int Project(RedisModuleCtx *ctx, RedisModuleString **argv, int argc){
  if (3 != argc){
    return RedisModule_WrongArity(ctx);
  }
  
  RedisModule *DBKey = RedisModule_OpenKey(ctx, argv[1], REDISMODULE_READ | REDISMODULE_WRITE);
  Phy_RediSQL_SQLiteDB *DB;
 
  if (REDISMODULE_KEYTYPE_MODULE != RedisModule_KeyType(DBKey) ||
      RediSQL_SQLiteDB != RedisModule_ModuleTypeGetType(DBKey)){
    RedisModule_CloseKey(DBKey);
    return RedisModule_ReplyWithError(ctx, REDISMODULE_ERRORMSG_WRONGTYPE);
  }

  DB = RedisModule_ModuleTypeGetValue(DBKey);

  RedisModule *PJKey = RedisModule_OpenKey(ctx, argv[2], REDISMODULE_READ | REDISMODULE_WRITE);

  if (REDISMODULE_KEYTYPE_EMPTY != RedisModule_KeyType(PJKey)){
    RedisModule_CloseKey(PJKey);
    return RedisModule_ReplyWithError(ctx, "KEY_USED They key you are trying to project is already bind.");
  }

  Projection* PJ = RedisModule_Alloc(sizeof(Projection));

  if (REDISMODULE_OK == RedisModule_ModuleTypeSetValue(
	PJKey, Projection, PJ)){
    RedisModule_CloseKey(PJKey);
    RedisModule_CloseKey(DBKey);
    return RedisModule_ReplyWithSImpleString(ctx, "OK");
  }

  return RedisModule_ReplyWithSimpleString(ctx, "Projection "); 
}

int RemoveProjection(RedisModuleCtx *ctx, RedisModuleString **argv, int argc){
  return RedisModule_ReplyWithSimpleString(ctx, "Remove Project"); 
}

/*
 * Arguments:
 * 1. The DB object
 * 2. The key to set, we must check that the table is the one projected
 * 3. The value to set
 *
 * */
int RediSQL_Set(RedisModuleCtx *ctx, RedisModuleString **argv, int argc){
  /* Check Arity */
  if (4 != argc){
    return RedisModule_WrongArity(ctx);
  }

  RedisModuleKey *DBKey = RedisModule_OpenKey(ctx, argv[1], REDISMODULE_READ | REDISMODULE_WRITE);
  Phy_RediSQL_SQLiteDB *DB;

  const char* key_immutable;
  char* key;

  char* table_name;
  char* id;
  char* field;


  if (REDISMODULE_KEYTYPE_MODULE != RedisModule_KeyType(DBKey) ||
      RediSQL_SQLiteDB != RedisModule_ModuleTypeGetType(DBKey)){
    RedisModule_CloseKey(DBKey);
    return RedisModule_ReplyWithError(ctx, REDISMODULE_ERRORMSG_WRONGTYPE);
  }

  DB = RedisModule_ModuleTypeGetValue(DBKey);
  RedisModule_Log(ctx, "notice", DB->name);

  key_immutable = RedisModule_StringPtrLen(argv[2], NULL);
  key = RedisModule_Alloc(sizeof(char) * strlen(key_immutable));
  strcpy(key, key_immutable);
  
  table_name = strtok(key, ":");
  id = strtok(NULL, ":");
  field = strtok(NULL, ":");
  
  RedisModuleString projection_name = RedisModule_CreateString(ctx, table_name, strlen(table_name))
  
  RedisModuleKey *PJKey = RedisModule_OpenKey(ctx, projection_name, REDISMODULE_READ | REDISMODULE_WRITE)

  return RedisModule_ReplyWithSimpleString(ctx, "SET Project"); 
}

int RediSQL_Get(RedisModuleCtx *ctx, RedisModuleString **argv, int argc){
  return RedisModule_ReplyWithSimpleString(ctx, "GET Project"); 
}

