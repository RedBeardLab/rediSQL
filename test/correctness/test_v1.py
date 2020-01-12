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
    self.redis.client.execute_command("REDISQL.V1.CREATE_DB", self.key)

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
    return self.client.execute_command("REDISQL.V1.CREATE_DB", key)

  def delete_db(self, key):
    return self.client.execute_command("DEL", key)

class TestRediSQLExec(TestRediSQLWithExec):
  def test_ping(self):
    self.assertTrue(self.client.ping())

  def test_create_table(self):
    with DB(self, "A"):
      done = self.exec_cmd("A", "CREATE TABLE test1 (A INTEGER);")
      self.assertEquals(done, ["DONE", 0])
      done = self.exec_cmd("A", "DROP TABLE test1")
      self.assertEquals(done, ["DONE", 0])

  def test_insert(self):
    with DB(self, "B"):
      with Table(self, "test2", "(A INTEGER)", key = "B"):
        done = self.exec_cmd("B", "INSERT INTO test2 VALUES(2);")
        self.assertEquals(done, ["DONE", 1])

  def test_select(self):
    with DB(self, "C"):
      with Table(self, "test3", "(A INTEGER)", key = "C"):
        done = self.exec_cmd("C", "INSERT INTO test3 VALUES(2);")
        self.assertEquals(done, ["DONE", 1])

        result = self.exec_cmd("C", "SELECT * from test3")
        self.assertEquals(result, [[2]])

        self.exec_cmd("C", "INSERT INTO test3 VALUES(3);")
        result = self.exec_cmd("C", "SELECT * from test3 ORDER BY A")
        self.assertEquals(result, [[2], [3]])

        self.exec_cmd("C", "INSERT INTO test3 VALUES(4);")
        result = self.exec_cmd("C", "SELECT * FROM test3 ORDER BY A")
        self.assertEquals(result, [[2], [3], [4]])

  def test_single_remove(self):
    with DB(self, "D"):
      with Table(self, "test4", "(A INTEGER)", key = "D"):
        self.exec_cmd("D", "INSERT INTO test4 VALUES(2);")
        self.exec_cmd("D", "INSERT INTO test4 VALUES(3);")
        self.exec_cmd("D", "INSERT INTO test4 VALUES(4);")
        result = self.exec_cmd("D", "SELECT * FROM test4 ORDER BY A")
        self.assertEquals(result, [[2], [3], [4]])
        self.exec_cmd("D", "DELETE FROM test4 WHERE A = 3;")
        result = self.exec_cmd("D", "SELECT * FROM test4 ORDER BY A")
        self.assertEquals(result, [[2], [4]])

  def test_big_select(self):
    elements = 50
    with DB(self, "E"):
      with Table(self, "test5", "(A INTERGER)", key = "E"):
        pipe = self.client.pipeline(transaction=False)
        for i in xrange(elements):
          pipe.execute_command("REDISQL.V1.EXEC", "E", 
              "INSERT INTO test5 VALUES({})".format(i))
        pipe.execute()
        result = self.exec_cmd("E", "SELECT * FROM test5 ORDER BY A")
        self.assertEquals(result, [[x] for x in xrange(elements)])

  def test_multiple_row(self):
    with DB(self, "F"):
      with Table(self, "test6", "(A INTEGER, B REAL, C TEXT)", key= "F"):
        self.exec_cmd("F", "INSERT INTO test6 VALUES(1, 1.0, '1point1')")
        self.exec_cmd("F", "INSERT INTO test6 VALUES(2, 2.0, '2point2')")
        self.exec_cmd("F", "INSERT INTO test6 VALUES(3, 3.0, '3point3')")
        self.exec_cmd("F", "INSERT INTO test6 VALUES(4, 4.0, '4point4')")
        self.exec_cmd("F", "INSERT INTO test6 VALUES(5, 5.0, '5point5')")

        result = self.exec_cmd("F", "SELECT A, B, C FROM test6 ORDER BY A")
        result = [[A, float(B), C] for [A, B, C] in result]
        self.assertEquals(result, 
            [[1, 1.0, "1point1"], [2, 2.0, '2point2'],
             [3, 3.0, '3point3'], [4, 4.0, '4point4'],
             [5, 5.0, '5point5']])

  def test_join(self):
    with DB(self, "G"):
      with Table(self, "testA", "(A INTEGER, B INTEGER)", key = "G"):
        with Table(self, "testB", "(C INTEGER, D INTEGER)", key = "G"):
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
    with DB(self, "A"):
      done = self.exec_cmd("A", "CREATE TABLE t1 (A INTEGER);")
      self.assertEquals(done, ["DONE", 0])
      done = self.exec_cmd("A", "DROP TABLE t1")
      self.assertEquals(done, ["DONE", 0])

  def test_insert_into_table(self):
    with DB(self, "B"):
      with Table(self, "t2", "(A INTEGER, B INTEGER)", key = "B"):
        done = self.exec_cmd("B", "INSERT INTO t2 VALUES(1,2)")
        self.assertEquals(done, ["DONE", 1])
        result = self.exec_cmd("B", "SELECT * FROM t2")
        self.assertEquals(result, [[1, 2]])

class TestMultipleInserts(TestRediSQLWithExec):
  def test_insert_two_rows(self):
    with DB(self, "M"):
      with Table(self, "t1", "(A INTEGER, B INTEGER)", key = "M"):
        done = self.exec_naked("REDISQL.V1.EXEC", "M", "INSERT INTO t1 values(1, 2);")
        self.assertEquals(done, ["DONE", 1])
        done = self.exec_naked("REDISQL.V1.EXEC", "M", "INSERT INTO t1 values(3, 4),(5, 6);")
        self.assertEquals(done, ["DONE", 2])
        done = self.exec_naked("REDISQL.V1.EXEC", "M", "INSERT INTO t1 values(7, 8);")
        self.assertEquals(done, ["DONE", 1])

  def test_multi_insert_same_statement(self):
    with DB(self, "N"):
      with Table(self, "t1", "(A INTEGER, B INTEGER)", key = "N"):
        done = self.exec_naked("REDISQL.V1.EXEC", "N", "INSERT INTO t1 values(1, 2); INSERT INTO t1 values(3, 4);")
        self.assertEquals(done, ["DONE", 2])
        done = self.exec_naked("REDISQL.V1.EXEC", "N", """BEGIN; 
              INSERT INTO t1 values(3, 4);
              INSERT INTO t1 values(5, 6);
              INSERT INTO t1 values(7, 8);
              COMMIT;""")
        self.assertEquals(done, ["DONE", 3])
        done = self.exec_naked("REDISQL.V1.EXEC", "N", """BEGIN; 
              INSERT INTO t1 values(3, 4);
              INSERT INTO t1 values(5, 6);
              INSERT INTO t1 values(7, 8);
              INSERT INTO t1 values(3, 4);
              INSERT INTO t1 values(5, 6);
              INSERT INTO t1 values(7, 8);
              COMMIT;""")
        self.assertEquals(done, ["DONE", 6])

class TestJSON(TestRediSQLWithExec):
  def test_multiple_insert_on_different_types(self):
    with DB(self, "H"):
      with Table(self, "j1", "(A text, B int)", key = "H"):
        done = self.exec_naked("REDISQL.V1.EXEC", "H", """BEGIN;
              INSERT INTO j1 VALUES ('{\"foo\" : \"bar\"}', 1);
              INSERT INTO j1 VALUES ('{\"foo\" : 3}', 2);
              INSERT INTO j1 VALUES ('{\"foo\" : [1, 2, 3]}', 3);
              INSERT INTO j1 VALUES ('{\"foo\" : {\"baz\" : [1, 2, 3]}}', 4);
              COMMIT;""")
        self.assertEquals(done, ["DONE", 4])
        result = self.exec_naked("REDISQL.V1.EXEC", "H", "SELECT json_extract(A, '$.foo') FROM j1 ORDER BY B;")
        self.assertEquals(result, [["bar"], [3], ["[1,2,3]"], ['{"baz":[1,2,3]}']])


class TestStatements(TestRediSQLWithExec):
  def test_create_statement(self):
    with DB(self, "A"):
      with Table(self, "t1", "(A INTEGER)", key = "A"):
        ok = self.exec_naked("REDISQL.V1.CREATE_STATEMENT", "A", "insert", "insert into t1 values(?1);")
        self.assertEquals(ok, "OK")
        done = self.exec_naked("REDISQL.V1.EXEC_STATEMENT", "A", "insert", "3")
        self.assertEquals(done, ["DONE", 1])
        done = self.exec_naked("REDISQL.V1.EXEC_STATEMENT", "A", "insert", "4")
        self.assertEquals(done, ["DONE", 1])
        result = self.exec_cmd("A", "SELECT * FROM t1 ORDER BY A;")
        self.assertEquals(result, [[3], [4]])

  def test_multi_statement_single_bind(self):
    with DB(self, "A"):
      with Table(self, "t1", "(A INTEGER)", key = "A"):
        ok = self.exec_naked("REDISQL.V1.CREATE_STATEMENT", "A", "insert", "insert into t1 values(?1); insert into t1 values(?1 + 1);")
        self.assertEquals(ok, "OK")
        done = self.exec_naked("REDISQL.V1.EXEC_STATEMENT", "A", "insert", "3")
        self.assertEquals(done, ["DONE", 2])
        done = self.exec_naked("REDISQL.V1.EXEC_STATEMENT", "A", "insert", "5")
        self.assertEquals(done, ["DONE", 2])
        result = self.exec_cmd("A", "SELECT * FROM t1 ORDER BY A;")
        self.assertEquals(result, [[3], [4], [5], [6]])

  def test_multi_statement_multi_table_single_bind(self):
    with DB(self, "A"):
      with Table(self, "t1", "(A INTEGER)", key = "A"):
        with Table(self, "t2", "(A INTEGER)", key = "A"):
          ok = self.exec_naked("REDISQL.V1.CREATE_STATEMENT", "A", "insert", "insert into t1 values(?1); insert into t2 values(?1 - 1);")
          self.assertEquals(ok, "OK")
          done = self.exec_naked("REDISQL.V1.EXEC_STATEMENT", "A", "insert", "3")
          self.assertEquals(done, ["DONE", 2])
          done = self.exec_naked("REDISQL.V1.EXEC_STATEMENT", "A", "insert", "5")
          self.assertEquals(done, ["DONE", 2])

          result = self.exec_cmd("A", "SELECT * FROM t1 ORDER BY A;")
          self.assertEquals(result, [[3], [5]])

          result = self.exec_cmd("A", "SELECT * FROM t2 ORDER BY A;")
          self.assertEquals(result, [[2], [4]])

  def test_multi_statement_different_bindings(self):
    with DB(self, "A"):
      with Table(self, "t1", "(A INTEGER)", key = "A"):
        ok = self.exec_naked("REDISQL.V1.CREATE_STATEMENT", "A", "insert", "insert into t1 values(?1); insert into t1 values(?2 + 1); select * from t1;")
        self.assertEquals(ok, "OK")
        result = self.exec_naked("REDISQL.V1.EXEC_STATEMENT", "A", "insert", "3", "8")
        self.assertEquals(result, [[3], [9]])


  def test_update_statement(self):
    with DB(self, "A"):
      with Table(self, "t1", "(A INTEGER)", key = "A"):
        ok = self.exec_naked("REDISQL.V1.CREATE_STATEMENT", "A", "insert", "insert into t1 values(?1);")
        self.assertEquals(ok, "OK")
        done = self.exec_naked("REDISQL.V1.EXEC_STATEMENT", "A", "insert", "3")
        self.assertEquals(done, ["DONE", 1])
        ok = self.exec_naked("REDISQL.V1.UPDATE_STATEMENT", "A", "insert", "insert into t1 values(?1 + 10001);")
        self.assertEquals(ok, "OK")
        done = self.exec_naked("REDISQL.V1.EXEC_STATEMENT", "A", "insert", "4")
        self.assertEquals(done, ["DONE", 1])
        result = self.exec_cmd("A", "SELECT * FROM t1 ORDER BY A;")
        self.assertEquals(result, [[3], [10005]])

  def test_rdb_persistency(self):
    with DB(self, "A"):
      with Table(self, "t1", "(A INTEGER)", key = "A"):
        ok = self.exec_naked("REDISQL.V1.CREATE_STATEMENT", "A", "insert", "insert into t1 values(?1);")
        self.assertEquals(ok, "OK")
        done = self.exec_naked("REDISQL.V1.EXEC_STATEMENT", "A", "insert", "3")
        self.assertEquals(done, ["DONE", 1])
        for _ in self.retry_with_reload():
          pass
        time.sleep(0.5)

        done = self.exec_naked("REDISQL.V1.EXEC_STATEMENT", "A", "insert", "4")
        self.assertEquals(done, ["DONE", 1])
        result = self.exec_cmd("A", "SELECT * FROM t1 ORDER BY A;")
        self.assertEquals(result, [[3], [4]])

  def test_rdb_persistency_no_statements(self):
    with DB(self, "A"):
      with Table(self, "t1", "(A INTEGER)", key = "A"):

        done = self.exec_cmd("A", "INSERT INTO t1 VALUES(5)")
        self.assertEquals(done, ["DONE", 1])

        for _ in self.retry_with_reload():
          pass
        time.sleep(0.5)

        done = self.exec_cmd("A", "INSERT INTO t1 VALUES(6)")
        self.assertEquals(done, ["DONE", 1])

        result = self.exec_cmd("A", "SELECT * FROM t1 ORDER BY A;")
        self.assertEquals(result, [[5], [6]])

  def test_rdb_persistency_multiple_statements(self):
    with DB(self, "A"):
      with Table(self, "t1", "(A INTEGER)", key = "A"):
        ok = self.exec_naked("REDISQL.V1.CREATE_STATEMENT", "A", "insert", "insert into t1 values(?1);")
        self.assertEquals(ok, "OK")
        ok = self.exec_naked("REDISQL.V1.CREATE_STATEMENT", "A", "insert più cento", "insert into t1 values(?1 + 100);")
        self.assertEquals(ok, "OK")

        done = self.exec_naked("REDISQL.V1.EXEC_STATEMENT", "A", "insert", "3")
        self.assertEquals(done, ["DONE", 1])
        done = self.exec_naked("REDISQL.V1.EXEC_STATEMENT", "A", "insert più cento", "3")
        self.assertEquals(done, ["DONE", 1])

        for _ in self.retry_with_reload():
          pass
        time.sleep(0.5)

        done = self.exec_naked("REDISQL.V1.EXEC_STATEMENT", "A", "insert", "4")
        self.assertEquals(done, ["DONE", 1])
        done = self.exec_naked("REDISQL.V1.EXEC_STATEMENT", "A", "insert più cento", "4")
        self.assertEquals(done, ["DONE", 1])

        result = self.exec_cmd("A", "SELECT * FROM t1 ORDER BY A;")
        self.assertEquals(result, [[3], [4], [103], [104]])

class TestSynchronous(TestRediSQLWithExec):
  def test_exec(self):
    with DB(self, "A"):
      done = self.exec_naked("REDISQL.V1.EXEC.NOW", "A", "CREATE TABLE test(a INT, b TEXT);")
      self.assertEquals(done, ["DONE", 0])
      done = self.exec_naked("REDISQL.V1.EXEC.NOW", "A", "INSERT INTO test VALUES(1, 'ciao'), (2, 'foo'), (100, 'baz');")
      self.assertEquals(done, ["DONE", 3])
      result = self.exec_naked("REDISQL.V1.EXEC.NOW", "A", "SELECT * FROM test ORDER BY a ASC")
      self.assertEquals(result, [[1, 'ciao'], [2, 'foo'], [100, 'baz']])

  def test_statements(self):
    with DB(self, "A"):
      with Table(self, "t1", "(A INTEGER)", key = "A"):
        ok = self.exec_naked("REDISQL.V1.CREATE_STATEMENT.NOW", "A", "insert+100", "INSERT INTO t1 VALUES(100 + ?1);")
        self.assertEquals(ok, "OK")
        done = self.exec_naked("REDISQL.V1.EXEC_STATEMENT", "A", "insert+100", 1)
        self.assertEquals(done, ["DONE", 1])
        done = self.exec_naked("REDISQL.V1.EXEC_STATEMENT.NOW", "A", "insert+100", 9)
        self.assertEquals(done, ["DONE", 1])
        ok = self.exec_naked("REDISQL.V1.CREATE_STATEMENT", "A", "query-100", "SELECT A-100 FROM t1 ORDER BY A ASC;")
        self.assertEquals(ok, "OK")
        result = self.exec_naked("REDISQL.V1.EXEC_STATEMENT", "A", "query-100")
        self.assertEquals(result, [[1], [9]])
        result = self.exec_naked("REDISQL.V1.EXEC_STATEMENT.NOW", "A", "query-100")
        self.assertEquals(result, [[1], [9]])

class TestRead(TestRediSQLWithExec):
  def test_read(self):
    with DB(self, "A"):
      with Table(self, "t1", "(A INTEGER)", key = "A"):
        done = self.exec_naked("REDISQL.V1.EXEC", "A", "INSERT INTO t1 VALUES(4);")
        result = self.exec_naked("REDISQL.V1.QUERY", "A", "SELECT A FROM t1 LIMIT 1;")
        self.assertEquals(result, [[4]])

  def test_not_insert(self):
    with DB(self, "B"):
      with Table(self, "t1", "(A INTEGER)", key = "B"):
        with self.assertRaises(redis.exceptions.ResponseError):
          self.exec_naked("REDISQL.V1.QUERY", "B", "INSERT INTO t1 VALUES(5);")
        done = self.exec_naked("REDISQL.V1.EXEC", "B", "CREATE TABLE test(a INT, b TEXT);")
        self.assertEquals(done, ["DONE", 0])
        done = self.exec_naked("REDISQL.V1.EXEC", "B", "INSERT INTO test VALUES(1, 'ciao'), (2, 'foo'), (100, 'baz');")
        self.assertEquals(done, ["DONE", 3])
        result = self.exec_naked("REDISQL.V1.QUERY", "B", "SELECT * FROM test ORDER BY a ASC")
        self.assertEquals(result, [[1, 'ciao'], [2, 'foo'], [100, 'baz']])

class TestCopy(TestRediSQLWithExec):
    def test_copy_mem_from_mem(self):
        done = self.exec_naked("REDISQL.V1.CREATE_DB", "DB1")
        self.assertEquals(done, "OK")
        done = self.exec_naked("REDISQL.V1.CREATE_DB", "DB2A")
        self.assertEquals(done, "OK")
        done = self.exec_naked("REDISQL.V1.EXEC", "DB1", "CREATE TABLE foo(a INT);")
        self.assertEquals(done, ["DONE", 0])

        for i in xrange(10):
            done = self.exec_naked("REDISQL.V1.EXEC", "DB1", "INSERT INTO foo VALUES({})".format(i))
            self.assertEquals(done, ["DONE", 1])

        done = self.exec_naked("REDISQL.V1.COPY", "DB1", "DB2A")

        result = self.exec_naked("REDISQL.V1.QUERY", "DB1", "SELECT a FROM foo ORDER BY a")
        self.assertEquals(result, [[0], [1], [2], [3], [4], [5], [6], [7], [8], [9]])

        result = self.exec_naked("REDISQL.V1.QUERY", "DB2A", "SELECT a FROM foo ORDER BY a")
        self.assertEquals(result, [[0], [1], [2], [3], [4], [5], [6], [7], [8], [9]])

    def test_statements_copy_mem_from_mem(self):
        done = self.exec_naked("REDISQL.V1.CREATE_DB", "DB1")
        self.assertEquals(done, "OK")
        done = self.exec_naked("REDISQL.V1.CREATE_DB", "DB2B")
        self.assertEquals(done, "OK")
        done = self.exec_naked("REDISQL.V1.CREATE_STATEMENT", "DB1", "select1", "SELECT 1;")
        self.assertEquals(done, "OK")

        done = self.exec_naked("REDISQL.V1.COPY", "DB1", "DB2B")
        self.assertEquals(done, "OK")

        result = self.exec_naked("REDISQL.V1.QUERY_STATEMENT", "DB1", "select1")
        self.assertEquals(result, [[1]])

        result = self.exec_naked("REDISQL.V1.QUERY_STATEMENT", "DB2B", "select1")
        self.assertEquals(result, [[1]])

    def test_double_copy(self):
        done = self.exec_naked("REDISQL.V1.CREATE_DB", "DB1")
        self.assertEquals(done, "OK")
        done = self.exec_naked("REDISQL.V1.CREATE_DB", "DB2C")
        self.assertEquals(done, "OK")
        done = self.exec_naked("REDISQL.V1.CREATE_STATEMENT", "DB1", "select1", "SELECT 1;")
        self.assertEquals(done, "OK")

        first_copy = self.exec_naked("REDISQL.V1.COPY", "DB1", "DB2C")
        self.assertEquals(first_copy, "OK")
        result = self.exec_naked("REDISQL.V1.QUERY_STATEMENT", "DB1", "select1")
        self.assertEquals(result, [[1]])

        second_copy = self.exec_naked("REDISQL.V1.COPY", "DB1", "DB2C")
        self.assertEquals(second_copy, "OK")
        result = self.exec_naked("REDISQL.V1.QUERY_STATEMENT", "DB1", "select1")
        self.assertEquals(result, [[1]])

        result = self.exec_naked("REDISQL.V1.QUERY_STATEMENT", "DB2C", "select1")
        self.assertEquals(result, [[1]])

class TestCopySyncronous(TestRediSQLWithExec):
    def test_copy_now_mem_from_mem(self):
        done = self.exec_naked("REDISQL.V1.CREATE_DB", "DB1")
        self.assertEquals(done, "OK")
        done = self.exec_naked("REDISQL.V1.CREATE_DB", "DB2")
        self.assertEquals(done, "OK")
        done = self.exec_naked("REDISQL.V1.EXEC", "DB1", "CREATE TABLE foo(a INT);")
        self.assertEquals(done, ["DONE", 0])

        for i in xrange(10):
            done = self.exec_naked("REDISQL.V1.EXEC", "DB1", "INSERT INTO foo VALUES({})".format(i))
            self.assertEquals(done, ["DONE", 1])

        done = self.exec_naked("REDISQL.V1.COPY.NOW", "DB1", "DB2")

        result = self.exec_naked("REDISQL.V1.QUERY", "DB1", "SELECT a FROM foo ORDER BY a")
        self.assertEquals(result, [[0], [1], [2], [3], [4], [5], [6], [7], [8], [9]])

        result = self.exec_naked("REDISQL.V1.QUERY", "DB2", "SELECT a FROM foo ORDER BY a")
        elf.assertEquals(result, [[0], [1], [2], [3], [4], [5], [6], [7], [8], [9]])

    def test_statements_copy_now_mem_from_mem(self):
        done = self.exec_naked("REDISQL.V1.CREATE_DB", "DB1")
        self.assertEquals(done, "OK")
        done = self.exec_naked("REDISQL.V1.CREATE_DB", "DB2")
        self.assertEquals(done, "OK")
        done = self.exec_naked("REDISQL.V1.CREATE_STATEMENT", "DB1", "select1", "SELECT 1;")
        self.assertEquals(done, "OK")

        done = self.exec_naked("REDISQL.V1.COPY.NOW", "DB1", "DB2")
        self.assertEquals(done, "OK")

        result = self.exec_naked("REDISQL.V1.QUERY_STATEMENT", "DB1", "select1")
        self.assertEquals(result, [[1]])

        result = self.exec_naked("REDISQL.V1.QUERY_STATEMENT", "DB2", "select1")
        self.assertEquals(result, [[1]])

    def test_double_copy_now(self):
        done = self.exec_naked("REDISQL.V1.CREATE_DB", "DB1")
        self.assertEquals(done, "OK")
        done = self.exec_naked("REDISQL.V1.CREATE_DB", "DB2")
        self.assertEquals(done, "OK")
        done = self.exec_naked("REDISQL.V1.CREATE_STATEMENT", "DB1", "select1", "SELECT 1;")
        self.assertEquals(done, "OK")

        first_copy = self.exec_naked("REDISQL.V1.COPY.NOW", "DB1", "DB2")
        self.assertEquals(first_copy, "OK")
        result = self.exec_naked("REDISQL.V1.QUERY_STATEMENT", "DB1", "select1")
        self.assertEquals(result, [[1]])

        second_copy = self.exec_naked("REDISQL.V1.COPY.NOW", "DB1", "DB2")
        self.assertEquals(second_copy, "OK")
        result = self.exec_naked("REDISQL.V1.QUERY_STATEMENT", "DB1", "select1")
        self.assertEquals(result, [[1]])

        result = self.exec_naked("REDISQL.V1.QUERY_STATEMENT", "DB2", "select1")
        self.assertEquals(result, [[1]])

class TestBigInt(TestRediSQLWithExec):
    def test_big_int(self):
        with DB(self, "A"):
            done = self.exec_naked("REDISQL.V1.EXEC", "A", "CREATE TABLE ip_to_asn(start INT8, end INT8, asn int, hosts int8)")
            self.assertEquals(done, ["DONE", 0])

            done = self.exec_naked("REDISQL.V1.EXEC", "A", "insert into ip_to_asn values (2883484276228096000, 2883484280523063295, 265030, 4294967295)")
            self.assertEquals(done, ["DONE", 1])

            result = self.exec_naked("REDISQL.V1.EXEC", "A", "SELECT * FROM ip_to_asn;")
            self.assertEquals(result, [
                [
                    2883484276228096000,
                    2883484280523063295,
                    265030,
                    4294967295]
                ])

class TestStreams(TestRediSQLWithExec):
    def test_stream_query(self):
        with DB(self, "A"):
            total_len = 513
            done = self.exec_naked("REDISQL.V1.EXEC", "A", "CREATE TABLE foo(a int, b string, c int);")
            self.assertEquals(done, ["DONE", 0])

            for i in xrange(total_len):
                insert_stmt = "INSERT INTO foo VALUES({}, '{}', {})".format(i, "bar", i+1)
                done = self.exec_naked("REDISQL.V1.EXEC", "A", insert_stmt)
                self.assertEquals(done, ["DONE", 1])

            result = self.exec_naked("REDISQL.V1.QUERY.INTO", "{A}:1", "A", "SELECT * FROM foo")
            self.assertEquals(result[0][0], "{A}:1")
            self.assertEquals(result[0][3], 513)

            result = self.exec_naked("XRANGE", "{A}:1", "-", "+")
            self.assertEquals(len(result), total_len)

            for i, row in enumerate(result):
                self.assertEquals(row[1], ['int:a', str(i), 'text:b', "bar", 'int:c', str(i+1)])

    def test_stream_query_statement(self):
        with DB(self, "B"):
            total_len = 513
            done = self.exec_naked("REDISQL.V1.EXEC", "B", "CREATE TABLE foo(a int, b string, c int);")
            self.assertEquals(done, ["DONE", 0])

            for i in xrange(total_len):
                insert_stmt = "INSERT INTO foo VALUES({}, '{}', {})".format(i, "bar", i-1)
                done = self.exec_naked("REDISQL.V1.EXEC", "B", insert_stmt)
                self.assertEquals(done, ["DONE", 1])

            done = self.exec_naked("REDISQL.V1.CREATE_STATEMENT", "B", "select_all", "SELECT * FROM foo;")
            self.assertEquals(done, "OK")

            result = self.exec_naked("REDISQL.V1.QUERY_STATEMENT.INTO",
                    "{B}:1", "B", "select_all")
            self.assertEquals(result[0][0], "{B}:1")
            self.assertEquals(result[0][3], 513)

            result = self.exec_naked("XRANGE", "{B}:1", "-", "+")
            self.assertEquals(len(result), total_len)

            for i, row in enumerate(result):
                self.assertEquals(row[1], ['int:a', str(i), 'text:b', "bar", 'int:c', str(i-1)])

class TestStreamsSynchronous(TestRediSQLWithExec):
    def test_stream_query(self):
        with DB(self, "A"):
            total_len = 513
            done = self.exec_naked("REDISQL.V1.EXEC.NOW", "A", "CREATE TABLE foo(a int, b string, c int);")
            self.assertEquals(done, ["DONE", 0])

            for i in xrange(total_len):
                insert_stmt = "INSERT INTO foo VALUES({}, '{}', {})".format(i, "bar", i+1)
                done = self.exec_naked("REDISQL.V1.EXEC.NOW", "A", insert_stmt)
                self.assertEquals(done, ["DONE", 1])

            result = self.exec_naked("REDISQL.V1.QUERY.INTO.NOW", "{A}:1", "A", "SELECT * FROM foo")
            self.assertEquals(result[0][0], "{A}:1")
            self.assertEquals(result[0][3], 513)

            result = self.exec_naked("XRANGE", "{A}:1", "-", "+")
            self.assertEquals(len(result), total_len)

            for i, row in enumerate(result):
                self.assertEquals(row[1], ['int:a', str(i), 'text:b', "bar", 'int:c', str(i+1)])

    def test_stream_query_statement(self):
        with DB(self, "B"):
            total_len = 513
            done = self.exec_naked("REDISQL.V1.EXEC.NOW", "B", "CREATE TABLE foo(a int, b string, c int);")
            self.assertEquals(done, ["DONE", 0])

            for i in xrange(total_len):
                insert_stmt = "INSERT INTO foo VALUES({}, '{}', {})".format(i, "bar", i-1)
                done = self.exec_naked("REDISQL.V1.EXEC.NOW", "B", insert_stmt)
                self.assertEquals(done, ["DONE", 1])

            done = self.exec_naked("REDISQL.V1.CREATE_STATEMENT.NOW", "B", "select_all", "SELECT * FROM foo;")
            self.assertEquals(done, "OK")

            result = self.exec_naked("REDISQL.V1.QUERY_STATEMENT.INTO.NOW",
                    "{B}:1", "B", "select_all")
            self.assertEquals(result[0][0], "{B}:1")
            self.assertEquals(result[0][3], 513)

            result = self.exec_naked("XRANGE", "{B}:1", "-", "+")
            self.assertEquals(len(result), total_len)

            for i, row in enumerate(result):
                self.assertEquals(row[1], ['int:a', str(i), 'text:b', "bar", 'int:c', str(i-1)])


class TestFilePersistency(TestRediSQLWithExec):
  def test_creation_rdb_file(self):
    path = tempfile.mkdtemp()
    ok = self.exec_naked("REDISQL.V1.CREATE_DB", "A", path + "/foo.sqlite")
    self.assertEquals(ok, "OK")
    for _ in self.retry_with_reload():
      pass
    time.sleep(0.5)

    shutil.rmtree(path)
    self.assertTrue(True)

  def test_storage_of_data(self):
    path = tempfile.mkdtemp()
    ok = self.exec_naked("REDISQL.V1.CREATE_DB", "B", path + "/foo.sqlite")
    self.assertEquals(ok, "OK")
    done = self.exec_naked("REDISQL.V1.EXEC", "B", "CREATE TABLE bar(a,b);")
    self.assertEquals(done, ["DONE", 0])
    done = self.exec_naked("REDISQL.V1.EXEC", "B", "INSERT INTO bar VALUES(1,2);")
    self.assertEquals(done, ["DONE", 1])
    for _ in self.retry_with_reload():
      pass
    time.sleep(0.5)

    result = self.exec_naked("REDISQL.V1.QUERY", "B", "SELECT * FROM bar;")
    self.assertEquals(result, [[1, 2]])
    shutil.rmtree(path)
    self.assertTrue(True)

  def test_without_file(self):
    path = tempfile.mkdtemp()
    ok = self.exec_naked("REDISQL.V1.CREATE_DB", "B", path + "/foo.sqlite")
    self.assertEquals(ok, "OK")
    done = self.exec_naked("REDISQL.V1.EXEC", "B", "CREATE TABLE bar(a,b);")
    self.assertEquals(done, ["DONE", 0])
    done = self.exec_naked("REDISQL.V1.EXEC", "B", "INSERT INTO bar VALUES(1,2);")
    self.assertEquals(done, ["DONE", 1])
    os.remove(path + "/foo.sqlite")
    self.assertFalse(os.path.isfile(path + "/foo.sqlite"))
    for _ in self.retry_with_reload():
      pass
    time.sleep(0.5)

    result = self.exec_naked("REDISQL.V1.QUERY", "B", "SELECT * FROM bar;")
    self.assertEquals(result, [[1, 2]])
    self.assertTrue(os.path.isfile(path + "/foo.sqlite"))
    shutil.rmtree(path)
    self.assertTrue(True)

class TestNullTerminatedStrings(TestRediSQLWithExec):
  def test_null_terminated(self):
      with DB(self, "NULL"):
          one = self.exec_naked("REDISQL.V1.EXEC", "NULL", "SELECT 1" + b'\x00')
          self.assertEquals(one, [[1]])

class TestBlankAfterSemicolon(TestRediSQLWithExec):
  def test_whitespace_after_semicolon(self):
      with DB(self, "NULL"):
          one = self.exec_naked("REDISQL.V1.EXEC", "NULL", "SELECT 1;  ")
          self.assertEquals(one, [[1]])

  def test_newline_after_semicolon(self):
      with DB(self, "NULL"):
          one = self.exec_naked("REDISQL.V1.EXEC", "NULL", "SELECT 1;\n")
          self.assertEquals(one, [[1]])

  def test_mix_after_semicolon(self):
      with DB(self, "NULL"):
          one = self.exec_naked("REDISQL.V1.EXEC", "NULL", "SELECT 1;  \n  ")
          self.assertEquals(one, [[1]])
  
  def test_whitespace_after_semicolon_then_query(self):
      with DB(self, "NULL"):
          one = self.exec_naked("REDISQL.V1.EXEC", "NULL", "SELECT 1;    SELECT 2;")
          self.assertEquals(one, [[2]])

  def test_newline_after_semicolon_then_query(self):
      with DB(self, "NULL"):
          one = self.exec_naked("REDISQL.V1.EXEC", "NULL", "SELECT 1;\nSELECT 2;")
          self.assertEquals(one, [[2]])

class TestWithNullChars(TestRediSQLWithExec):
    def test_with_null_at_the_end(self):
        with DB(self, 'A'):
            self.assertRaises(redis.ResponseError, self.exec_naked, "REDISQL.V1.EXEC", "A", "SELECT \0 1;")
        with DB(self, 'B'):
            self.assertRaises(redis.ResponseError, self.exec_naked, "REDISQL.V1.EXEC", "A", "SELECT 1;\0")

if __name__ == '__main__':
   unittest.main()

