FROM redis:latest

# RUN apk update; apk add libgcc_s.so.1

COPY releases/1.0.1/redisql_v1.0.1_x86_64.so /usr/local/lib/libredis_sql.so

EXPOSE 6379

CMD ["redis-server", "--loadmodule", "/usr/local/lib/libredis_sql.so"]
