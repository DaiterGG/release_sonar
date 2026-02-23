FROM rust:alpine AS builder
WORKDIR /usr/src/rust_request
RUN apk add --no-cache openssl-dev protoc protobuf-dev
ENV OPENSSL_DIR="/usr"

COPY Cargo.toml Cargo.lock ./
COPY src/bin/fake_main.rs ./src/bin/fake_main.rs

RUN cargo install --no-default-features --bin deps-run --path .

COPY build.rs .
COPY src ./src
RUN cargo install --no-default-features --path .
RUN cargo clean

FROM alpine
COPY --from=builder /usr/local/cargo/bin/rust_request /usr/local/bin
EXPOSE 3000
ENV SQS_URL="https://sqs.us-east-1.amazonaws.com/748887400437/InvokeQueue.fifo"
ENTRYPOINT rust_request
