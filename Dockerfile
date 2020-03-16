FROM rust:slim AS builder
WORKDIR /usr/src/app
COPY Cargo.lock .
COPY Cargo.toml .
RUN mkdir .cargo
RUN cargo vendor > .cargo/config

COPY ./src src
RUN cargo build --release
RUN cargo install --path . --verbose

FROM debian:buster-slim AS final
COPY --from=builder /usr/local/cargo/bin/pccg-rs /bin
COPY config.toml /bin
EXPOSE 8080
CMD ["pccg-rs", "config.toml"]
