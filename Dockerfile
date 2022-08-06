FROM alpine

RUN apk add --no-cache rust cargo pkgconfig

COPY mn /app
WORKDIR /app
RUN apk add --no-cache openssl-dev
RUN cargo build --release
