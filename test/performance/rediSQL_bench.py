import redis
import random
import string

from multiprocessing import Pool

def prepare_tables(r):
    tables = ["""CREATE TABLE triple_int(
                    aInt INT, 
                    bInt INT, 
                    cInt INT);""",
              """CREATE TABLE triple_float(
                    aF FLOAT, 
                    bF FLOAT, 
                    cF FLOAT);""",
              """CREATE TABLE triple_string(
                    aStr STRING,
                    bStr STRING,
                    cStr STRING);""",
              """CREATE TABLE relation(
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
    return "'" + ''.join(random.choice(string.ascii_uppercase + string.digits) for _ in range(random.randrange(200)))+"'"

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

def run_command_in_parallel(pool):

    def exec_q(query):
        client = redis_pool.get_connection()
        client.execute_command("REDISQL.EXEC", query)
        redis_pool.release(client)
    
    return exec_q

if __name__ == '__main__':
    print "Starting performance test"
    thread_pool = Pool(10)
    redis_pool = redis.Redis(
            connection_pool=redis.BlockingConnectionPool(
                max_connection = 20))
    print "Creating tables"
    client = redis_pool.get_connection()
    prepare_tables(client)
    redis_pool.release(client)
    print "Tables created"
    in_parallel = run_command_in_parallel(redis_pool)
    thread_pool.map(
            lambda _: in_parallel(generate_insert()),
            xrange(100))
    
    #for i in xrange(1000):
    #    command = random.choice([generate_join, generate_delete, generate_insert, generate_select])
    #    print i, command
    #    r.execute_command("REDISQL.EXEC", command())

