CC=gcc
CFLAGS=-c -Wpedantic -fPIC
LD=ld

all:
	$(CC) $(CFLAGS) -IRedisModulesSDK/ -Bstatic rediSQL.c -o rediSQL.o
	$(LD) -o rediSQL.so rediSQL.o sqlite3.o RedisModulesSDK/rmutil/librmutil.a -shared -lc
sqlite:
	$(CC) -c -o sqlite3.o sqlite3.c

