FROM rust:1.39 as build

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
    echo "[build]\ntarget = \"x86_64-unknown-linux-musl\"\n" > .cargo/config

# Download and build dependencies
RUN USER=root cargo new --bin github-status-stats
WORKDIR /github-status-stats
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release && \
    rm target/x86_64-unknown-linux-musl/release/deps/github_status_stats*

# Build application
COPY src ./src
RUN cargo build --release

# Create tiny image
FROM scratch
COPY --from=build /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=build /github-status-stats/target/x86_64-unknown-linux-musl/release/github-status-stats /
ENV SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt
ENV SSL_CERT_DIR=/etc/ssl/certs
CMD ["/github-status-stats"]
