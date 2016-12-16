import redis
import random
import string

from multiprocessing.pool import ThreadPool
from multiprocessing import Value
import itertools
import threading
from time import time
import collections

def prepare_tables(r):
    tables = ["""CREATE TABLE IF NOT EXISTS triple_int(
                    aInt INT, 
                    bInt INT, 
                    cInt INT);""",
              """CREATE TABLE IF NOT EXISTS triple_float(
                    aF FLOAT, 
                    bF FLOAT, 
                    cF FLOAT);""",
              """CREATE TABLE IF NOT EXISTS triple_string(
                    aStr STRING,
                    bStr STRING,
                    cStr STRING);""",
              """CREATE TABLE IF NOT EXISTS relation(
                    aRel INT,
                    bRel FLOAT,
                    cRel STRING);"""
            ]
    for t in tables:
        r.execute_command("REDISQL.EXEC", t)

def generate_integers():
    return str(random.randrange(0, 10000))

def generate_floats():
    return "{0:.2f}".format(random.uniform(0, 10000), 2)

def generate_string():
    return '"' + ''.join(random.choice(string.ascii_uppercase + string.digits) for _ in range(random.randrange(200)))+'"'

def generate_select():
    options = [
            ["triple_int", 
                lambda: random.choice(["aInt", "bInt", "cInt"]), 
                lambda: random.choice(["=", "<", ">", "<=", ">="]),
                lambda: generate_integers()],
            ["triple_float", 
                lambda: random.choice(["aF", "bF", "cF"]), 
                lambda: random.choice(["=", "<", ">", "<=", ">="]),
                lambda: generate_floats()],
            ["triple_string", 
                lambda: random.choice(["aStr", "bStr", "cStr"]),
                lambda: random.choice(["<", ">", "="]),
                lambda: generate_string()],
            ["relation",
                lambda: random.choice(["aRel", "bRel", "cRel"]),
                lambda: random.choice(["<", ">", "="]),
                lambda: generate_integers()]
        ]
    q = random.choice(options)
    return "SELECT * FROM {} WHERE {} {} {};".format(q[0], q[1](), q[2](), q[3]())

def generate_insert():
    options = [
            ["triple_int", lambda: ", ".join([generate_integers() for _ in range(3)])],
            ["triple_float", lambda: ", ".join([generate_floats() for _ in range(3)])],
            ["triple_string", lambda: ", ".join([generate_string() for _ in range(3)])],
            ["relation", lambda: ", ".join([generate_integers(), generate_floats(), generate_string()])]]
    q = random.choice(options)
    return "INSERT INTO {} VALUES ({});".format(q[0], q[1]())

def generate_delete():
    options = [
            ["triple_int", 
                lambda: random.choice(["aInt", "bInt", "cInt"]), 
                lambda: random.choice(["=", "<", ">", "<=", ">="]),
                lambda: generate_integers()],
            ["triple_float", 
                lambda: random.choice(["aF", "bF", "cF"]), 
                lambda: random.choice(["=", "<", ">", "<=", ">="]),
                lambda: generate_floats()],
            ["triple_string", 
                lambda: random.choice(["aStr", "bStr", "cStr"]),
                lambda: random.choice(["<", ">", "="]),
                lambda: generate_string()],
            ["relation",
                lambda: random.choice(["aRel", "bRel", "cRel"]),
                lambda: random.choice(["<", ">", "="]),
                lambda: generate_integers()]
        ]
    q = random.choice(options)
    return "DELETE FROM {} WHERE {} {} {};".format(q[0], q[1](), q[2](), q[3]())

def generate_join():
    options = [
            lambda: "SELECT * FROM triple_int, relation WHERE {} {} aRel;".format(
                random.choice(["aInt", "bInt", "cInt"]), 
                random.choice(["=", ">", "<"])),
            lambda: "SELECT * FROM triple_int, relation, triple_float WHERE {} {} aRel AND {} {} bRel;".format(
                random.choice(["aInt", "bInt", "cInt"]), 
                random.choice(["=", "<", ">"]), 
                random.choice(["aF", "bF", "cF"]), 
                random.choice(["=", "<", ">"])),
            lambda: "SELECT * FROM triple_int, relation, triple_string WHERE {} {} aRel AND {} {} cRel;".format(
                random.choice(["aInt", "bInt", "cInt"]), 
                random.choice(["=", "<", ">"]), 
                random.choice(["aStr", "bStr", "cStr"]), 
                random.choice(["=", "<", ">"]))]
    return random.choice(options)()

def generate_queries():
    for _ in xrange(1000):
        pass

   

if __name__ == '__main__':
    print "Starting performance test"
    
    redis_pool = redis.StrictRedis(connection_pool = 
            redis.BlockingConnectionPool(
                max_connections = 15,
                timeout = 10))
    
    total = Value('i', 0)
    errors = Value('i', 0)

    stats = collections.deque()
    stats_lock = threading.Lock()

    def f(i):
        command = generate_insert()
        # print i, command, "\n"
        try:
            redis_pool.execute_command("REDISQL.EXEC", command)
        except Exception as e:
            print i, command
            print str(e)

    def g(i):
        # command = random.choice([generate_join, generate_insert, generate_select])()
        command = generate_insert()
        #print i, command, "\n"
        try:
            with total.get_lock():
                total.value += 1
            start = time()
            redis_pool.execute_command("REDISQL.EXEC", command)
            total_time = time() - start
            with stats_lock:
                stats.append((total_time, command))
        except Exception as e:
            with errors.get_lock():
                errors.value += 1
            print i, command
            print str(e)

    thread_pool = ThreadPool(10)
      
    print "Creating tables"
    prepare_tables(redis_pool)
    
    print "Tables created"
    
    
    thread_pool.map(f, xrange(100))
    
    start = time()

    thread_pool.map(g, xrange(1000))

    end = time()

    stats = list(stats)
    stats.sort(key = lambda x: x[0])

    print "Total: ", total.value
    print "Error: ", errors.value
    print "Total time sec: ", end - start

    for time, query in stats[-50:]:
        print time, query
