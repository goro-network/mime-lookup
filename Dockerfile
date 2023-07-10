## Stage - Build

FROM rust:1.70.0 as builder

WORKDIR /builder
COPY . .

RUN RUSTFLAGS="-C target-cpu=generic -C link-args=-s" \
    cargo build --release

## Stage - Final

FROM ubuntu:22.04

LABEL org.opencontainers.image.authors "goro-network Developers <https://github.com/goro-network>"
LABEL org.opencontainers.image.source "https://github.com/goro-network/mime-lookup"
LABEL org.opencontainers.image.description "MIME hash lookup service for goro Subnetwork (storage-node)"

EXPOSE 8383

COPY --from=builder /builder/target/release/mime-lookup /usr/bin/mime-lookup

ENTRYPOINT [ "mime-lookup" ]
