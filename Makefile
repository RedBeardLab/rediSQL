CC=gcc
CFLAGS=-c -Wpedantic -Wall -fPIC
LD=ld


all: rediSQL.o projection.o sqlite3.o
	$(LD) -o rediSQL.so rediSQL.o sqlite3.o projection.o RedisModulesSDK/rmutil/librmutil.a -shared -lc

rediSQL.o: rediSQL.c 
	$(CC) $(CFLAGS) -IRedisModulesSDK/ -Bstatic rediSQL.c -o rediSQL.o

projection.o: projection.c 
	$(CC) $(CFLAGS) -IRedisModulesSDK/ -o projection.o projection.c

sqlite3.o: sqlite3.c
	$(CC) $(CFLAGS) -o sqlite3.o sqlite3.c

clean:
	rm -f sqlite3.o projection.o rediSQL.o
