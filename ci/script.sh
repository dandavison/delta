#!/usr/bin/env bash

set -ex

# Incorporate TARGET env var to the build and test process
cargo build --target "$TARGET" --verbose

# We cannot run arm executables on linux
if [[ $TARGET != arm-unknown-linux-gnueabihf ]] && [[ $TARGET != aarch64-unknown-linux-gnu ]]; then
    cargo test --target "$TARGET" --verbose
    cargo build --release
    ./tests/test_raw_output_matches_git_on_full_repo_history
    ./tests/test_deprecated_options > /dev/null

    cargo run --target "$TARGET" -- < /dev/null
fi
