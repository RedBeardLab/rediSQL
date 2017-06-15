FROM scorpil/rust:stable

RUN apt update \
  && apt install -y libclang-dev make

ENV REDIS_VERSION=4.0-rc3
ENV REDIS_DOWNLOAD_URL=https://github.com/antirez/redis/archive/$REDIS_VERSION.tar.gz

RUN curl -fsOSL $REDIS_DOWNLOAD_URL \
  && tar xzf $REDIS_VERSION.tar.gz \
  && cd redis-$REDIS_VERSION \
  && make
