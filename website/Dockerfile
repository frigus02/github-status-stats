ARG REGISTRY=frigus02
FROM $REGISTRY/github-status-stats-base as build

# Create tiny image
FROM scratch
ARG CARGO_MODE=release
COPY --from=build /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=build /src/target/x86_64-unknown-linux-musl/$CARGO_MODE/website /
COPY website/static /static
COPY website/templates /templates
ENV SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt
ENV SSL_CERT_DIR=/etc/ssl/certs
CMD ["/website"]
