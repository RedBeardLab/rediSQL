/*
 *
 * <RediSQL, SQL capabilities to redis.>
 * Copyright (C) 2016  Simone Mosciatti
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 * 
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * */



#include <stdio.h>
#include <string.h>

#include "sqlite3.h"
#include "redismodule.h"
#include "rmutil/util.h"
#include "rmutil/strings.h"
#include "rmutil/test_util.h"

#define PERSISTENTSQLITEDB_ENCODING_VERSION 1

static RedisModuleType *PersistentSQLiteDB;

typedef struct {
  char *name;
  sqlite3 *connection;
} PhyPersistentSQLiteDB;

sqlite3 *db;

int RediSQL_ExecOnConnection(RedisModuleCtx *ctx, sqlite3 *connection, const char *query, size_t query_len);

typedef void (*ReadAndReturn_type)(RedisModuleCtx *, sqlite3_stmt*, int);

void ReadAndReturn_Integer(RedisModuleCtx *ctx, sqlite3_stmt *stmt, int i){
  int result = sqlite3_column_int(stmt, i);
  RedisModule_ReplyWithLongLong(ctx, result);
}

void ReadAndReturn_Float(RedisModuleCtx *ctx, sqlite3_stmt *stmt, int i){
  double result = sqlite3_column_double(stmt, i);
  RedisModule_ReplyWithDouble(ctx, result);
}

void ReadAndReturn_Blob(RedisModuleCtx *ctx, sqlite3_stmt *stmt, int i){
  const char* result = (char*)sqlite3_column_blob(stmt, i);
  RedisModule_ReplyWithStringBuffer(ctx, result, strlen(result));
}

void ReadAndReturn_Null(RedisModuleCtx *ctx, sqlite3_stmt *stmt, int i){
  RedisModule_ReplyWithNull(ctx); 
}

void ReadAndReturn_Text(RedisModuleCtx *ctx, sqlite3_stmt *stmt, int i){
  const char *result = (char*)sqlite3_column_text(stmt, i);
  RedisModule_ReplyWithStringBuffer(ctx, result, strlen(result));
}




int createDB(RedisModuleCtx *ctx, RedisModuleString *key_name, const char *path){
  int rc;
  RedisModuleKey *key = RedisModule_OpenKey(ctx, key_name, REDISMODULE_WRITE);

  if (REDISMODULE_KEYTYPE_EMPTY != RedisModule_KeyType(key)){
    RedisModule_CloseKey(key);
    return RedisModule_ReplyWithError(ctx, "KEY_USED The key used is already bind");
  }
  
  PhyPersistentSQLiteDB *PhySQLiteDB = RedisModule_Alloc(sizeof(PhyPersistentSQLiteDB));
  PhySQLiteDB->name = RedisModule_Alloc(sizeof(char) * 100);
  if (NULL == path){
    PhySQLiteDB->name = NULL;
    rc = sqlite3_open(":memory:", &PhySQLiteDB->connection);
  } else {
    strcpy(PhySQLiteDB->name, path);
    rc = sqlite3_open(path,       &PhySQLiteDB->connection);
  }

  if (SQLITE_OK != rc){
    RedisModule_CloseKey(key);
    return RedisModule_ReplyWithError(ctx, "ERR - Problem opening the database");
  }
  
  RedisModule_CloseKey(key);
  if (REDISMODULE_OK == RedisModule_ModuleTypeSetValue(
	key, PersistentSQLiteDB, PhySQLiteDB)){
    return RedisModule_ReplyWithSimpleString(ctx, "OK");
  } else {
    return RedisModule_ReplyWithError(ctx, "ERR - Impossible to set the key");
  }
}

int CreateDB(RedisModuleCtx *ctx, RedisModuleString **argv, int argc){
  const char *path;
  if (2 == argc){
    return createDB(ctx, argv[1], NULL);
  }
  if (3 == argc){
    path = RedisModule_StringPtrLen(argv[2], NULL); 
    return createDB(ctx, argv[1], path);
  }
  return RedisModule_WrongArity(ctx);
}

int ExecCommand(RedisModuleCtx *ctx, RedisModuleString **argv, int argc){
  RedisModule_AutoMemory(ctx);
  sqlite3 *connection;
  size_t query_len;
  const char* query;
  RedisModuleKey *key;
  int key_type;
  PhyPersistentSQLiteDB *key_value;
  
  if (argc == 3){
    key = RedisModule_OpenKey(ctx, argv[1], REDISMODULE_READ | REDISMODULE_WRITE);
    key_type = RedisModule_KeyType(key);  
    
    if (REDISMODULE_KEYTYPE_EMPTY          == key_type ||
	RedisModule_ModuleTypeGetType(key) != PersistentSQLiteDB ){
      
      return RedisModule_ReplyWithError(ctx, REDISMODULE_ERRORMSG_WRONGTYPE);
    }

    key_value = RedisModule_ModuleTypeGetValue(key);
    
    connection = key_value->connection;
    query = RedisModule_StringPtrLen(argv[2], &query_len); 
    
    return RediSQL_ExecOnConnection(ctx, connection, query, query_len);
  }
  if (argc == 2){
    connection = db;
    query = RedisModule_StringPtrLen(argv[1], &query_len);

    return RediSQL_ExecOnConnection(ctx, connection, query, query_len);
  }
  else {
    return RedisModule_WrongArity(ctx);
  }
}

int RediSQL_ExecOnConnection(RedisModuleCtx *ctx, sqlite3 *connection, const char *query, size_t query_len){
  
  sqlite3_stmt *stm;
  int result_code = 0;
  int num_results = 0;
  int num_of_columns = 0;
  int i = 0;

  result_code = sqlite3_prepare_v2(connection, query, query_len, &stm, NULL);
  

  if (SQLITE_OK != result_code){
    RedisModuleString *e = RedisModule_CreateStringPrintf(ctx, 
		    "ERR - %s | Query: %s", sqlite3_errmsg(connection), query);
    sqlite3_finalize(stm);
    return RedisModule_ReplyWithError(ctx, 
		    RedisModule_StringPtrLen(e, NULL));
  }

  result_code = sqlite3_step(stm);
    
  if (SQLITE_OK == result_code){
    RedisModule_ReplyWithSimpleString(ctx, "OK");  
    sqlite3_finalize(stm);
    return REDISMODULE_OK;
  }
  else if (SQLITE_ROW == result_code) {
    RedisModule_ReplyWithArray(ctx, REDISMODULE_POSTPONED_ARRAY_LEN);
    num_of_columns = sqlite3_column_count(stm);

    ReadAndReturn_type* ReadAndReturn_Functions = RedisModule_Alloc(sizeof(ReadAndReturn_type) * num_of_columns);
      
    for(i = 0; i < num_of_columns; i++){
      switch(sqlite3_column_type(stm, i)){
	case SQLITE_INTEGER: 
	  ReadAndReturn_Functions[i] = ReadAndReturn_Integer;
	break;
	  
	case SQLITE_FLOAT:
	  ReadAndReturn_Functions[i] = ReadAndReturn_Float;
	break;
	  
	case SQLITE_BLOB:
	  ReadAndReturn_Functions[i] = ReadAndReturn_Blob;
	break;
	  
	case SQLITE_NULL:
	  ReadAndReturn_Functions[i] = ReadAndReturn_Null;
	break;
	  	  
	case SQLITE_TEXT:
	  ReadAndReturn_Functions[i] = ReadAndReturn_Text;
	break;
      }
    }

    while(SQLITE_ROW == result_code) {
      num_results++;

      RedisModule_ReplyWithArray(ctx, num_of_columns);
      for(i = 0; i < num_of_columns; i++){
	ReadAndReturn_Functions[i](ctx, stm, i);
      }

      result_code = sqlite3_step(stm);
    }
    RedisModule_ReplySetArrayLength(ctx, num_results);
    RedisModule_Free(ReadAndReturn_Functions);
    sqlite3_finalize(stm);
    return REDISMODULE_OK;
  }
  else if (SQLITE_DONE == result_code) {
      sqlite3_finalize(stm);
      return RedisModule_ReplyWithSimpleString(ctx, "OK");
  }
  else {
    RedisModuleString *e = RedisModule_CreateStringPrintf(ctx, 
		    "ERR - %s | Query: %s", sqlite3_errmsg(connection), query);
    sqlite3_finalize(stm);
    return RedisModule_ReplyWithError(ctx, 
		    RedisModule_StringPtrLen(e, NULL));
  }
    
  sqlite3_finalize(stm);
  return REDISMODULE_OK;
}

int SQLiteVersion(RedisModuleCtx *ctx, RedisModuleString **argv, int argc){
  if (argc != 1)
    return RedisModule_WrongArity(ctx);

  return RedisModule_ReplyWithSimpleString(ctx, sqlite3_version);
}

int RedisModule_OnLoad(RedisModuleCtx *ctx, RedisModuleString **argv, int argc) {
  if (RedisModule_Init(ctx, "rediSQL__", 1, REDISMODULE_APIVER_1) ==
      REDISMODULE_ERR) {
    return REDISMODULE_ERR;
  }

  RedisModuleTypeMethods tm = {
	.version = PERSISTENTSQLITEDB_ENCODING_VERSION,
	.rdb_load = NULL,
	.rdb_save = NULL,
	.aof_rewrite = NULL,
	.free = NULL};

  PersistentSQLiteDB = RedisModule_CreateDataType(
	ctx, "Per_DB_Co", 
	PERSISTENTSQLITEDB_ENCODING_VERSION, &tm);

  if (PersistentSQLiteDB == NULL) return REDISMODULE_ERR;

  int rc;
  const char* database_name;

  if (1 == argc){
    database_name = RedisModule_StringPtrLen(argv[0], NULL);

    rc = sqlite3_open(database_name, &db);
  } else {
    rc = sqlite3_open(":memory:", &db);
  }

  if (rc != SQLITE_OK)
    return REDISMODULE_ERR;

  if (RedisModule_CreateCommand(ctx, "rediSQL.EXEC", ExecCommand, 
	"deny-oom random no-cluster", 1, 1, 1) == REDISMODULE_ERR){
    return REDISMODULE_ERR;
  }

  if (RedisModule_CreateCommand(ctx, "rediSQL.SQLITE_VERSION", SQLiteVersion, "readonly", 1, 1, 1) == REDISMODULE_ERR){
    return REDISMODULE_ERR;
  }

  if (RedisModule_CreateCommand(ctx, "rediSQL.CREATE_DB", CreateDB, "write deny-oom no-cluster", 1, 1, 1) == REDISMODULE_ERR){
    return REDISMODULE_ERR;
  }

  return REDISMODULE_OK;
}

