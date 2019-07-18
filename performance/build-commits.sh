#!/bin/bash

set -e

build () {
    local commit=$1
    local i=$2
    local outfile=$outdir/$(printf "%04s" $i)-$commit
    [ -e $outfile ] && return
    echo $outfile
    git checkout --quiet $commit
    cargo build --release 2>&1 > /dev/null && \
        mv target/release/delta $outfile || \
        echo build failed: $commit 1>&2
}

original_head=$(git rev-parse --abbrev-ref HEAD)
outdir=$1
mkdir -p $outdir
i=1
git rev-list $@ | tac | while read commit; do build $commit $i; ((i+=1)); done
git checkout $original_head
