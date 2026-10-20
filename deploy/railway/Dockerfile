# iroh-relay pinned to the same 1.3.0 release as the `iroh` crate in Cargo.lock.
# The published crate's rust-version is 1.91.
FROM rust:1.91.0-bookworm AS build

ARG IROH_RELAY_VERSION=1.3.0
RUN cargo install "iroh-relay@${IROH_RELAY_VERSION}" --locked --features server

FROM debian:bookworm-slim

RUN useradd --system --home /nonexistent --shell /usr/sbin/nologin relay
COPY --from=build /usr/local/cargo/bin/iroh-relay /usr/local/bin/iroh-relay
COPY entrypoint.sh /usr/local/bin/oan-relay-entrypoint
RUN chmod 755 /usr/local/bin/iroh-relay /usr/local/bin/oan-relay-entrypoint

USER relay
ENV PORT=8080
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/oan-relay-entrypoint"]
