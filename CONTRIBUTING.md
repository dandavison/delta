# Contribution guidelines

First off, thank you for considering contributing to delta.

If your contribution is not straightforward, please first discuss the change you
wish to make by creating a new issue before making the change.

## Developing

### Set up

This is no different than other Rust projects.

```shell
git clone https://github.com/dandavison/delta/
cd delta
cargo build
```

### Useful Commands

- Build and run release version:

  ```shell
  cargo build --release && cargo run --release
  ```

- Run Clippy:

  ```shell
  cargo clippy
  ```

- Run all tests:

  ```shell
  cargo test
  ```

- Check to see if there are code formatting issues

  ```shell
  cargo fmt -- --check
  ```

- Format the code in the project

  ```shell
  cargo fmt
  ```
