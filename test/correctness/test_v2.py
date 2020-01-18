#!/usr/bin/python -tt
# -*- coding: utf-8 -*-

import unittest
import os
import tempfile
import shutil
import time

import redis
from rmtest import ModuleTestCase

if "REDIS_MODULE_PATH" not in os.environ:
    os.environ["REDIS_MODULE_PATH"] = "../../target/release/libredis_sql.so"

os.environ["RUST_BACKTRACE"] = "full"

class Table():
  def __init__(self, redis, name, values, key = ""):
    self.redis = redis
    self.key = key
    self.name = name
    self.values = values

  def __enter__(self):
    create_table = "CREATE TABLE {} {}".format(self.name, self.values)
    if self.key:
      self.redis.client.execute_command("REDISQL.V1.EXEC", self.key, create_table) 
    else:
      self.redis.client.execute_command("REDISQL.V1.EXEC", create_table)

  def __exit__(self, type, value, traceback):
    drop_table = "DROP TABLE {}".format(self.name)
    if self.key:
      self.redis.client.execute_command("REDISQL.V1.EXEC", self.key, drop_table)
    else:
      self.redis.client.execute_command("REDISQL.V1.EXEC", drop_table)

class DB():
  def __init__(self, redis, key):
    self.redis = redis
    self.key = key

  def __enter__(self):
    self.redis.client.execute_command("REDISQL.V2.CREATE_DB", self.key)

  def __exit__(self, type, value, traceback):
    self.redis.client.execute_command("DEL", self.key)


class TestRediSQLWithExec(ModuleTestCase('')):
  def setUp(self):
    self.disposable_redis = self.redis()

  def tearDown(self):
    pass

  def exec_naked(self, *command):
    return self.client.execute_command(*command)

  def exec_cmd(self, *command):
    return self.client.execute_command("REDISQL.V1.EXEC", *command)

  def create_db(self, key):
    return self.client.execute_command("REDISQL.V2.CREATE_DB", key)

  def delete_db(self, key):
    return self.client.execute_command("DEL", key)

  def get_redis_server_major_version(self):
    result = self.exec_naked("INFO", "SERVER")
    major_version = result['redis_version'].split(':')[0].split('.')[0]
    return int(major_version)


class TestRediSQLCreateDB(TestRediSQLWithExec):
  def test_create_db(self):
    ok = self.exec_naked("REDISQL.V2.CREATE_DB", "DB1")
    self.assertEqual(ok, b'OK')

  def test_can_exists_flag(self):
    ok = self.exec_naked("REDISQL.V2.CREATE_DB", "DB2")
    self.assertEqual(ok, b'OK')
    ok = self.exec_naked("REDISQL.V2.CREATE_DB", "DB2", "CAN_EXISTS")
    self.assertEqual(ok, b'OK')

  def test_can_exists_default(self):
    ok = self.exec_naked("REDISQL.V2.CREATE_DB", "DB3")
    self.assertEqual(ok, b'OK')
    ok = self.exec_naked("REDISQL.V2.CREATE_DB", "DB3",)
    self.assertEqual(ok, b'OK')

  def test_must_create_flag(self):
    ok = self.exec_naked("REDISQL.V2.CREATE_DB", "DB4")
    self.assertEqual(ok, b'OK')
    with self.assertRaises(redis.exceptions.ResponseError):
      self.exec_naked("REDISQL.V2.CREATE_DB", "DB4", "MUST_CREATE")

class TestRediSQLExec(TestRediSQLWithExec):
  def test_ping(self):
    self.assertTrue(self.client.ping())

  def test_create_table(self):
    with DB(self, "A"):
      result = self.exec_nake("REDISQL.V2.EXEC", "A", "QUERY", "SELECT 1;")
      self.assertEqual(result, [[1]])


if __name__ == '__main__':
   unittest.main()
