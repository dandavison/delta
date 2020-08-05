#!/bin/bash
d1=$(mktemp -d)
d2=$(mktemp -d)
mkdir $d1 $d2
git show HEAD~10:./src/delta.rs > $d1/a.rs
git show HEAD:./src/delta.rs > $d2/a.rs
touch $d2/b.rs
git show HEAD~10:./src/paint.rs > $d1/c.rs
git show HEAD:./src/paint.rs > $d2/c.rs
diff -u $d1 $d2
