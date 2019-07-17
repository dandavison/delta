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

EXECUTABLES_DIRECTORY := /tmp/delta
GIT_DIFF_INPUT := ./performance/data/input.gitdiff
HYPERFINE_OUTPUT := ./performance/data/hyperfine-output.json
HYPERFINE_PROCESSED_OUTPUT := ./performance/data/hyperfine-processed-output.json

benchmark:
	./performance/build-commits.sh master $(EXECUTABLES_DIRECTORY)
	git log -p 23c292d3f25c67082a2ba315a187268be1a9b0ab > $(GIT_DIFF_INPUT)
	./performance/benchmark-commits.py $(GIT_DIFF_INPUT) $(EXECUTABLES_DIRECTORY) $(HYPERFINE_OUTPUT)
	./performance/transform-benchmark-data.py < $(HYPERFINE_OUTPUT) > $(HYPERFINE_PROCESSED_OUTPUT)
