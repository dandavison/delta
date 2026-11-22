# Build delta from source

You'll need to [install the rust tools](https://www.rust-lang.org/learn/get-started). Then clone the repository and build it:

```sh
git clone https://github.com/dandavison/delta.git
cd delta
cargo build --release
```

and use the executable found at `./target/release/delta`.

Alternatively, homebrew users can do

```sh
brew install --HEAD git-delta
```

to install the development version of delta with merged but unreleased changes.
