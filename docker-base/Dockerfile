FROM rust:1.40

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
    rustup component add clippy-preview

# Download and build dependencies
WORKDIR /src
RUN USER=root cargo new --bin importer && \
    USER=root cargo new --bin website && \
    USER=root cargo new --lib packages/github_client && \
    USER=root cargo new --lib packages/influxdb_client
COPY Cargo.toml Cargo.lock ./
COPY importer/Cargo.toml ./importer/
COPY website/Cargo.toml ./website/
COPY packages/github_client/Cargo.toml ./packages/github_client/
COPY packages/influxdb_client/Cargo.toml ./packages/influxdb_client/
RUN cargo build --release && \
    rm target/x86_64-unknown-linux-musl/release/deps/importer* && \
    rm target/x86_64-unknown-linux-musl/release/deps/website* && \
    rm target/x86_64-unknown-linux-musl/release/deps/libgithub_client* && \
    rm target/x86_64-unknown-linux-musl/release/deps/libinfluxdb_client*

# Build
COPY . .
RUN cargo clippy --workspace --release --frozen -- -Dwarnings && \
    cargo build --release
