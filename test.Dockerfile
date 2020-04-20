FROM rust:slim

RUN apt update
RUN apt install -y pkg-config libssl-dev

WORKDIR /usr/src/app
COPY Cargo.lock .
COPY Cargo.toml .
RUN mkdir .cargo
RUN cargo vendor > .cargo/config

COPY ./src src
RUN cargo test
