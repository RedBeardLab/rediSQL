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
    return self.client.execute_command("REDISQL.V2.EXEC", *command)

  def exec_query(self, database, query, *command):
    return self.client.execute_command("REDISQL.V2.EXEC", database, "QUERY", query, *command)

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

  def test_simple_select(self):
    with DB(self, "A"):
      result = self.exec_naked("REDISQL.V2.EXEC", "A", "QUERY", "SELECT 1;", "NO_HEADER")
      self.assertEqual(result, [[1]])

  def test_with_table(self):
    with DB(self, "B"):
      self.exec_naked("REDISQL.V2.EXEC", "B", "QUERY", "CREATE TABLE foo(bar string, baz int, ping float);")
      self.exec_naked("REDISQL.V2.EXEC", "B", "QUERY", "INSERT INTO foo VALUES('AAA', 3, 4.5),('BBB', 4, 5.5);")
      result = self.exec_naked("REDISQL.V2.EXEC", "B", "QUERY", "SELECT * FROM foo;", "NO_HEADER")
      self.assertEqual(result, [[b'AAA', 3, b'4.5'], [b'BBB', 4, b'5.5']])
      result = self.exec_naked("REDISQL.V2.EXEC", "B", "QUERY", "SELECT * FROM foo;")
      self.assertEqual(result, [
          [b'bar', b'baz', b'ping'],
          [b'TEXT', b'INT', b'FLOAT'],
          [b'AAA', 3, b'4.5'],
          [b'BBB', 4, b'5.5']])

  def test_create_table(self):
    with DB(self, "A"):
      done = self.exec_query("A", "CREATE TABLE test1 (A INTEGER);")
      self.assertEqual(done, [b'DONE', 0])
      done = self.exec_query("A", "DROP TABLE test1")
      self.assertEqual(done, [b'DONE', 0])

  def test_insert(self):
    with DB(self, "B"):
      with Table(self, "test2", "(A INTEGER)", key = "B"):
        done = self.exec_query("B", "INSERT INTO test2 VALUES(2);")
        self.assertEqual(done, [b'DONE', 1])

  def test_select(self):
    with DB(self, "C"):
      with Table(self, "test3", "(A INTEGER)", key = "C"):
        done = self.exec_query("C", "INSERT INTO test3 VALUES(2);")
        self.assertEqual(done, [b'DONE', 1])

        result = self.exec_query("C", "SELECT * from test3", "NO_HEADER")
        self.assertEqual(result, [[2]])
        result = self.exec_query("C", "SELECT * from test3")
        self.assertEqual(result, [[b'A'], [b'INT'], [2]])

        self.exec_query("C", "INSERT INTO test3 VALUES(3);")
        result = self.exec_query("C", "SELECT * from test3 ORDER BY A", "NO_HEADER")
        self.assertEqual(result, [[2], [3]])
        result = self.exec_query("C", "SELECT * from test3 ORDER BY A")
        self.assertEqual(result, [[b'A'], [b'INT'], [2], [3]])

        self.exec_query("C", "INSERT INTO test3 VALUES(4);")
        result = self.exec_query("C", "SELECT * FROM test3 ORDER BY A", "NO_HEADER")
        self.assertEqual(result, [[2], [3], [4]])
        result = self.exec_query("C", "SELECT * FROM test3 ORDER BY A")
        self.assertEqual(result, [[b'A'], [b'INT'], [2], [3], [4]])

  def test_single_remove(self):
    with DB(self, "D"):
      with Table(self, "test4", "(A INTEGER)", key = "D"):
        self.exec_query("D", "INSERT INTO test4 VALUES(2);")
        self.exec_query("D", "INSERT INTO test4 VALUES(3);")
        self.exec_query("D", "INSERT INTO test4 VALUES(4);")
        result = self.exec_query("D", "SELECT * FROM test4 ORDER BY A", "NO_HEADER")
        self.assertEqual(result, [[2], [3], [4]])
        result = self.exec_query("D", "SELECT * FROM test4 ORDER BY A")
        self.assertEqual(result, [[b'A'], [b'INT'], [2], [3], [4]])
        self.exec_query("D", "DELETE FROM test4 WHERE A = 3;")
        result = self.exec_query("D", "SELECT * FROM test4 ORDER BY A", "NO_HEADER")
        self.assertEqual(result, [[2], [4]])
        result = self.exec_query("D", "SELECT * FROM test4 ORDER BY A")
        self.assertEqual(result, [[b'A'], [b'INT'], [2], [4]])

  def test_big_select(self):
    elements = 50
    with DB(self, "E"):
      with Table(self, "test5", "(A INTERGER)", key = "E"):
        pipe = self.client.pipeline(transaction=False)
        for i in range(elements):
          pipe.execute_command("REDISQL.V1.EXEC", "E",
              "INSERT INTO test5 VALUES({})".format(i))
        pipe.execute()
        result = self.exec_query("E", "SELECT * FROM test5 ORDER BY A", "NO_HEADER")
        self.assertEqual(result, [[x] for x in range(elements)])
        result = self.exec_query("E", "SELECT * FROM test5 ORDER BY A")
        self.assertEqual(result,  [[b'A'], [b'INT']] + ( [[x] for x in range(elements)] ) )

  def test_multiple_row(self):
    with DB(self, "F"):
      with Table(self, "test6", "(A INTEGER, B REAL, C TEXT)", key= "F"):
        self.exec_query("F", "INSERT INTO test6 VALUES(1, 1.0, '1point1')")
        self.exec_query("F", "INSERT INTO test6 VALUES(2, 2.0, '2point2')")
        self.exec_query("F", "INSERT INTO test6 VALUES(3, 3.0, '3point3')")
        self.exec_query("F", "INSERT INTO test6 VALUES(4, 4.0, '4point4')")
        self.exec_query("F", "INSERT INTO test6 VALUES(5, 5.0, '5point5')")

        result = self.exec_query("F", "SELECT A, B, C FROM test6 ORDER BY A", "NO_HEADER")
        result = [[A, float(B), C] for [A, B, C] in result]
        self.assertEqual(result,
            [[1, 1.0, b'1point1'], [2, 2.0, b'2point2'],
             [3, 3.0, b'3point3'], [4, 4.0, b'4point4'],
             [5, 5.0, b'5point5']])

        result = self.exec_query("F", "SELECT A, B, C FROM test6 ORDER BY A")
        self.assertEqual(result,
            [[b'A', b'B', b'C'], [b'INT', b'FLOAT', b'TEXT'],
             [1, b'1', b'1point1'], [2, b'2', b'2point2'],
             [3, b'3', b'3point3'], [4, b'4', b'4point4'],
             [5, b'5', b'5point5']])


  def test_join(self):
    with DB(self, "G"):
      with Table(self, "testA", "(A INTEGER, B INTEGER)", key = "G"):
        with Table(self, "testB", "(C INTEGER, D INTEGER)", key = "G"):
          self.exec_query("G", "INSERT INTO testA VALUES(1, 2)")
          self.exec_query("G", "INSERT INTO testA VALUES(3, 4)")
          self.exec_query("G", "INSERT INTO testB VALUES(1, 2)")
          self.exec_query("G", "INSERT INTO testB VALUES(3, 4)")
          result = self.exec_query("G", "SELECT A, B, C, D FROM testA, testB WHERE A = C ORDER BY A", "NO_HEADER")
          self.assertEqual(result, [
              [1, 2, 1, 2],
              [3, 4, 3, 4]])
          result = self.exec_query("G", "SELECT A, B, C, D FROM testA, testB WHERE A = C ORDER BY A")
          self.assertEqual(result, [
              [b'A', b'B', b'C', b'D'],
              [b'INT', b'INT', b'INT', b'INT'],
              [1, 2, 1, 2],
              [3, 4, 3, 4]])

class TestSynchronous(TestRediSQLWithExec):
  def test_exec(self):
    with DB(self, "A"):
      done = self.exec_query("A", "CREATE TABLE test(a INT, b TEXT);", "NOW")
      self.assertEqual(done, [b'DONE', 0])
      done = self.exec_query("A", "INSERT INTO test VALUES(1, 'ciao'), (2, 'foo'), (100, 'baz');", "NOW")
      self.assertEqual(done, [b'DONE', 3])
      result = self.exec_query("A", "SELECT * FROM test ORDER BY a ASC", "NOW", "NO_HEADER")
      self.assertEqual(result, [[1, b'ciao'], [2, b'foo'], [100, b'baz']])

class TestRead(TestRediSQLWithExec):
  def test_read(self):
    with DB(self, "A"):
      done = self.exec_query("B", "CREATE TABLE t1(a INTEGER);")
      self.assertEqual(done, [b'DONE', 0])
      done = self.exec_query("B", "INSERT INTO t1 VALUES(4);")
      result = self.exec_naked("REDISQL.V2.EXEC", "A", "QUERY", "SELECT A FROM t1 LIMIT 1;", "READ_ONLY", "NO_HEADER")
      self.assertEqual(result, [[4]])
      result = self.exec_naked("REDISQL.V2.EXEC", "A", "QUERY", "SELECT A FROM t1 LIMIT 1;", "READ_ONLY")
      self.assertEqual(result, [[b'a'], [b'INT'], [4]])

  def test_not_insert(self):
    with DB(self, "B"):
      done = self.exec_query("B", "CREATE TABLE t1(a INTEGER);")
      self.assertEqual(done, [b'DONE', 0])
      with self.assertRaises(redis.exceptions.ResponseError):
        self.exec_naked("REDISQL.V2.EXEC", "B", "QUERY", "INSERT INTO t1 VALUES(5);", "READ_ONLY")

      done = self.exec_naked("REDISQL.V2.EXEC", "B", "QUERY", "CREATE TABLE test(a INT, b TEXT);")
      self.assertEqual(done, [b'DONE', 0])
      done = self.exec_naked("REDISQL.V2.EXEC", "B", "QUERY",
          "INSERT INTO test VALUES(1, 'ciao'), (2, 'foo'), (100, 'baz');")
      self.assertEqual(done, [b'DONE', 3])
      result = self.exec_naked("REDISQL.V2.EXEC", "B", "QUERY", "SELECT * FROM test ORDER BY a ASC",
          "READ_ONLY", "NO_HEADER")
      self.assertEqual(result, [[1, b'ciao'], [2, b'foo'], [100, b'baz']])
      result = self.exec_naked("REDISQL.V2.EXEC", "B", "QUERY", "SELECT * FROM test ORDER BY a ASC", "READ_ONLY")
      self.assertEqual(result, [[b'a', b'b'], [b'INT', b'TEXT'], [1, b'ciao'], [2, b'foo'], [100, b'baz']])


class TestStatements(TestRediSQLWithExec):
  def test_create_statement(self):
    with DB(self, "A"):
      with Table(self, "t1", "(A INTEGER)", key = "A"):
        ok = self.exec_naked("REDISQL.V2.STATEMENT", "A", "NEW", "insert", "insert into t1 values(?1);")
        self.assertEqual(ok, b'OK')
        done = self.exec_naked("REDISQL.V2.EXEC", "A", "STATEMENT", "insert", "ARGS", "3")
        self.assertEqual(done, [b'DONE', 1])
        done = self.exec_naked("REDISQL.V2.EXEC", "A", "STATEMENT", "insert", "ARGS", "4")
        self.assertEqual(done, [b'DONE', 1])
        result = self.exec_naked("REDISQL.V2.EXEC", "A", "QUERY", "SELECT * FROM t1 ORDER BY A;", "NO_HEADER")
        self.assertEqual(result, [[3], [4]])
        result = self.exec_naked("REDISQL.V2.EXEC", "A", "QUERY", "SELECT * FROM t1 ORDER BY A;")
        self.assertEqual(result, [[b'A'], [b'INT'], [3], [4]])

  def test_create_statement_with_update(self):
    with DB(self, "B"):
      with Table(self, "t1", "(A INTEGER)", key = "B"):
        ok = self.exec_naked("REDISQL.V2.STATEMENT", "B", "UPDATE", "insert", "insert into t1 values(?1);", "CAN_CREATE")
        self.assertEqual(ok, b'OK')
        done = self.exec_naked("REDISQL.V2.EXEC", "B", "STATEMENT", "insert", "ARGS", "3")
        self.assertEqual(done, [b'DONE', 1])
        done = self.exec_naked("REDISQL.V2.EXEC", "B", "STATEMENT", "insert", "ARGS", "4")
        self.assertEqual(done, [b'DONE', 1])
        result = self.exec_naked("REDISQL.V2.EXEC", "B", "QUERY", "SELECT * FROM t1 ORDER BY A;", "NO_HEADER")
        self.assertEqual(result, [[3], [4]])
        result = self.exec_naked("REDISQL.V2.EXEC", "B", "QUERY", "SELECT * FROM t1 ORDER BY A;")
        self.assertEqual(result, [[b'A'], [b'INT'], [3], [4]])

  def test_multi_statement_single_bind(self):
    with DB(self, "A"):
      with Table(self, "t1", "(A INTEGER)", key = "A"):
        ok = self.exec_naked("REDISQL.V2.STATEMENT", "A", "NEW", "insert", "insert into t1 values(?1); insert into t1 values(?1 + 1);")
        self.assertEqual(ok, b'OK')
        done = self.exec_naked("REDISQL.V2.EXEC", "A", "STATEMENT", "insert", "ARGS", "3")
        self.assertEqual(done, [b'DONE', 2])
        done = self.exec_naked("REDISQL.V2.EXEC", "A", "STATEMENT", "insert", "ARGS", "5")
        self.assertEqual(done, [b'DONE', 2])
        result = self.exec_naked("REDISQL.V2.EXEC", "A", "QUERY", "SELECT * FROM t1 ORDER BY A;", "NO_HEADER")
        self.assertEqual(result, [[3], [4], [5], [6]])
        result = self.exec_naked("REDISQL.V2.EXEC", "A", "QUERY", "SELECT * FROM t1 ORDER BY A;")
        self.assertEqual(result, [[b'A'], [b'INT'], [3], [4], [5], [6]])

  def test_multi_statement_multi_table_single_bind(self):
    with DB(self, "A"):
      with Table(self, "t1", "(A INTEGER)", key = "A"):
        with Table(self, "t2", "(A INTEGER)", key = "A"):
          ok = self.exec_naked("REDISQL.V2.STATEMENT", "A", "NEW", "insert", "insert into t1 values(?1); insert into t2 values(?1 - 1);")
          self.assertEqual(ok, b'OK')
          done = self.exec_naked("REDISQL.V2.EXEC", "A", "STATEMENT", "insert", "ARGS", "3")
          self.assertEqual(done, [b'DONE', 2])
          done = self.exec_naked("REDISQL.V2.EXEC", "A", "STATEMENT", "insert", "ARGS", "5")
          self.assertEqual(done, [b'DONE', 2])

          result = self.exec_naked("REDISQL.V2.EXEC", "A", "QUERY", "SELECT * FROM t1 ORDER BY A;", "NO_HEADER")
          self.assertEqual(result, [[3], [5]])

          result = self.exec_naked("REDISQL.V2.EXEC", "A", "QUERY", "SELECT * FROM t2 ORDER BY A;", "NO_HEADER")
          self.assertEqual(result, [[2], [4]])

  def test_multi_statement_different_bindings(self):
    with DB(self, "A"):
      with Table(self, "t1", "(A INTEGER)", key = "A"):
        ok = self.exec_naked("REDISQL.V2.STATEMENT", "A", "NEW", "insert", "insert into t1 values(?1); insert into t1 values(?2 + 1); select * from t1;")
        self.assertEqual(ok, b'OK')
        result = self.exec_naked("REDISQL.V2.EXEC", "A", "STATEMENT", "insert", "ARGS", "3", "8")
        self.assertEqual(result, [[3], [9]])


  def test_update_statement(self):
    with DB(self, "A"):
      with Table(self, "t1", "(A INTEGER)", key = "A"):
        ok = self.exec_naked("REDISQL.V2.STATEMENT", "A", "NEW", "insert", "insert into t1 values(?1);")
        self.assertEqual(ok, b'OK')
        done = self.exec_naked("REDISQL.V2.EXEC", "A", "STATEMENT", "insert", "ARGS", "3")
        self.assertEqual(done, [b'DONE', 1])
        ok = self.exec_naked("REDISQL.V2.STATEMENT", "A", "UPDATE", "insert", "insert into t1 values(?1 + 10001);")
        self.assertEqual(ok, b'OK')
        done = self.exec_naked("REDISQL.V2.EXEC", "A", "STATEMENT", "insert", "ARGS", "4")
        self.assertEqual(done, [b'DONE', 1])
        result = self.exec_naked("REDISQL.V2.EXEC", "A", "QUERY", "SELECT * FROM t1 ORDER BY A;", "NO_HEADER")
        self.assertEqual(result, [[3], [10005]])

  def test_update_statement_with_create(self):
    with DB(self, "A"):
      with Table(self, "t1", "(A INTEGER)", key = "A"):
        ok = self.exec_naked("REDISQL.V2.STATEMENT", "A", "NEW", "insert", "insert into t1 values(?1);")
        self.assertEqual(ok, b'OK')
        done = self.exec_naked("REDISQL.V2.EXEC", "A", "STATEMENT", "insert", "ARGS", "3")
        self.assertEqual(done, [b'DONE', 1])
        ok = self.exec_naked("REDISQL.V2.STATEMENT", "A", "NEW", "insert", "insert into t1 values(?1 + 10001);", "CAN_UPDATE")
        self.assertEqual(ok, b'OK')
        done = self.exec_naked("REDISQL.V2.EXEC", "A", "STATEMENT", "insert", "ARGS", "4")
        self.assertEqual(done, [b'DONE', 1])
        result = self.exec_naked("REDISQL.V2.EXEC", "A", "QUERY", "SELECT * FROM t1 ORDER BY A;", "NO_HEADER")
        self.assertEqual(result, [[3], [10005]])

  def test_rdb_persistency(self):
    with DB(self, "A"):
      with Table(self, "t1", "(A INTEGER)", key = "A"):
        ok = self.exec_naked("REDISQL.V2.STATEMENT", "A", "NEW", "insert", "insert into t1 values(?1);")
        self.assertEqual(ok, b'OK')
        done = self.exec_naked("REDISQL.V2.EXEC", "A", "STATEMENT", "insert", "ARGS", "3")
        self.assertEqual(done, [b'DONE', 1])
        for _ in self.retry_with_reload():
          pass
        time.sleep(0.5)

        done = self.exec_naked("REDISQL.V2.EXEC", "A", "STATEMENT", "insert", "ARGS", "4")
        self.assertEqual(done, [b'DONE', 1])
        result = self.exec_naked("REDISQL.V2.EXEC", "A", "QUERY", "SELECT * FROM t1 ORDER BY A;", "NO_HEADER")
        self.assertEqual(result, [[3], [4]])

  def test_rdb_persistency_no_statements(self):
    with DB(self, "A"):
      with Table(self, "t1", "(A INTEGER)", key = "A"):

        done = self.exec_naked("REDISQL.V2.EXEC", "A", "QUERY", "INSERT INTO t1 VALUES(5)")
        self.assertEqual(done, [b'DONE', 1])

        for _ in self.retry_with_reload():
          pass
        time.sleep(0.5)

        done = self.exec_naked("REDISQL.V2.EXEC", "A", "QUERY", "INSERT INTO t1 VALUES(6)")
        self.assertEqual(done, [b'DONE', 1])

        result = self.exec_naked("REDISQL.V2.EXEC", "A", "QUERY", "SELECT * FROM t1 ORDER BY A;", "NO_HEADER")
        self.assertEqual(result, [[5], [6]])

  def test_rdb_persistency_multiple_statements(self):
    with DB(self, "A"):
      with Table(self, "t1", "(A INTEGER)", key = "A"):
        ok = self.exec_naked("REDISQL.V2.STATEMENT", "A", "NEW", "insert", "insert into t1 values(?1);")
        self.assertEqual(ok, b'OK')
        ok = self.exec_naked("REDISQL.V2.STATEMENT", "A", "NEW", "insert più cento", "insert into t1 values(?1 + 100);")
        self.assertEqual(ok, b'OK')

        done = self.exec_naked("REDISQL.V2.EXEC", "A", "STATEMENT", "insert", "ARGS", "3")
        self.assertEqual(done, [b'DONE', 1])
        done = self.exec_naked("REDISQL.V2.EXEC", "A", "STATEMENT", "insert più cento", "ARGS", "3")
        self.assertEqual(done, [b'DONE', 1])

        for _ in self.retry_with_reload():
          pass
        time.sleep(0.5)

        done = self.exec_naked("REDISQL.V2.EXEC", "A", "statement", "insert", "Args", "4")
        self.assertEqual(done, [b'DONE', 1])
        done = self.exec_naked("REDISQL.V2.EXEC", "A", "STateMent", "insert più cento", "aRGs", "4")
        self.assertEqual(done, [b'DONE', 1])

        result = self.exec_naked("REDISQL.V2.EXEC", "A", "QUeRY", "SELECT * FROM t1 ORDER BY A;", "no_HEader")
        self.assertEqual(result, [[3], [4], [103], [104]])



class TestStatementsSynchronous(TestRediSQLWithExec):
  def test_create_statement_synchronous(self):
    with DB(self, "A"):
      with Table(self, "t1", "(A INTEGER)", key = "A"):
        ok = self.exec_naked("REDISQL.V2.STATEMENT", "A", "NEW", "insert", "insert into t1 values(?1);", "NOW")
        self.assertEqual(ok, b'OK')
        done = self.exec_naked("REDISQL.V2.EXEC", "A", "STATEMENT", "insert", "NOW", "ARGS", "3")
        self.assertEqual(done, [b'DONE', 1])
        done = self.exec_naked("REDISQL.V2.EXEC", "A", "STATEMENT", "insert", "NOW", "ARGS", "4")
        self.assertEqual(done, [b'DONE', 1])
        result = self.exec_naked("REDISQL.V2.EXEC", "A", "QUERY", "SELECT * FROM t1 ORDER BY A;", "NO_HEADER", "NOW")
        self.assertEqual(result, [[3], [4]])
        result = self.exec_naked("REDISQL.V2.EXEC", "A", "QUERY", "SELECT * FROM t1 ORDER BY A;", "NOW")
        self.assertEqual(result, [[b'A'], [b'INT'], [3], [4]])

  def test_create_statement_with_update_synchronous(self):
    with DB(self, "B"):
      with Table(self, "t1", "(A INTEGER)", key = "B"):
        ok = self.exec_naked("REDISQL.V2.STATEMENT", "B", "UPDATE", "insert", "insert into t1 values(?1);", "CAN_CREATE", "NOW")
        self.assertEqual(ok, b'OK')
        done = self.exec_naked("REDISQL.V2.EXEC", "B", "STATEMENT", "insert", "NOW", "ARGS", "3")
        self.assertEqual(done, [b'DONE', 1])
        done = self.exec_naked("REDISQL.V2.EXEC", "B", "STATEMENT", "insert", "NOW", "ARGS", "4")
        self.assertEqual(done, [b'DONE', 1])
        result = self.exec_naked("REDISQL.V2.EXEC", "B", "QUERY", "SELECT * FROM t1 ORDER BY A;", "NO_HEADER", "NOW")
        self.assertEqual(result, [[3], [4]])
        result = self.exec_naked("REDISQL.V2.EXEC", "B", "QUERY", "SELECT * FROM t1 ORDER BY A;", "NOW")
        self.assertEqual(result, [[b'A'], [b'INT'], [3], [4]])

  def test_multi_statement_single_bind_synchronous(self):
    with DB(self, "C"):
      with Table(self, "t1", "(A INTEGER)", key = "C"):
        ok = self.exec_naked("REDISQL.V2.STATEMENT", "C", "NEW", "insert", "insert into t1 values(?1); insert into t1 values(?1 + 1);", "NOW")
        self.assertEqual(ok, b'OK')
        done = self.exec_naked("REDISQL.V2.EXEC", "C", "STATEMENT", "insert", "NOW", "ARGS", "3")
        self.assertEqual(done, [b'DONE', 2])
        done = self.exec_naked("REDISQL.V2.EXEC", "C", "STATEMENT", "insert", "NOW", "ARGS", "5")
        self.assertEqual(done, [b'DONE', 2])
        result = self.exec_naked("REDISQL.V2.EXEC", "C", "QUERY", "SELECT * FROM t1 ORDER BY A;", "NO_HEADER", "NOW")
        self.assertEqual(result, [[3], [4], [5], [6]])
        result = self.exec_naked("REDISQL.V2.EXEC", "C", "QUERY", "SELECT * FROM t1 ORDER BY A;", "NOW")
        self.assertEqual(result, [[b'A'], [b'INT'], [3], [4], [5], [6]])

  def test_multi_statement_multi_table_single_bind_synchronous(self):
    with DB(self, "D"):
      with Table(self, "t1", "(A INTEGER)", key = "D"):
        with Table(self, "t2", "(A INTEGER)", key = "D"):
          ok = self.exec_naked("REDISQL.V2.STATEMENT", "D", "NEW", "insert", "insert into t1 values(?1); insert into t2 values(?1 - 1);", "NOW")
          self.assertEqual(ok, b'OK')
          done = self.exec_naked("REDISQL.V2.EXEC", "D", "STATEMENT", "insert", "NOW", "ARGS", "3")
          self.assertEqual(done, [b'DONE', 2])
          done = self.exec_naked("REDISQL.V2.EXEC", "D", "STATEMENT", "insert", "NOW", "ARGS", "5")
          self.assertEqual(done, [b'DONE', 2])

          result = self.exec_naked("REDISQL.V2.EXEC", "D", "QUERY", "SELECT * FROM t1 ORDER BY A;", "NO_HEADER", "NOW")
          self.assertEqual(result, [[3], [5]])

          result = self.exec_naked("REDISQL.V2.EXEC", "D", "QUERY", "SELECT * FROM t2 ORDER BY A;", "NO_HEADER", "NOW")
          self.assertEqual(result, [[2], [4]])

  def test_multi_statement_different_bindings_synchronous(self):
    with DB(self, "E"):
      with Table(self, "t1", "(A INTEGER)", key = "E"):
        ok = self.exec_naked("REDISQL.V2.STATEMENT", "E", "NEW", "insert", "insert into t1 values(?1); insert into t1 values(?2 + 1); select * from t1;")
        self.assertEqual(ok, b'OK')
        result = self.exec_naked("REDISQL.V2.EXEC", "E", "STATEMENT", "insert", "NOW", "NO_HEADER", "ARGS", "3", "8")
        self.assertEqual(result, [[3], [9]])


  def test_update_statement_synchronous(self):
    with DB(self, "F"):
      with Table(self, "t1", "(A INTEGER)", key = "F"):
        ok = self.exec_naked("REDISQL.V2.STATEMENT", "F", "NEW", "insert", "insert into t1 values(?1);", "NOW")
        self.assertEqual(ok, b'OK')
        done = self.exec_naked("REDISQL.V2.EXEC", "F", "STATEMENT", "insert", "NOW", "ARGS", "3")
        self.assertEqual(done, [b'DONE', 1])
        ok = self.exec_naked("REDISQL.V2.STATEMENT", "F", "UPDATE", "insert", "insert into t1 values(?1 + 10001);", "NOW")
        self.assertEqual(ok, b'OK')
        done = self.exec_naked("REDISQL.V2.EXEC", "F", "STATEMENT", "insert", "NOW", "ARGS", "4")
        self.assertEqual(done, [b'DONE', 1])
        result = self.exec_naked("REDISQL.V2.EXEC", "F", "QUERY", "SELECT * FROM t1 ORDER BY A;", "NOW", "NO_HEADER")
        self.assertEqual(result, [[3], [10005]])

  def test_update_statement_with_create_synchronous(self):
    with DB(self, "G"):
      with Table(self, "t1", "(A INTEGER)", key = "G"):
        ok = self.exec_naked("REDISQL.V2.STATEMENT", "G", "NEW", "insert", "insert into t1 values(?1);", "NOW")
        self.assertEqual(ok, b'OK')
        done = self.exec_naked("REDISQL.V2.EXEC", "G", "STATEMENT", "insert", "NOW", "ARGS", "3")
        self.assertEqual(done, [b'DONE', 1])
        ok = self.exec_naked("REDISQL.V2.STATEMENT", "G", "NEW", "insert", "insert into t1 values(?1 + 10001);", "CAN_UPDATE", "NOW")
        self.assertEqual(ok, b'OK')
        done = self.exec_naked("REDISQL.V2.EXEC", "G", "STATEMENT", "insert", "NOW", "ARGS", "4")
        self.assertEqual(done, [b'DONE', 1])
        result = self.exec_naked("REDISQL.V2.EXEC", "G", "QUERY", "SELECT * FROM t1 ORDER BY A;", "NOW", "NO_HEADER")
        self.assertEqual(result, [[3], [10005]])

  def test_rdb_persistency_synchronous(self):
    with DB(self, "H"):
      with Table(self, "t1", "(A INTEGER)", key = "H"):
        ok = self.exec_naked("REDISQL.V2.STATEMENT", "H", "NEW", "insert", "insert into t1 values(?1);", "NOW")
        self.assertEqual(ok, b'OK')
        done = self.exec_naked("REDISQL.V2.EXEC", "H", "STATEMENT", "insert", "NOW", "ARGS", "3")
        self.assertEqual(done, [b'DONE', 1])
        for _ in self.retry_with_reload():
          pass
        time.sleep(0.5)

        done = self.exec_naked("REDISQL.V2.EXEC", "H", "STATEMENT", "insert", "NOW", "ARGS", "4")
        self.assertEqual(done, [b'DONE', 1])
        result = self.exec_naked("REDISQL.V2.EXEC", "H", "QUERY", "SELECT * FROM t1 ORDER BY A;", "NO_HEADER", "NOW")
        self.assertEqual(result, [[3], [4]])

  def test_rdb_persistency_no_statements_synchronous(self):
    with DB(self, "I"):
      with Table(self, "t1", "(A INTEGER)", key = "I"):

        done = self.exec_naked("REDISQL.V2.EXEC", "I", "QUERY", "INSERT INTO t1 VALUES(5)", "NOW")
        self.assertEqual(done, [b'DONE', 1])

        for _ in self.retry_with_reload():
          pass
        time.sleep(0.5)

        done = self.exec_naked("REDISQL.V2.EXEC", "I", "QUERY", "INSERT INTO t1 VALUES(6)", "NOW")
        self.assertEqual(done, [b'DONE', 1])

        result = self.exec_naked("REDISQL.V2.EXEC", "I", "QUERY", "SELECT * FROM t1 ORDER BY A;", "NOW", "NO_HEADER")
        self.assertEqual(result, [[5], [6]])

  def test_rdb_persistency_multiple_statements_synchronous(self):
    with DB(self, "L"):
      with Table(self, "t1", "(A INTEGER)", key = "L"):
        ok = self.exec_naked("REDISQL.V2.STATEMENT", "L", "NEW", "insert", "insert into t1 values(?1);", "NOW")
        self.assertEqual(ok, b'OK')
        ok = self.exec_naked("REDISQL.V2.STATEMENT", "L", "NEW", "insert più cento", "insert into t1 values(?1 + 100);", "NOW")
        self.assertEqual(ok, b'OK')

        done = self.exec_naked("REDISQL.V2.EXEC", "L", "STATEMENT", "insert", "NOW", "ARGS", "3")
        self.assertEqual(done, [b'DONE', 1])
        done = self.exec_naked("REDISQL.V2.EXEC", "L", "STATEMENT", "insert più cento", "NOW", "ARGS", "3")
        self.assertEqual(done, [b'DONE', 1])

        for _ in self.retry_with_reload():
          pass
        time.sleep(0.5)

        done = self.exec_naked("REDISQL.V2.EXEC", "L", "statement", "insert", "NOW", "Args", "4")
        self.assertEqual(done, [b'DONE', 1])
        done = self.exec_naked("REDISQL.V2.EXEC", "L", "STateMent", "insert più cento", "NOW", "aRGs", "4")
        self.assertEqual(done, [b'DONE', 1])

        result = self.exec_naked("REDISQL.V2.EXEC", "L", "QUeRY", "SELECT * FROM t1 ORDER BY A;", "no_HEader", "NoW")
        self.assertEqual(result, [[3], [4], [103], [104]])



if __name__ == '__main__':
   unittest.main()
