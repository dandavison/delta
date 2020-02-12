# Test script written by @gibfahn in https://github.com/dandavison/delta/issues/93
dir=/tmp/tst
rm -fr $dir && mkdir -p $dir && cd $dir
git init
echo hello > bar
git add bar && git commit -m "added text file bar"
echo -n -e \\x48\\x00\\x49\\x00 > foo
git add foo
GIT_PAGER="delta --theme=TwoDark" git diff --staged
GIT_PAGER=less git diff --staged
git commit -m "added binary file foo"
echo -n -e \\x49\\x00\\x48\\x00 > foo
git add foo
git commit -m "changed binary file foo"
GIT_PAGER="delta --theme=TwoDark" git log -p
GIT_PAGER=less git log -p
