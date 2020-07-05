FROM rust:1.44.1

ARG CARGO_FLAGS=--release
ARG CARGO_MODE=release

# Build statically linked OpenSSL
RUN apt-get update && \
    apt-get install -y \
        musl-tools \
        && \
    rm -rf /var/lib/apt/lists/*

ENV OPENSSL_VERSION 1.1.1f
ENV CC musl-gcc
ENV PREFIX /usr/local
ENV PATH /usr/local/bin:$PATH
ENV PKG_CONFIG_PATH /usr/local/lib/pkgconfig
# TODO: Disabling secure memory here to make it easier to build OpenSSL for MUSL.
#       Would be good to find a proper solution. See issue:
#       https://github.com/openssl/openssl/issues/7207
RUN curl -sL http://www.openssl.org/source/openssl-$OPENSSL_VERSION.tar.gz | tar xz && \
    cd openssl-$OPENSSL_VERSION && \
    ./Configure --prefix=$PREFIX --openssldir=$PREFIX/ssl linux-x86_64 no-shared no-engine no-zlib -DOPENSSL_NO_SECURE_MEMORY && \
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
    rustup component add clippy rustfmt

# Download and build dependencies
WORKDIR /src
RUN USER=root cargo new --lib crates/ghss_github && \
    USER=root cargo new --bin crates/ghss_importer && \
    USER=root cargo new --bin crates/ghss_store && \
    USER=root cargo new --lib crates/ghss_store_client && \
    USER=root cargo new --lib crates/ghss_tracing && \
    USER=root cargo new --bin crates/ghss_website
COPY Cargo.toml Cargo.lock ./
COPY crates/ghss_github/Cargo.toml ./crates/ghss_github/
COPY crates/ghss_importer/Cargo.toml ./crates/ghss_importer/
COPY crates/ghss_store/Cargo.toml ./crates/ghss_store/
COPY crates/ghss_store_client/Cargo.toml ./crates/ghss_store_client/
COPY crates/ghss_tracing/Cargo.toml ./crates/ghss_tracing/
COPY crates/ghss_website/Cargo.toml ./crates/ghss_website/
RUN cargo clippy $CARGO_FLAGS && \
    cargo build $CARGO_FLAGS && \
    rm target/x86_64-unknown-linux-musl/$CARGO_MODE/deps/ghss_importer* && \
    rm target/x86_64-unknown-linux-musl/$CARGO_MODE/deps/ghss_store* && \
    rm target/x86_64-unknown-linux-musl/$CARGO_MODE/deps/ghss_website* && \
    rm target/x86_64-unknown-linux-musl/$CARGO_MODE/deps/libghss_github* && \
    rm target/x86_64-unknown-linux-musl/$CARGO_MODE/deps/libghss_store_client* && \
    rm target/x86_64-unknown-linux-musl/$CARGO_MODE/deps/libghss_tracing*

# Build
COPY . .
RUN cargo clippy $CARGO_FLAGS -- -Dwarnings && \
    cargo build $CARGO_FLAGS
