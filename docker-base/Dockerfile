FROM rust:1.41

ARG CARGO_FLAGS=--release
ARG CARGO_MODE=release

# Build statically linked OpenSSL
RUN apt-get update && \
    apt-get install -y \
        musl-tools \
        && \
    rm -rf /var/lib/apt/lists/*

ENV OPENSSL_VERSION 1.0.2r
ENV CC musl-gcc
ENV PREFIX /usr/local
ENV PATH /usr/local/bin:$PATH
ENV PKG_CONFIG_PATH /usr/local/lib/pkgconfig
RUN curl -sL http://www.openssl.org/source/openssl-$OPENSSL_VERSION.tar.gz | tar xz && \
    cd openssl-$OPENSSL_VERSION && \
    ./Configure no-shared --prefix=$PREFIX --openssldir=$PREFIX/ssl no-zlib linux-x86_64 -fPIC && \
    make -j$(nproc) && \
    make install && \
    cd .. && \
    rm -rf openssl-$OPENSSL_VERSION
ENV SSL_CERT_FILE /etc/ssl/certs/ca-certificates.crt
ENV SSL_CERT_DIR /etc/ssl/certs
ENV OPENSSL_LIB_DIR $PREFIX/lib
ENV OPENSSL_INCLUDE_DIR $PREFIX/include
ENV OPENSSL_DIR $PREFIX
ENV OPENSSL_STATIC 1
ENV PKG_CONFIG_ALLOW_CROSS 1

# Configure target for static linking
RUN rustup target add x86_64-unknown-linux-musl && \
    mkdir .cargo && \
    echo "[build]\ntarget = \"x86_64-unknown-linux-musl\"\n" > .cargo/config && \
    rustup component add clippy

# Download and build dependencies
WORKDIR /src
RUN USER=root cargo new --bin importer && \
    USER=root cargo new --bin website && \
    USER=root cargo new --lib packages/ghss_github && \
    USER=root cargo new --lib packages/ghss_influxdb && \
    USER=root cargo new --lib packages/ghss_models && \
    USER=root cargo new --lib packages/ghss_tracing
COPY Cargo.toml Cargo.lock ./
COPY importer/Cargo.toml ./importer/
COPY website/Cargo.toml ./website/
COPY packages/ghss_github/Cargo.toml ./packages/ghss_github/
COPY packages/ghss_influxdb/Cargo.toml ./packages/ghss_influxdb/
COPY packages/ghss_models/Cargo.toml ./packages/ghss_models/
COPY packages/ghss_tracing/Cargo.toml ./packages/ghss_tracing/
RUN cargo clippy $CARGO_FLAGS && \
    cargo build $CARGO_FLAGS && \
    rm target/x86_64-unknown-linux-musl/$CARGO_MODE/deps/importer* && \
    rm target/x86_64-unknown-linux-musl/$CARGO_MODE/deps/website* && \
    rm target/x86_64-unknown-linux-musl/$CARGO_MODE/deps/libghss_github* && \
    rm target/x86_64-unknown-linux-musl/$CARGO_MODE/deps/libghss_influxdb* && \
    rm target/x86_64-unknown-linux-musl/$CARGO_MODE/deps/libghss_models* && \
    rm target/x86_64-unknown-linux-musl/$CARGO_MODE/deps/libghss_tracing*

# Build
COPY . .
RUN cargo clippy $CARGO_FLAGS --frozen -- -Dwarnings && \
    cargo build $CARGO_FLAGS
