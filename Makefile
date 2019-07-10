build:
	@printf "____________________________________________________________________________________\n"
	cargo build

test:
	cargo test
	bash -c "diff -u <(git log -p | cut -c 2-) <(git log -p | delta --width variable --no-structural-changes | ansifilter | cut -c 2-)"
