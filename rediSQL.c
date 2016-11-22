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

#define rediSQL_ENCODING_VERSION 0

#define REPLY_ERROR_FMT(ctx, fmt, err)                                         \
  RedisModule_ReplyWithError(                                                  \
      ctx, RedisModule_StringPtrLen(                                           \
               RedisModule_CreateStringPrintf(ctx, fmt, err), NULL));

static RedisModuleType *rediSQL;

typedef sqlite3 redisSQL;

redisSQL *db;

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


int ExecCommand(RedisModuleCtx *ctx, RedisModuleString **argv, int argc){
  RedisModule_AutoMemory(ctx);
  if (argc != 2){
    return RedisModule_WrongArity(ctx);
  }

  sqlite3_stmt *stm;
  int result_code = 0;
  int num_results = 0;
  int num_of_columns = 0;
  int i = 0;

  size_t query_len;
  const char* query = RedisModule_StringPtrLen(argv[1], &query_len);
  
  result_code = sqlite3_prepare_v2(db, query, query_len, &stm, NULL);
  

  if (SQLITE_OK != result_code){
    RedisModuleString *e = RedisModule_CreateStringPrintf(ctx, 
		    "ERR - %s | Query: %s", sqlite3_errmsg(db), query);
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
		    "ERR - %s | Query: %s", sqlite3_errmsg(db), query);
    sqlite3_finalize(stm);
    return RedisModule_ReplyWithError(ctx, 
		    RedisModule_StringPtrLen(e, NULL));
  }
    
  sqlite3_finalize(stm);
  return REDISMODULE_OK;
}

int RedisModule_OnLoad(RedisModuleCtx *ctx) {
  if (RedisModule_Init(ctx, "rediSQL__", 1, REDISMODULE_APIVER_1) ==
      REDISMODULE_ERR) {
    return REDISMODULE_ERR;
  }

  int rc;
  
  rc = sqlite3_open(":memory:", &db);
  if (rc != SQLITE_OK)
    return REDISMODULE_ERR;

  if (RedisModule_CreateCommand(ctx, "rediSQL.exec", ExecCommand, 
	"deny-oom random no-cluster", 1, 1, 1) == REDISMODULE_ERR){
    return REDISMODULE_ERR;
  }

  return REDISMODULE_OK;
}

