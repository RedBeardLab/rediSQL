import unittest

import redis


class Table():
    def __init__(self, redis, name, parameters):
        self.redis = redis
        self.name = name
        self.parameters = parameters

    def __enter__(self):
        create_table = "CREATE TABLE {} {}".format(self.name, self.parameters)
        self.redis.execute_command("REDISQL.EXEC", create_table)

    def __exit__(self, type, value, traceback):
        drop_table = "DROP TABLE {}".format(self.name)
        self.redis.execute_command("REDISQL.EXEC", drop_table)



class TestRediSQL(unittest.TestCase):
    @classmethod
    def setUpClass(self):
        self.redis = redis.StrictRedis()
    
    def setUp(self):
        pass

    def exec_cmd(self, command):
        return self.redis.execute_command("REDISQL.EXEC", command)

    def test_ping(self):
        self.assertTrue(self.redis.ping())

    def test_create_table(self):
        done = self.exec_cmd("CREATE TABLE test (A INTEGER);")
        self.assertEquals(done, "DONE")
        done = self.exec_cmd("DROP TABLE test")
        self.assertEquals(done, "DONE")

    def test_insert(self):
        with Table(self.redis, "test", "(A INTEGER)"):
            done = self.exec_cmd("INSERT INTO test VALUES(2);")
            self.assertEquals(done, "DONE")

    def test_select(self):
        with Table(self.redis, "test", "(A INTEGER)"):
            done = self.exec_cmd("INSERT INTO test VALUES(2);")
            self.assertEquals(done, "DONE")
            
            result = self.exec_cmd("SELECT * from test")
            self.assertEquals(result, [[2]])
            
            self.exec_cmd("INSERT INTO test VALUES(3);")
            result = self.exec_cmd("SELECT * from test ORDER BY A")
            self.assertEquals(result, [[2], [3]])

            self.exec_cmd("INSERT INTO test VALUES(4);")
            result = self.exec_cmd("SELECT * FROM test ORDER BY A")
            self.assertEquals(result, [[2], [3], [4]])
    
    def test_single_remove(self):
        with Table(self.redis, "test", "(A INTEGER)"):
            self.exec_cmd("INSERT INTO test VALUES(2);")
            self.exec_cmd("INSERT INTO test VALUES(3);")
            self.exec_cmd("INSERT INTO test VALUES(4);")
            result = self.exec_cmd("SELECT * FROM test ORDER BY A")
            self.assertEquals(result, [[2], [3], [4]])
            self.exec_cmd("DELETE FROM test WHERE A = 3;")
            result = self.exec_cmd("SELECT * FROM test ORDER BY A")
            self.assertEquals(result, [[2], [4]])
 
    def test_big_select(self):
        elements = 50
        with Table(self.redis, "test", "(A INTERGER)"):
            pipe = self.redis.pipeline(transaction=False)
            for i in xrange(elements):
                pipe.execute_command("REDISQL.EXEC",
                        "INSERT INTO test VALUES({})".format(i))
                pipe.execute()
            result = self.exec_cmd("SELECT * FROM test ORDER BY A")
            self.assertEquals(result, [[x] for x in xrange(elements)])
 
    def test_multiple_row(self):
        with Table(self.redis, "test", "(A INTEGER, B REAL, C TEXT)"):
            self.exec_cmd("INSERT INTO test VALUES(1, 1.0, '1point1')")
            self.exec_cmd("INSERT INTO test VALUES(2, 2.0, '2point2')")
            self.exec_cmd("INSERT INTO test VALUES(3, 3.0, '3point3')")
            self.exec_cmd("INSERT INTO test VALUES(4, 4.0, '4point4')")
            self.exec_cmd("INSERT INTO test VALUES(5, 5.0, '5point5')")

            result = self.exec_cmd("SELECT A, B, C FROM test ORDER BY A")
            result = [[A, float(B), C] for [A, B, C] in result]
            self.assertEquals(result, 
                    [[1L, 1.0, "1point1"], [2L, 2.0, '2point2'],
                     [3L, 3.0, '3point3'], [4L, 4.0, '4point4'],
                     [5L, 5.0, '5point5']])
            
    def test_join(self):
        with Table(self.redis, "testA", "(A INTEGER, B INTEGER)"):
            with Table(self.redis, "testB", "(C INTEGER, D INTEGER)"):
                self.exec_cmd("INSERT INTO testA VALUES(1, 2)")
                self.exec_cmd("INSERT INTO testA VALUES(3, 4)")
                self.exec_cmd("INSERT INTO testB VALUES(1, 2)")
                self.exec_cmd("INSERT INTO testB VALUES(3, 4)")
                result = self.exec_cmd("SELECT A, B, C, D FROM testA, testB WHERE A = C ORDER BY A")
                self.assertEquals(result, [[1, 2, 1, 2], [3, 4, 3, 4]])
                

    def runTest(self):
        pass

if __name__ == '__main__':
    unittest.main()


