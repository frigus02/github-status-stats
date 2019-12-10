#!/bin/sh
set -eu
cargo clippy --frozen -- -Dwarnings
