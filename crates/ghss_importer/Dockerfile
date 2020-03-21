ARG REGISTRY=frigus02
FROM $REGISTRY/ghss-base as build

# Create tiny image
FROM scratch
ARG CARGO_MODE=release
COPY --from=build /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=build /src/target/x86_64-unknown-linux-musl/$CARGO_MODE/ghss_importer /
ENV SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt
ENV SSL_CERT_DIR=/etc/ssl/certs
CMD ["/ghss_importer"]
