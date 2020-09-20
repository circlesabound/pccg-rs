FROM ekidd/rust-musl-builder AS builder

ADD --chown=rust:rust . ./
RUN cargo build --release
RUN cargo install --path server --verbose

FROM alpine:latest AS final
ARG GIT_COMMIT_HASH=unversioned
ENV GIT_COMMIT_HASH=$GIT_COMMIT_HASH

RUN apk add --no-cache openssl ca-certificates

COPY --from=builder \
    /home/rust/.cargo/bin/pccg-rs \
    /usr/local/bin
COPY config.toml /usr/local/bin
EXPOSE 7224
CMD ["pccg-rs", "/usr/local/bin/config.toml"]
