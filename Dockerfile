FROM rust:slim AS builder

RUN apt update
RUN apt install -y pkg-config libssl-dev

WORKDIR /usr/src/app
COPY ./ .
RUN cargo build --release
RUN cargo install --path server --verbose

FROM debian:buster-slim AS final
ARG GIT_COMMIT_HASH=unversioned
ENV GIT_COMMIT_HASH=$GIT_COMMIT_HASH

RUN apt update
RUN apt install -y pkg-config openssl ca-certificates

COPY --from=builder /usr/local/cargo/bin/pccg-rs /bin
COPY config.toml /bin
EXPOSE 7224
CMD ["pccg-rs", "/bin/config.toml"]
