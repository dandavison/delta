build:
	cargo build --release

lint:
	cargo clippy

test: unit-test end-to-end-test

unit-test:
	cargo test

end-to-end-test: build
	bash -c "diff -u <(git log -p) <(git log -p | target/release/delta --color-only | ansifilter)"

release:
	@make -f release.Makefile release

version:
	@grep version Cargo.toml | head -n1 | sed -E 's,.*version = "([^"]+)",\1,'

hash:
	@version=$$(make version) && \
    printf "$$version-tar.gz %s\n" $$(curl -sL https://github.com/dandavison/delta/archive/$$version.tar.gz | sha256sum -) && \
	printf "delta-$$version-x86_64-apple-darwin.tar.gz %s\n" $$(curl -sL https://github.com/dandavison/delta/releases/download/$$version/delta-$$version-x86_64-apple-darwin.tar.gz | sha256sum -) && \
	printf "delta-$$version-x86_64-unknown-linux-musl.tar.gz %s\n" $$(curl -sL https://github.com/dandavison/delta/releases/download/$$version/delta-$$version-x86_64-unknown-linux-musl.tar.gz | sha256sum -)

chronologer:
	chronologer performance/chronologer.yaml

.PHONY: build lint test unit-test end-to-end-test release vesion hash chronologer
