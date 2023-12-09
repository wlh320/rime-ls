FROM rust as builder

RUN apt update && apt install -y clang librime-dev

WORKDIR /app

COPY . .

RUN cargo build --release

FROM debian:12

RUN apt update && apt install -y librime-dev

WORKDIR /app

COPY --from=builder /app/target/release/rime_ls /app

EXPOSE 9257

CMD ["/app/rime_ls","--listen","0.0.0.0:9257"]
