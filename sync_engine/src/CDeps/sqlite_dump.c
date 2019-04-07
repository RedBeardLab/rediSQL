#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <assert.h>
#include <unistd.h>
#include <pthread.h>
#include <sys/types.h>

#include "../../../redisql_lib/src/CDeps/SQLite/include/sqlite3.h"

#define utf8_printf fprintf

/*
** Render output like fprintf().  This should not be used on anything that
** includes string formatting (e.g. "%s").
*/
#if !defined(raw_printf)
#define raw_printf fprintf
#endif

/*
** Used to prevent warnings about unused parameters
*/
#define UNUSED_PARAMETER(x) (void)(x)

/*
** The following is the open SQLite database.  We make a pointer
** to this database a static variable so that it can be accessed
** by the SIGINT handler to interrupt database processing.
*/
static sqlite3* globalDb = 0;

/*
** A global char* and an SQL function to access its current value
** from within an SQL statement. This program used to use the
** sqlite_exec_printf() API to substitue a string into an SQL statement.
** The correct way to do this with sqlite3 is to use the bind API, but
** since the shell is built around the callback paradigm it would be a lot
** of work. Instead just use this hack, which is quite harmless.
*/
static const char* zShellStatic = 0;
static void
_shellstaticFunc(sqlite3_context* context, int argc, sqlite3_value** argv)
{
  assert(0 == argc);
  assert(zShellStatic);
  UNUSED_PARAMETER(argc);
  UNUSED_PARAMETER(argv);
  sqlite3_result_text(context, zShellStatic, -1, SQLITE_STATIC);
}

/*
** Compute a string length that is limited to what can be stored in
** lower 30 bits of a 32-bit signed integer.
*/
static int
_strlen30(const char* z)
{
  const char* z2 = z;
  while (*z2) {
    z2++;
  }
  return 0x3fffffff & (int)(z2 - z);
}

/* zIn is either a pointer to a NULL-terminated string in memory obtained
** from malloc(), or a NULL pointer. The string pointed to by zAppend is
** added to zIn, and the result returned in memory obtained from malloc().
** zIn, if it was not NULL, is freed.
**
** If the third argument, quote, is not '\0', then it is used as a
** quote character for zAppend.
*/
static char*
_appendText(char* zIn, char const* zAppend, char quote)
{
  int len;
  int i;
  int nAppend = _strlen30(zAppend);
  int nIn = (zIn ? _strlen30(zIn) : 0);

  len = nAppend + nIn + 1;
  if (quote) {
    len += 2;
    for (i = 0; i < nAppend; i++) {
      if (zAppend[i] == quote)
        len++;
    }
  }

  zIn = (char*)realloc(zIn, len);
  if (!zIn) {
    return 0;
  }

  if (quote) {
    char* zCsr = &zIn[nIn];
    *zCsr++ = quote;
    for (i = 0; i < nAppend; i++) {
      *zCsr++ = zAppend[i];
      if (zAppend[i] == quote)
        *zCsr++ = quote;
    }
    *zCsr++ = quote;
    *zCsr++ = '\0';
    assert((zCsr - zIn) == len);
  } else {
    memcpy(&zIn[nIn], zAppend, nAppend);
    zIn[len - 1] = '\0';
  }

  return zIn;
}

/*
** State information about the database connection is contained in an
** instance of the following structure.
*/
typedef struct ShellState ShellState;
struct ShellState
{
  sqlite3* db; /* The database */
  //  int echoOn;            /* True to echo input commands */
  // int autoExplain;       /* Automatically turn on .explain mode */
  //  int autoEQP;           /* Run EXPLAIN QUERY PLAN prior to seach SQL stmt
  //  */
  //  int statsOn;           /* True to display memory stats before each
  //  finalize */
  //  int scanstatsOn;       /* True to display scan stats before each finalize
  //  */
  //  int countChanges;      /* True to display change counts */
  //  int backslashOn;       /* Resolve C-style \x escapes in SQL input text */
  //  int outCount;          /* Revert to stdout when reaching zero */
  //  int cnt;               /* Number of records displayed so far */
  FILE* out; /* Write results here */
             //  FILE *traceOut;        /* Output for sqlite3_trace() */
  int nErr;  /* Number of errors seen */
             //  int mode;              /* An output mode setting */
  //  int cMode;             /* temporary output mode for the current query */
  //  int normalMode;        /* Output mode before ".explain on" */
  int writableSchema; /* True if PRAGMA writable_schema=ON */
  //  int showHeader;        /* True to show column names in List or Column mode
  //  */
  //  unsigned shellFlgs;    /* Various flags */
  //  char *zDestTable;      /* Name of destination table when MODE_Insert */
  //  char colSeparator[20]; /* Column separator character for several modes */
  //  char rowSeparator[20]; /* Row separator character for MODE_Ascii */
  //  int colWidth[100];     /* Requested width of each column when in column
  //  mode*/
  //  int actualWidth[100];  /* Actual width of each column */
  //  char nullValue[20];    /* The text to print when a NULL comes back from
  //                         ** the database */
  //  char outfile[FILENAME_MAX]; /* Filename for *out */
  const char* zDbFilename; /* name of the database file */
  //  char *zFreeOnClose;         /* Filename to free when closing */
  //  const char *zVfs;           /* Name of VFS to use */
  //  sqlite3_stmt *pStmt;   /* Current statement if any. */
  //  FILE *pLog;            /* Write log output here */
  //  int *aiIndent;         /* Array of indents used in MODE_Explain */
  //  int nIndent;           /* Size of array aiIndent[] */
  //  int iIndent;           /* Index of current op in aiIndent[] */
  //#if defined(SQLITE_ENABLE_SESSION)
  //  int nSession;             /* Number of active sessions */
  //  OpenSession aSession[4];  /* Array of sessions.  [0] is in focus. */
  //#endif
};

/*
** Implementation of the "readfile(X)" SQL function.  The entire content
** of the file named X is read and returned as a BLOB.  NULL is returned
** if the file does not exist or is unreadable.
*/
static void
_readfileFunc(sqlite3_context* context, int argc, sqlite3_value** argv)
{
  const char* zName;
  FILE* in;
  long nIn;
  void* pBuf;

  UNUSED_PARAMETER(argc);
  zName = (const char*)sqlite3_value_text(argv[0]);
  if (zName == 0)
    return;
  in = fopen(zName, "rb");
  if (in == 0)
    return;
  fseek(in, 0, SEEK_END);
  nIn = ftell(in);
  rewind(in);
  pBuf = sqlite3_malloc64(nIn);
  if (pBuf && 1 == fread(pBuf, nIn, 1, in)) {
    sqlite3_result_blob(context, pBuf, nIn, sqlite3_free);
  } else {
    sqlite3_free(pBuf);
  }
  fclose(in);
}

/*
** Implementation of the "writefile(X,Y)" SQL function.  The argument Y
** is written into file X.  The number of bytes written is returned.  Or
** NULL is returned if something goes wrong, such as being unable to open
** file X for writing.
*/
static void
_writefileFunc(sqlite3_context* context, int argc, sqlite3_value** argv)
{
  FILE* out;
  const char* z;
  sqlite3_int64 rc;
  const char* zFile;

  UNUSED_PARAMETER(argc);
  zFile = (const char*)sqlite3_value_text(argv[0]);
  if (zFile == 0)
    return;
  out = fopen(zFile, "wb");
  if (out == 0)
    return;
  z = (const char*)sqlite3_value_blob(argv[1]);
  if (z == 0) {
    rc = 0;
  } else {
    rc = fwrite(z, 1, sqlite3_value_bytes(argv[1]), out);
  }
  fclose(out);
  sqlite3_result_int64(context, rc);
}

/*
** Make sure the database is open.  If it is not, then open it.  If
** the database fails to open, print an error message and exit.
*/
static void
_open_db(ShellState* p, int keepAlive)
{
  if (p->db == 0) {
    sqlite3_initialize();
    sqlite3_open(p->zDbFilename, &p->db);
    globalDb = p->db;
    if (p->db && sqlite3_errcode(p->db) == SQLITE_OK) {
      sqlite3_create_function(p->db, "shellstatic", 0, SQLITE_UTF8, 0,
                              _shellstaticFunc, 0, 0);
    }
    if (p->db == 0 || SQLITE_OK != sqlite3_errcode(p->db)) {
      utf8_printf(stderr, "Error: unable to open database \"%s\": %s\n",
                  p->zDbFilename, sqlite3_errmsg(p->db));
      if (keepAlive)
        return;
      exit(1);
    }
#ifndef SQLITE_OMIT_LOAD_EXTENSION
    sqlite3_enable_load_extension(p->db, 1);
#endif
    sqlite3_create_function(p->db, "readfile", 1, SQLITE_UTF8, 0, _readfileFunc,
                            0, 0);
    sqlite3_create_function(p->db, "writefile", 2, SQLITE_UTF8, 0,
                            _writefileFunc, 0, 0);
  }
}

/*
** Execute a query statement that will generate SQL output.  Print
** the result columns, comma-separated, on a line and then add a
** semicolon terminator to the end of that line.
**
** If the number of columns is 1 and that column contains text "--"
** then write the semicolon on a separate line.  That way, if a
** "--" comment occurs at the end of the statement, the comment
** won't consume the semicolon terminator.
*/
static int
_run_table_dump_query(
  ShellState* p,        /* Query context */
  const char* zSelect,  /* SELECT statement to extract content */
  const char* zFirstRow /* Print before first row, if not NULL */
  )
{
  sqlite3_stmt* pSelect;
  int rc;
  int nResult;
  int i;
  const char* z;
  rc = sqlite3_prepare_v2(p->db, zSelect, -1, &pSelect, 0);
  if (rc != SQLITE_OK || !pSelect) {
    utf8_printf(p->out, "/**** ERROR: (%d) %s *****/\n", rc,
                sqlite3_errmsg(p->db));
    if ((rc & 0xff) != SQLITE_CORRUPT)
      p->nErr++;
    return rc;
  }
  rc = sqlite3_step(pSelect);
  nResult = sqlite3_column_count(pSelect);
  while (rc == SQLITE_ROW) {
    if (zFirstRow) {
      utf8_printf(p->out, "%s", zFirstRow);
      zFirstRow = 0;
    }
    z = (const char*)sqlite3_column_text(pSelect, 0);
    utf8_printf(p->out, "%s", z);
    for (i = 1; i < nResult; i++) {
      utf8_printf(p->out, ",%s", sqlite3_column_text(pSelect, i));
    }
    if (z == 0)
      z = "";
    while (z[0] && (z[0] != '-' || z[1] != '-'))
      z++;
    if (z[0]) {
      raw_printf(p->out, "\n;\n");
    } else {
      raw_printf(p->out, ";\n");
    }
    rc = sqlite3_step(pSelect);
  }
  rc = sqlite3_finalize(pSelect);
  if (rc != SQLITE_OK) {
    utf8_printf(p->out, "/**** ERROR: (%d) %s *****/\n", rc,
                sqlite3_errmsg(p->db));
    if ((rc & 0xff) != SQLITE_CORRUPT)
      p->nErr++;
  }
  return rc;
}

/*
** This is a different callback routine used for dumping the database.
** Each row received by this callback consists of a table name,
** the table type ("index" or "table") and SQL to create the table.
** This routine should print text sufficient to recreate the table.
*/
static int
_dump_callback(void* pArg, int nArg, char** azArg, char** azCol)
{
  int rc;
  const char* zTable;
  const char* zType;
  const char* zSql;
  const char* zPrepStmt = 0;
  ShellState* p = (ShellState*)pArg;

  UNUSED_PARAMETER(azCol);
  if (nArg != 3)
    return 1;
  zTable = azArg[0];
  zType = azArg[1];
  zSql = azArg[2];

  if (strcmp(zTable, "sqlite_sequence") == 0) {
    zPrepStmt = "DELETE FROM sqlite_sequence;\n";
  } else if (sqlite3_strglob("sqlite_stat?", zTable) == 0) {
    raw_printf(p->out, "ANALYZE sqlite_master;\n");
  } else if (strncmp(zTable, "sqlite_", 7) == 0) {
    return 0;
  } else if (strncmp(zSql, "CREATE VIRTUAL TABLE", 20) == 0) {
    char* zIns;
    if (!p->writableSchema) {
      raw_printf(p->out, "PRAGMA writable_schema=ON;\n");
      p->writableSchema = 1;
    }
    zIns = sqlite3_mprintf(
      "INSERT INTO sqlite_master(type,name,tbl_name,rootpage,sql)"
      "VALUES('table','%q','%q',0,'%q');",
      zTable, zTable, zSql);
    utf8_printf(p->out, "%s\n", zIns);
    sqlite3_free(zIns);
    return 0;
  } else {
    utf8_printf(p->out, "%s;\n", zSql);
  }

  if (strcmp(zType, "table") == 0) {
    sqlite3_stmt* pTableInfo = 0;
    char* zSelect = 0;
    char* zTableInfo = 0;
    char* zTmp = 0;
    int nRow = 0;

    zTableInfo = _appendText(zTableInfo, "PRAGMA table_info(", 0);
    zTableInfo = _appendText(zTableInfo, zTable, '"');
    zTableInfo = _appendText(zTableInfo, ");", 0);

    rc = sqlite3_prepare_v2(p->db, zTableInfo, -1, &pTableInfo, 0);
    free(zTableInfo);
    if (rc != SQLITE_OK || !pTableInfo) {
      return 1;
    }

    zSelect = _appendText(zSelect, "SELECT 'INSERT INTO ' || ", 0);
    /* Always quote the table name, even if it appears to be pure ascii,
    ** in case it is a keyword. Ex:  INSERT INTO "table" ... */
    zTmp = _appendText(zTmp, zTable, '"');
    if (zTmp) {
      zSelect = _appendText(zSelect, zTmp, '\'');
      free(zTmp);
    }
    zSelect = _appendText(zSelect, " || ' VALUES(' || ", 0);
    rc = sqlite3_step(pTableInfo);
    while (rc == SQLITE_ROW) {
      const char* zText = (const char*)sqlite3_column_text(pTableInfo, 1);
      zSelect = _appendText(zSelect, "quote(", 0);
      zSelect = _appendText(zSelect, zText, '"');
      rc = sqlite3_step(pTableInfo);
      if (rc == SQLITE_ROW) {
        zSelect = _appendText(zSelect, "), ", 0);
      } else {
        zSelect = _appendText(zSelect, ") ", 0);
      }
      nRow++;
    }
    rc = sqlite3_finalize(pTableInfo);
    if (rc != SQLITE_OK || nRow == 0) {
      free(zSelect);
      return 1;
    }
    zSelect = _appendText(zSelect, "|| ')' FROM  ", 0);
    zSelect = _appendText(zSelect, zTable, '"');

    rc = _run_table_dump_query(p, zSelect, zPrepStmt);
    if (rc == SQLITE_CORRUPT) {
      zSelect = _appendText(zSelect, " ORDER BY rowid DESC", 0);
      _run_table_dump_query(p, zSelect, 0);
    }
    free(zSelect);
  }
  return 0;
}

/*
** Run zQuery.  Use _dump_callback() as the callback routine so that
** the contents of the query are output as SQL statements.
**
** If we get a SQLITE_CORRUPT error, rerun the query after appending
** "ORDER BY rowid DESC" to the end.
*/
static int
_run_schema_dump_query(ShellState* p, const char* zQuery)
{
  int rc;
  char* zErr = 0;
  rc = sqlite3_exec(p->db, zQuery, _dump_callback, p, &zErr);
  if (rc == SQLITE_CORRUPT) {
    char* zQ2;
    int len = _strlen30(zQuery);
    raw_printf(p->out, "/****** CORRUPTION ERROR *******/\n");
    if (zErr) {
      utf8_printf(p->out, "/****** %s ******/\n", zErr);
      sqlite3_free(zErr);
      zErr = 0;
    }
    zQ2 = malloc(len + 100);
    if (zQ2 == 0)
      return rc;
    sqlite3_snprintf(len + 100, zQ2, "%s ORDER BY rowid DESC", zQuery);
    rc = sqlite3_exec(p->db, zQ2, _dump_callback, p, &zErr);
    if (rc) {
      utf8_printf(p->out, "/****** ERROR: %s ******/\n", zErr);
    } else {
      rc = SQLITE_CORRUPT;
    }
    sqlite3_free(zErr);
    free(zQ2);
  }
  return rc;
}

void
_write_to_state(ShellState* p)
{
  _open_db(p, 0);
  /* When playing back a "dump", the content might appear in an order
  ** which causes immediate foreign key constraints to be violated.
  ** So disable foreign-key constraint enforcement to prevent problems. */
  raw_printf(p->out, "PRAGMA foreign_keys=OFF;\n");
  raw_printf(p->out, "BEGIN TRANSACTION;\n");
  p->writableSchema = 0;
  sqlite3_exec(p->db, "SAVEPOINT dump; PRAGMA writable_schema=ON", 0, 0, 0);
  p->nErr = 0;

  _run_schema_dump_query(
    p, "SELECT name, type, sql FROM sqlite_master "
       "WHERE sql NOT NULL AND type=='table' AND name!='sqlite_sequence'");
  _run_schema_dump_query(p, "SELECT name, type, sql FROM sqlite_master "
                           "WHERE name=='sqlite_sequence'");
  _run_table_dump_query(
    p, "SELECT sql FROM sqlite_master "
       "WHERE sql NOT NULL AND type IN ('index','trigger','view')",
    0);

  if (p->writableSchema) {
    raw_printf(p->out, "PRAGMA writable_schema=OFF;\n");
    p->writableSchema = 0;
  }
  sqlite3_exec(p->db, "PRAGMA writable_schema=OFF;", 0, 0, 0);
  sqlite3_exec(p->db, "RELEASE dump;", 0, 0, 0);
  raw_printf(p->out, p->nErr ? "ROLLBACK; -- due to errors\n" : "COMMIT;\n");
  fclose(p->out);
  free(p);
}

int
start(sqlite3* db)
{
  ShellState* state = malloc(sizeof(ShellState));
  state->db = db;
  int pipefd[2];

  if (pipe(pipefd) == -1) {
    return -1;
  }

  int read_end = pipefd[0];
  int write_end = pipefd[1];
  state->out = fdopen(write_end, "wr");

  pthread_t iterator;
  pthread_create(&iterator, NULL, (void*)(void*)_write_to_state, state);

  return read_end;
}

ssize_t
read_from_pipe(int pipefd_read_end, void* buffer, ssize_t nbytes)
{
  return read(pipefd_read_end, buffer, nbytes);
}

int
close_read_pipe(int pipefd_read_end)
{
  return close(pipefd_read_end);
}
