CC=gcc
CFLAGS=-c -Wpedantic -Wall -fPIC
LD=ld

all: sqlite3.o
	$(CC) $(CFLAGS) -IRedisModulesSDK/ -Bstatic rediSQL.c -o rediSQL.o
	$(LD) -o rediSQL.so rediSQL.o sqlite3.o RedisModulesSDK/rmutil/librmutil.a -shared -lc

sqlite3.o:
	$(CC) -fPIC -c -o sqlite3.o sqlite3.c

