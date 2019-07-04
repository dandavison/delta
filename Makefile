build:
	cargo build

test:
	bash -c "diff -u <(git log -p | cut -c 2-) <(git log -p | delta --width variable | ansifilter | cut -c 2-)"
