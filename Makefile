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

hyperfine:
	./scripts/build-commits.sh master
	git log -p 23c292d3f25c67082a2ba315a187268be1a9b0ab > /tmp/input.gitdiff
	./scripts/hyperfine-commits.py /tmp/input.gitdiff /tmp/hyperfine.json /tmp/delta
