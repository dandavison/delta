#!/usr/bin/env bash

set -ex

# Incorporate TARGET env var to the build and test process
cargo build --target "$TARGET" --verbose

# We cannot run arm executables on linux
if [[ $TARGET != arm-unknown-linux-gnueabihf ]] && [[ $TARGET != aarch64-unknown-linux-gnu ]]; then
    cargo test --target "$TARGET" --verbose

    cargo run --target "$TARGET" -- < /dev/null
fi
