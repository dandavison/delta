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

shell-completion:
	for shell in bash fish zsh; do ./target/release/delta --generate-completion $$shell > etc/completion/completion.$$shell; done

release:
	@make -f etc/release.Makefile release

version:
	@grep version Cargo.toml | head -n1 | sed -E 's,.*version = "([^"]+)",\1,'

# Depend on the latest published delta-gitu
bump-gitu:
	v=$$(cargo search delta-gitu --limit 1 | sed -n 's/^delta-gitu = "\([^"]*\)".*/\1/p') && \
	perl -pi -e 's/^(delta-gitu = \{ version = ")[^"]+/$${1}'$$v'/' Cargo.toml && \
	cargo update -p delta-gitu && \
	$(MAKE) build && \
	git commit -m "delta-gitu v$$v" Cargo.toml Cargo.lock

BENCHMARK_INPUT_FILE = /tmp/delta-benchmark-input.gitdiff
BENCHMARK_COMMAND = git log -p 23c292d3f25c67082a2ba315a187268be1a9b0ab
benchmark: build
	$(BENCHMARK_COMMAND) > $(BENCHMARK_INPUT_FILE)
	hyperfine --warmup 10 --min-runs 20 \
		'target/release/delta --no-gitconfig < $(BENCHMARK_INPUT_FILE) > /dev/null'

# https://github.com/brendangregg/FlameGraph
flamegraph: build
	$(BENCHMARK_COMMAND) | target/release/delta > /dev/null &
	sample delta | stackcollapse-sample | flamegraph > etc/performance/flamegraph.svg

chronologer:
	chronologer etc/performance/chronologer.yaml

.PHONY: bump-gitu build format lint test unit-test end-to-end-test release shell-completion version benchmark flamegraph chronologer
