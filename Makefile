build:
	cargo build

test:
	bash -c "diff -u <(git show | cut -c 2-) <(git show | delta --width variable | ansifilter | cut -c 2-)"
