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

chronologer:
	chronologer performance/chronologer.yaml
