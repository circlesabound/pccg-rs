FROM rust:slim

RUN apt update
RUN apt install -y pkg-config libssl-dev

WORKDIR /usr/src/app
COPY ./ .
RUN cargo test
