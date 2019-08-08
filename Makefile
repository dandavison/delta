lint:
	cargo clippy

build:
	@printf "____________________________________________________________________________________\n"
	cargo build

test:
	cargo test
	bash -c "diff -u <(git log -p | cut -c 2-) \
                     <(git log -p | delta --width variable \
                                          --commit-style plain \
                                          --file-style plain \
                                          --hunk-style plain \
                                  | ansifilter | cut -c 2-)"

hash:
	@version=$$(grep version Cargo.toml | head -n1 | sed -E 's,.*version = "([^"]+)",\1,') && \
	printf "delta-$$version-x86_64-apple-darwin.tar.gz %s\n" $$(curl -sL https://github.com/dandavison/delta/releases/download/$$version/delta-$$version-x86_64-apple-darwin.tar.gz | sha256sum -) && \
	printf "delta-$$version-x86_64-unknown-linux-musl.tar.gz %s\n" $$(curl -sL https://github.com/dandavison/delta/releases/download/$$version/delta-$$version-x86_64-unknown-linux-musl.tar.gz | sha256sum -)

chronologer:
	chronologer performance/chronologer.yaml
