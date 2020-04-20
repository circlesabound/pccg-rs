FROM rust:slim

RUN sudo apt update
RUN sudo apt install pkg-config libssl-dev

WORKDIR /usr/src/app
COPY Cargo.lock .
COPY Cargo.toml .
RUN mkdir .cargo
RUN cargo vendor > .cargo/config

COPY ./src src
RUN cargo test
