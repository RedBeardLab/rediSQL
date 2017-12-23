FROM redis:latest

# RUN apk update; apk add libgcc_s.so.1

COPY /target/release/libredis_sql.so /usr/local/lib

EXPOSE 6379

CMD ["redis-server", "--loadmodule", "/usr/local/lib/libredis_sql.so"]
