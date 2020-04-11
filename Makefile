build:
	cargo build --release

lint:
	cargo clippy

test:
	cargo test
	bash -c "diff -u <(git log -p) \
                     <(git log -p | delta --width variable \
                                          --tabs 0 \
                                          --keep-plus-minus-markers \
                                          --commit-style plain \
                                          --file-style plain \
                                          --hunk-style plain \
                                  | ansifilter)"

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
