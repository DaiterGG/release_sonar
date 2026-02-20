FROM rust:alpine AS builder
WORKDIR /usr/src/rust_request
RUN apk add --no-cache openssl-dev protoc protobuf-dev
ENV OPENSSL_DIR="/usr"

COPY Cargo.toml Cargo.lock ./
COPY src/bin/fake_main.rs ./src/bin/fake_main.rs

RUN cargo install --bin deps-run --path .

COPY build.rs .
COPY src ./src
RUN cargo install --path .
RUN cargo clean

FROM alpine
COPY --from=builder /usr/local/cargo/bin/rust_request /usr/local/bin
EXPOSE 3000
ENTRYPOINT rust_request
