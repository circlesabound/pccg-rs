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
ARG GIT_COMMIT_HASH=unversioned
ENV GIT_COMMIT_HASH=$GIT_COMMIT_HASH
COPY --from=builder /usr/local/cargo/bin/pccg-rs /bin
COPY config.toml /bin
EXPOSE 8080
CMD ["pccg-rs", "/bin/config.toml"]
