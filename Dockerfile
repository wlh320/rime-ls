FROM alpine:edge AS builder

# 1. install dependencies
RUN apk add --no-cache rust cargo musl-dev clang16-libclang librime-dev librime-plugins

# 2. build rime-ls
WORKDIR /src
COPY . .
RUN cargo build --release

# 3. build rime-ls image
FROM alpine:edge
# NOTE: 2025.2 Thanks to Alpine maintainer Celeste,
#              we don't need to build rime plugins now.
RUN apk add --no-cache librime librime-plugins
WORKDIR /app
COPY --from=builder /src/target/release/rime_ls /app
EXPOSE 9257
ENTRYPOINT ["./rime_ls"]
