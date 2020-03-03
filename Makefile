build:
	cargo build --release

lint:
	cargo clippy

test:
	cargo test
	bash -c "diff -u <(git log -p | cut -c 2-) \
                     <(git log -p | delta --width variable \
                                          --tabs 0 \
                                          --commit-style plain \
                                          --file-style plain \
                                          --hunk-style plain \
                                  | ansifilter | cut -c 2-)"

release:
	cargo publish

brew:
	cd $$(brew --repo homebrew/homebrew-core) && \
	brew uninstall --force git-delta && \
	brew install --build-from-source git-delta && \
	brew test git-delta && \
	brew uninstall --force git-delta && \
	brew install git-delta && \
	brew audit --strict git-delta

hash:
	@version=$$(grep version Cargo.toml | head -n1 | sed -E 's,.*version = "([^"]+)",\1,') && \
    printf "$$version-tar.gz %s\n" $$(curl -sL https://github.com/dandavison/delta/archive/$$version.tar.gz | sha256sum -) && \
	printf "delta-$$version-x86_64-apple-darwin.tar.gz %s\n" $$(curl -sL https://github.com/dandavison/delta/releases/download/$$version/delta-$$version-x86_64-apple-darwin.tar.gz | sha256sum -) && \
	printf "delta-$$version-x86_64-unknown-linux-musl.tar.gz %s\n" $$(curl -sL https://github.com/dandavison/delta/releases/download/$$version/delta-$$version-x86_64-unknown-linux-musl.tar.gz | sha256sum -)

chronologer:
	chronologer performance/chronologer.yaml
