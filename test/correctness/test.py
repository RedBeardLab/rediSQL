import unittest

import redis


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


class TestRediSQLWithExec(unittest.TestCase):
  @classmethod
  def setUpClass(self):
    self.redis = redis.StrictRedis()
   
  def exec_cmd(self, *command):
    return self.redis.execute_command("REDISQL.EXEC", *command)

  def create_db(self, key):
    return self.redis.execute_command("REDISQL.CREATE_DB", key)

  def delete_db(self, key):
    return self.redis.execute_command("DEL", key)

class TestRediSQLExec(TestRediSQLWithExec):
  def setUp(self):
    pass

  def test_ping(self):
    self.assertTrue(self.redis.ping())

  def test_create_table(self):
    with DB(self.redis, "A"):
      done = self.exec_cmd("A", "CREATE TABLE test1 (A INTEGER);")
      self.assertEquals(done, "DONE")
      done = self.exec_cmd("A", "DROP TABLE test1")
      self.assertEquals(done, "DONE")

  def test_insert(self):
    with DB(self.redis, "B"):
      with Table(self.redis, "test2", "(A INTEGER)", key = "B"):
        done = self.exec_cmd("B", "INSERT INTO test2 VALUES(2);")
        self.assertEquals(done, "DONE")

  def test_select(self):
    with DB(self.redis, "C"):
      with Table(self.redis, "test3", "(A INTEGER)", key = "C"):
        done = self.exec_cmd("C", "INSERT INTO test3 VALUES(2);")
        self.assertEquals(done, "DONE")
      
        result = self.exec_cmd("C", "SELECT * from test3")
        self.assertEquals(result, [[2]])
      
        self.exec_cmd("C", "INSERT INTO test3 VALUES(3);")
        result = self.exec_cmd("C", "SELECT * from test3 ORDER BY A")
        self.assertEquals(result, [[2], [3]])

        self.exec_cmd("C", "INSERT INTO test3 VALUES(4);")
        result = self.exec_cmd("C", "SELECT * FROM test3 ORDER BY A")
        self.assertEquals(result, [[2], [3], [4]])
  
  def test_single_remove(self):
    with DB(self.redis, "D"):
      with Table(self.redis, "test4", "(A INTEGER)", key = "D"):
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
    with DB(self.redis, "E"):
      with Table(self.redis, "test5", "(A INTERGER)", key = "E"):
        pipe = self.redis.pipeline(transaction=False)
        for i in xrange(elements):
          pipe.execute_command("REDISQL.EXEC", "E", 
              "INSERT INTO test5 VALUES({})".format(i))
          pipe.execute()
        result = self.exec_cmd("E", "SELECT * FROM test5 ORDER BY A")
        self.assertEquals(result, [[x] for x in xrange(elements)])
 
  def test_multiple_row(self):
    with DB(self.redis, "F"):
      with Table(self.redis, "test6", "(A INTEGER, B REAL, C TEXT)", key= "F"):
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
    with DB(self.redis, "G"):
      with Table(self.redis, "testA", "(A INTEGER, B INTEGER)", key = "G"):
        with Table(self.redis, "testB", "(C INTEGER, D INTEGER)", key = "G"):
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
    keys = self.redis.keys("A_REDISQL")
    self.assertEquals(["A_REDISQL"], keys)
    ok = self.delete_db("A_REDISQL")
    keys = self.redis.keys("A_REDISQL")
    self.assertEquals([], keys)

  def test_create_table_inside_key(self):
    with DB(self.redis, "A"):
      done = self.exec_cmd("A", "CREATE TABLE t1 (A INTEGER);")
      self.assertEquals(done, "DONE")
      done = self.exec_cmd("A", "DROP TABLE t1")
      self.assertEquals(done, "DONE")

  def test_insert_into_table(self):
    with DB(self.redis, "B"):
      with Table(self.redis, "t2", "(A INTEGER, B INTEGER)", key = "B"):
        done = self.exec_cmd("B", "INSERT INTO t2 VALUES(1,2)")
        self.assertEquals(done, "DONE")
        result = self.exec_cmd("B", "SELECT * FROM t2")
        self.assertEquals(result, [[1, 2]])


if __name__ == '__main__':
  unittest.main()


