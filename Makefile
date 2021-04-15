build:
	cargo build --release

format:
	git ls-files | grep '\.rs$$' | xargs -P 0 rustfmt

lint:
	cargo clippy

test: unit-test end-to-end-test

unit-test:
	cargo test

end-to-end-test: build
	./tests/test_raw_output_matches_git_on_full_repo_history
	./tests/test_deprecated_options > /dev/null
	./tests/test_navigate_less_history_file

release:
	@make -f release.Makefile release

version:
	@grep version Cargo.toml | head -n1 | sed -E 's,.*version = "([^"]+)",\1,'

hash:
	@version=$$(make version) && \
	printf "$$version-tar.gz %s\n" $$(curl -sL https://github.com/dandavison/delta/archive/$$version.tar.gz | sha256sum -) && \
	printf "delta-$$version-x86_64-apple-darwin.tar.gz %s\n" $$(curl -sL https://github.com/dandavison/delta/releases/download/$$version/delta-$$version-x86_64-apple-darwin.tar.gz | sha256sum -) && \
	printf "delta-$$version-x86_64-unknown-linux-musl.tar.gz %s\n" $$(curl -sL https://github.com/dandavison/delta/releases/download/$$version/delta-$$version-x86_64-unknown-linux-musl.tar.gz | sha256sum -)

BENCHMARK_INPUT_FILE = /tmp/delta-benchmark-input.gitdiff
BENCHMARK_COMMAND = git log -p 23c292d3f25c67082a2ba315a187268be1a9b0ab
benchmark: build
	$(BENCHMARK_COMMAND) > $(BENCHMARK_INPUT_FILE)
	hyperfine 'target/release/delta --no-gitconfig < $(BENCHMARK_INPUT_FILE) > /dev/null'

# https://github.com/brendangregg/FlameGraph
flamegraph: build
	$(BENCHMARK_COMMAND) | target/release/delta > /dev/null &
	sample delta | stackcollapse-sample | flamegraph > etc/performance/flamegraph.svg

chronologer:
	chronologer etc/performance/chronologer.yaml

.PHONY: build format lint test unit-test end-to-end-test release version hash benchmark flamegraph chronologer
