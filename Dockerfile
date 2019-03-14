FROM redis:latest

# RUN apk update; apk add libgcc_s.so.1

COPY releases/0.9.1/redisql_v0.9.1_x86_64.so /usr/local/lib

EXPOSE 6379

CMD ["redis-server", "--loadmodule", "/usr/local/lib/libredis_sql.so"]
