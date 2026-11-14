demo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
export PATH=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)/target/release:$PATH
export PS1='\[\e[34m\]\W\[\e[0m\] $ '
cd "$DEMO_WORK"
# The owner's log alias, which uses the [pretty] format of demo.gitconfig.
alias gl='git log --date relative --color=always --pretty=commit-format --stat'

# Prints the git config a recording uses, as a viewer would put it in their own: demo.gitconfig
# without comments, without the recording's name, email and editor, and without builtin-themes, which
# only the diff chapter's theme selection needs, the include of delta's
# themes, and, in a dark recording, dark.gitconfig's file-style. Arguments are lines printed after
# it, for what one chapter uses besides.
show_config() {
    local demo=$demo_dir file_style=
    if [[ $(git config --get delta.dark) == true ]]; then
        file_style=$(git config --file "$demo/dark.gitconfig" --get delta.my-theme.file-style)
    fi
    {
        awk -v file_style="$file_style" '
            /^\[/ { skip = ($0 ~ /^\[(user|core)\]/ || $0 ~ /^\[delta-tui "general\.diff-renderer"\]/) }
            skip || /^ *#/ { next }
            file_style != "" && /^ +file-style = / { print "    file-style = \"" file_style "\""; next }
            { print }
        ' "$demo/demo.gitconfig"
        printf '\n[include]\n    path = /path/to/delta/themes.gitconfig\n'
        if (($#)); then
            printf '\n'
            printf '%s\n' "$@"
        fi
    } | cat -s | command bat --language gitconfig --style plain --color always --paging never \
        --theme "$([[ -n $file_style ]] && echo "Monokai Extended" || echo GitHub)"
}
