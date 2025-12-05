# Notes on contributing to delta

First off, thank you for considering contributing to delta.

If your contribution is not straightforward, please first discuss the change you
wish to make by creating a new issue before making the change.

## The codebase

Take a look at [ARCHITECTURE.md](./ARCHITECTURE.md).

## Set up

This is no different than other Rust projects.

```shell
git clone https://github.com/dandavison/delta/
cd delta
cargo build --release
```

The executable is then at `./target/release/delta`.

## Useful Commands

- Run all tests:

  ```shell
  make test
  ```

- Run Clippy:

  ```shell
  cargo clippy
  ```

- Check to see if there are code formatting issues

  ```shell
  cargo fmt -- --check
  ```

- Format the code in the project

  ```shell
  cargo fmt
  ```

- Debug build

A "debug" build can be built using `cargo build` and
`./target/debug/delta`. This is faster to compile, but has much worse
performance than the release build.

## Benchmarking

Here's an example using `hyperfine` to measure execution time distributions, and `script -q` to trick git into using its pager despite input not being a tty.

```
$ hyperfine --warmup 10 'GIT_PAGER=less script -q /dev/null command git log -n 1' 'GIT_PAGER=/opt/homebrew/bin/delta script -q /dev/null command git log -n 1' 'GIT_PAGER=/Users/dan/src/delta/target/release/delta script -q /dev/null  command git log -n 1'
Benchmark 1: GIT_PAGER=less script -q /dev/null command git log -n 1
  Time (mean ± σ):      19.1 ms ±   3.5 ms    [User: 6.5 ms, System: 12.3 ms]
  Range (min … max):    13.2 ms …  29.0 ms    65 runs

Benchmark 2: GIT_PAGER=/opt/homebrew/bin/delta script -q /dev/null command git log -n 1
  Time (mean ± σ):      1.132 s ±  0.011 s    [User: 0.061 s, System: 0.046 s]
  Range (min … max):    1.105 s …  1.143 s    10 runs

Benchmark 3: GIT_PAGER=/Users/dan/src/delta/target/release/delta script -q /dev/null  command git log -n 1
  Time (mean ± σ):      1.132 s ±  0.012 s    [User: 0.046 s, System: 0.047 s]
  Range (min … max):    1.107 s …  1.143 s    10 runs

Summary
  GIT_PAGER=less script -q /dev/null command git log -n 1 ran
   59.28 ± 11.02 times faster than GIT_PAGER=/opt/homebrew/bin/delta script -q /dev/null command git log -n 1
   59.28 ± 11.03 times faster than GIT_PAGER=/Users/dan/src/delta/target/release/delta script -q /dev/null  command git log -n 1
```