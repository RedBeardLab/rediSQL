FROM redis:latest

# RUN apk update; apk add libgcc_s.so.1

RUN apt-get update; apt-get install -y ca-certificates

COPY releases/RediSQL_v1.1.1_9b110f_x86_64-unknown-linux-gnu_release.so /usr/local/lib/redisql.so

EXPOSE 6379

CMD ["redis-server", "--loadmodule", "/usr/local/lib/redisql.so"]
