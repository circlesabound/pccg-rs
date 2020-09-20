FROM rust:slim

RUN apt update
RUN apt install -y pkg-config libssl-dev clang

WORKDIR /usr/src/app
COPY ./ .
RUN cargo test
