#include <stdlib.h>
#include "src/CDeps/SQLite/include/sqlite3.h"

int start(sqlite3* db);
ssize_t read_from_pipe(int pipefd_read_end, void* buffer, ssize_t nbytes);
int close_read_pipe(int pipefd_read_end);
