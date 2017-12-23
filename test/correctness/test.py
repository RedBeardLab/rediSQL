#!/usr/bin/python -tt
# -*- coding: utf-8 -*-

import unittest
import os

import redis
from rmtest import ModuleTestCase
    
os.environ["REDIS_MODULE_PATH"] = "/home/simo/rediSQL/target/release/libredis_sql.so"
os.environ["REDIS_PATH"] = "/home/simo/redis-4.0.2/src/redis-server"
 
class Table():
  def __init__(self, redis, name, values, key = ""):
    self.redis = redis
    self.key = key
    self.name = name
    self.values = values

  def __enter__(self):
    create_table = "CREATE TABLE {} {}".format(self.name, self.values)
    if self.key:
      self.redis.execute_command("REDISQL.EXEC", self.key, create_table) 
    else:
      self.redis.execute_command("REDISQL.EXEC", create_table)
  
  def __exit__(self, type, value, traceback):
    drop_table = "DROP TABLE {}".format(self.name)
    if self.key:
      self.redis.execute_command("REDISQL.EXEC", self.key, drop_table)
    else:
      self.redis.execute_command("REDISQL.EXEC", drop_table)

class DB():
  def __init__(self, redis, key):
    self.redis = redis
    self.key = key

  def __enter__(self):
    self.redis.execute_command("REDISQL.CREATE_DB", self.key)

  def __exit__(self, type, value, traceback):
    self.redis.execute_command("DEL", self.key)


class TestRediSQLWithExec(ModuleTestCase('')):
  def setUp(self):
    self.disposable_redis = self.redis()
    self.client = self.disposable_redis.__enter__()

  def tearDown(self):
    self.disposable_redis.__exit__(None, None, None)
    del self.client

  def exec_naked(self, *command):
    return self.client.execute_command(*command)

  def exec_cmd(self, *command):
    return self.client.execute_command("REDISQL.EXEC", *command)

  def create_db(self, key):
    return self.client.execute_command("REDISQL.CREATE_DB", key)

  def delete_db(self, key):
    return self.client.execute_command("DEL", key)

class TestRediSQLExec(TestRediSQLWithExec):
  def test_ping(self):
    self.assertTrue(self.client.ping())

  def test_create_table(self):
    with DB(self.client, "A"):
      done = self.exec_cmd("A", "CREATE TABLE test1 (A INTEGER);")
      self.assertEquals(done, ["DONE", 0L])
      done = self.exec_cmd("A", "DROP TABLE test1")
      self.assertEquals(done, ["DONE", 0L])

  def test_insert(self):
    with DB(self.client, "B"):
      with Table(self.client, "test2", "(A INTEGER)", key = "B"):
        done = self.exec_cmd("B", "INSERT INTO test2 VALUES(2);")
        self.assertEquals(done, ["DONE", 1L])

  def test_select(self):
    with DB(self.client, "C"):
      with Table(self.client, "test3", "(A INTEGER)", key = "C"):
        done = self.exec_cmd("C", "INSERT INTO test3 VALUES(2);")
        self.assertEquals(done, ["DONE", 1L])
      
        result = self.exec_cmd("C", "SELECT * from test3")
        self.assertEquals(result, [[2]])
      
        self.exec_cmd("C", "INSERT INTO test3 VALUES(3);")
        result = self.exec_cmd("C", "SELECT * from test3 ORDER BY A")
        self.assertEquals(result, [[2], [3]])

        self.exec_cmd("C", "INSERT INTO test3 VALUES(4);")
        result = self.exec_cmd("C", "SELECT * FROM test3 ORDER BY A")
        self.assertEquals(result, [[2], [3], [4]])
  
  def test_single_remove(self):
    with DB(self.client, "D"):
      with Table(self.client, "test4", "(A INTEGER)", key = "D"):
        self.exec_cmd("D", "INSERT INTO test4 VALUES(2);")
        self.exec_cmd("D", "INSERT INTO test4 VALUES(3);")
        self.exec_cmd("D", "INSERT INTO test4 VALUES(4);")
        result = self.exec_cmd("D", "SELECT * FROM test4 ORDER BY A")
        self.assertEquals(result, [[2], [3], [4]])
        self.exec_cmd("D", "DELETE FROM test4 WHERE A = 3;")
        result = self.exec_cmd("D", "SELECT * FROM test4 ORDER BY A")
        self.assertEquals(result, [[2], [4]])
 
  @unittest.skip("Quite slow investigate")
  def test_big_select(self):
    elements = 50
    with DB(self.client, "E"):
      with Table(self.client, "test5", "(A INTERGER)", key = "E"):
        pipe = self.client.pipeline(transaction=False)
        for i in xrange(elements):
          pipe.execute_command("REDISQL.EXEC", "E", 
              "INSERT INTO test5 VALUES({})".format(i))
        pipe.execute()
        result = self.exec_cmd("E", "SELECT * FROM test5 ORDER BY A")
        self.assertEquals(result, [[x] for x in xrange(elements)])
 
  def test_multiple_row(self):
    with DB(self.client, "F"):
      with Table(self.client, "test6", "(A INTEGER, B REAL, C TEXT)", key= "F"):
        self.exec_cmd("F", "INSERT INTO test6 VALUES(1, 1.0, '1point1')")
        self.exec_cmd("F", "INSERT INTO test6 VALUES(2, 2.0, '2point2')")
        self.exec_cmd("F", "INSERT INTO test6 VALUES(3, 3.0, '3point3')")
        self.exec_cmd("F", "INSERT INTO test6 VALUES(4, 4.0, '4point4')")
        self.exec_cmd("F", "INSERT INTO test6 VALUES(5, 5.0, '5point5')")
  
        result = self.exec_cmd("F", "SELECT A, B, C FROM test6 ORDER BY A")
        result = [[A, float(B), C] for [A, B, C] in result]
        self.assertEquals(result, 
            [[1L, 1.0, "1point1"], [2L, 2.0, '2point2'],
             [3L, 3.0, '3point3'], [4L, 4.0, '4point4'],
             [5L, 5.0, '5point5']])
        
  def test_join(self):
    with DB(self.client, "G"):
      with Table(self.client, "testA", "(A INTEGER, B INTEGER)", key = "G"):
        with Table(self.client, "testB", "(C INTEGER, D INTEGER)", key = "G"):
          self.exec_cmd("G", "INSERT INTO testA VALUES(1, 2)")
          self.exec_cmd("G", "INSERT INTO testA VALUES(3, 4)")
          self.exec_cmd("G", "INSERT INTO testB VALUES(1, 2)")
          self.exec_cmd("G", "INSERT INTO testB VALUES(3, 4)")
          result = self.exec_cmd("G", "SELECT A, B, C, D FROM testA, testB WHERE A = C ORDER BY A")
          self.assertEquals(result, [[1, 2, 1, 2], [3, 4, 3, 4]])
        

  def runTest(self):
    pass

class NoDefaultDB(TestRediSQLWithExec):
  def test_that_we_need_a_key(self):
    with self.assertRaises(redis.exceptions.ResponseError):
      self.exec_cmd("SELECT 'foo';")

class TestRediSQLKeys(TestRediSQLWithExec):
  def test_create_and_destroy_key(self):
    ok = self.create_db("A_REDISQL")
    self.assertEquals(ok, "OK")
    keys = self.client.keys("A_REDISQL")
    self.assertEquals(["A_REDISQL"], keys)
    ok = self.delete_db("A_REDISQL")
    keys = self.client.keys("A_REDISQL")
    self.assertEquals([], keys)

  def test_create_table_inside_key(self):
    with DB(self.client, "A"):
      done = self.exec_cmd("A", "CREATE TABLE t1 (A INTEGER);")
      self.assertEquals(done, ["DONE", 0L])
      done = self.exec_cmd("A", "DROP TABLE t1")
      self.assertEquals(done, ["DONE", 0L])

  def test_insert_into_table(self):
    with DB(self.client, "B"):
      with Table(self.client, "t2", "(A INTEGER, B INTEGER)", key = "B"):
        done = self.exec_cmd("B", "INSERT INTO t2 VALUES(1,2)")
        self.assertEquals(done, ["DONE", 1L])
        result = self.exec_cmd("B", "SELECT * FROM t2")
        self.assertEquals(result, [[1, 2]])

class TestMultipleInserts(TestRediSQLWithExec):
  def test_insert_two_rows(self):
    with DB(self.client, "M"):
      with Table(self.client, "t1", "(A INTEGER, B INTEGER)", key = "M"):
        done = self.exec_naked("REDISQL.EXEC", "M", "INSERT INTO t1 values(1, 2);")
        self.assertEquals(done, ["DONE", 1L])
        done = self.exec_naked("REDISQL.EXEC", "M", "INSERT INTO t1 values(3, 4),(5, 6);")
        self.assertEquals(done, ["DONE", 2L])
        done = self.exec_naked("REDISQL.EXEC", "M", "INSERT INTO t1 values(7, 8);")
        self.assertEquals(done, ["DONE", 1L])
 
  def test_multi_insert_same_statement(self):
    with DB(self.client, "N"):
      with Table(self.client, "t1", "(A INTEGER, B INTEGER)", key = "N"):
        done = self.exec_naked("REDISQL.EXEC", "N", "INSERT INTO t1 values(1, 2); INSERT INTO t1 values(3, 4);")
        self.assertEquals(done, ["DONE", 2L])
        done = self.exec_naked("REDISQL.EXEC", "N", """BEGIN; 
              INSERT INTO t1 values(3, 4);
              INSERT INTO t1 values(5, 6);
              INSERT INTO t1 values(7, 8);
              COMMIT;""")
        self.assertEquals(done, ["DONE", 3L])
        done = self.exec_naked("REDISQL.EXEC", "N", """BEGIN; 
              INSERT INTO t1 values(3, 4);
              INSERT INTO t1 values(5, 6);
              INSERT INTO t1 values(7, 8);
              INSERT INTO t1 values(3, 4);
              INSERT INTO t1 values(5, 6);
              INSERT INTO t1 values(7, 8);
              COMMIT;""")
        self.assertEquals(done, ["DONE", 6L])

class TestJSON(TestRediSQLWithExec):
  def test_multiple_insert_on_different_types(self):
    with DB(self.client, "H"):
      with Table(self.client, "j1", "(A text, B int)", key = "H"):
        done = self.exec_naked("REDISQL.EXEC", "H", """BEGIN;
              INSERT INTO j1 VALUES ('{\"foo\" : \"bar\"}', 1);
              INSERT INTO j1 VALUES ('{\"foo\" : 3}', 2);
              INSERT INTO j1 VALUES ('{\"foo\" : [1, 2, 3]}', 3);
              INSERT INTO j1 VALUES ('{\"foo\" : {\"baz\" : [1, 2, 3]}}', 4);
              COMMIT;""")
        self.assertEquals(done, ["DONE", 4L])
        result = self.exec_naked("REDISQL.EXEC", "H", "SELECT json_extract(A, '$.foo') FROM j1 ORDER BY B;")
        self.assertEquals(result, [["bar"], [3], ["[1,2,3]"], ['{"baz":[1,2,3]}']])


class TestStatements(TestRediSQLWithExec):
  def test_create_statement(self):
    with DB(self.client, "A"):
      with Table(self.client, "t1", "(A INTEGER)", key = "A"):
        ok = self.exec_naked("REDISQL.CREATE_STATEMENT", "A", "insert", "insert into t1 values(?1);")
        self.assertEquals(ok, "OK")
        done = self.exec_naked("REDISQL.EXEC_STATEMENT", "A", "insert", "3")
        self.assertEquals(done, ["DONE", 1L])
        done = self.exec_naked("REDISQL.EXEC_STATEMENT", "A", "insert", "4")
        self.assertEquals(done, ["DONE", 1L])
        result = self.exec_cmd("A", "SELECT * FROM t1 ORDER BY A;")
        self.assertEquals(result, [[3], [4]])

  def test_update_statement(self):
    with DB(self.client, "A"):
      with Table(self.client, "t1", "(A INTEGER)", key = "A"):
        ok = self.exec_naked("REDISQL.CREATE_STATEMENT", "A", "insert", "insert into t1 values(?1);")
        self.assertEquals(ok, "OK")
        done = self.exec_naked("REDISQL.EXEC_STATEMENT", "A", "insert", "3")
        self.assertEquals(done, ["DONE", 1L])
        ok = self.exec_naked("REDISQL.UPDATE_STATEMENT", "A", "insert", "insert into t1 values(?1 + 10001);")
        self.assertEquals(ok, "OK")
        done = self.exec_naked("REDISQL.EXEC_STATEMENT", "A", "insert", "4")
        self.assertEquals(done, ["DONE", 1L])
        result = self.exec_cmd("A", "SELECT * FROM t1 ORDER BY A;")
        self.assertEquals(result, [[3], [10005]])

  def test_rds_persistency(self):
    with DB(self.client, "A"):
      with Table(self.client, "t1", "(A INTEGER)", key = "A"):
        ok = self.exec_naked("REDISQL.CREATE_STATEMENT", "A", "insert", "insert into t1 values(?1);")
        self.assertEquals(ok, "OK")
        done = self.exec_naked("REDISQL.EXEC_STATEMENT", "A", "insert", "3")
        self.assertEquals(done, ["DONE", 1L])
        self.disposable_redis.dump_and_reload()
        done = self.exec_naked("REDISQL.EXEC_STATEMENT", "A", "insert", "4")
        self.assertEquals(done, ["DONE", 1L])
        result = self.exec_cmd("A", "SELECT * FROM t1 ORDER BY A;")
        self.assertEquals(result, [[3], [4]])

  def test_rds_persistency_no_statements(self):
    with DB(self.client, "A"):
      with Table(self.client, "t1", "(A INTEGER)", key = "A"):
        
        done = self.exec_cmd("A", "INSERT INTO t1 VALUES(5)")
        self.assertEquals(done, ["DONE", 1L])

        self.disposable_redis.dump_and_reload()
        done = self.exec_cmd("A", "INSERT INTO t1 VALUES(6)")
        self.assertEquals(done, ["DONE", 1L])
        
        result = self.exec_cmd("A", "SELECT * FROM t1 ORDER BY A;")
        self.assertEquals(result, [[5], [6]])

  def test_rds_persistency_multiple_statements(self):
    with DB(self.client, "A"):
      with Table(self.client, "t1", "(A INTEGER)", key = "A"):
        ok = self.exec_naked("REDISQL.CREATE_STATEMENT", "A", "insert", "insert into t1 values(?1);")
        self.assertEquals(ok, "OK")
        ok = self.exec_naked("REDISQL.CREATE_STATEMENT", "A", "insert più cento", "insert into t1 values(?1 + 100);")
        self.assertEquals(ok, "OK")

        done = self.exec_naked("REDISQL.EXEC_STATEMENT", "A", "insert", "3")
        self.assertEquals(done, ["DONE", 1L])
        done = self.exec_naked("REDISQL.EXEC_STATEMENT", "A", "insert più cento", "3")
        self.assertEquals(done, ["DONE", 1L])

        self.disposable_redis.dump_and_reload()
        done = self.exec_naked("REDISQL.EXEC_STATEMENT", "A", "insert", "4")
        self.assertEquals(done, ["DONE", 1L])
        done = self.exec_naked("REDISQL.EXEC_STATEMENT", "A", "insert più cento", "4")
        self.assertEquals(done, ["DONE", 1L])
        
        result = self.exec_cmd("A", "SELECT * FROM t1 ORDER BY A;")
        self.assertEquals(result, [[3], [4], [103], [104]])


if __name__ == '__main__':
   unittest.main()

