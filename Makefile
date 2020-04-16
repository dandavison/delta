build:
	cargo build --release

lint:
	cargo clippy

test: unit-test end-to-end-test

unit-test:
	cargo test

end-to-end-test: build
	bash -c "diff -u <(git log -p) <(git log -p | target/release/delta --color-only | sed 's/\x1b\[[0-9;]*m//g')"

release:
	@make -f release.Makefile release

version:
	@grep version Cargo.toml | head -n1 | perl -pe 's/\e\[[0-9;]*m//g'

hash:
	@version=$$(make version) && \
    printf "$$version-tar.gz %s\n" $$(curl -sL https://github.com/dandavison/delta/archive/$$version.tar.gz | sha256sum -) && \
	printf "delta-$$version-x86_64-apple-darwin.tar.gz %s\n" $$(curl -sL https://github.com/dandavison/delta/releases/download/$$version/delta-$$version-x86_64-apple-darwin.tar.gz | sha256sum -) && \
	printf "delta-$$version-x86_64-unknown-linux-musl.tar.gz %s\n" $$(curl -sL https://github.com/dandavison/delta/releases/download/$$version/delta-$$version-x86_64-unknown-linux-musl.tar.gz | sha256sum -)

BENCHMARK_INPUT_FILE = /tmp/delta-benchmark-input.gitdiff
benchmark: build
	git log -p 23c292d3f25c67082a2ba315a187268be1a9b0ab > $(BENCHMARK_INPUT_FILE)
	hyperfine 'target/release/delta < $(BENCHMARK_INPUT_FILE) > /dev/null'

chronologer:
	chronologer performance/chronologer.yaml

.PHONY: build lint test unit-test end-to-end-test release vesion hash benchmark chronologer
