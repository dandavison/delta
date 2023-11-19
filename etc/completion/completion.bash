_delta() {
    local i cur prev opts cmd
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    cmd=""
    opts=""

    for i in ${COMP_WORDS[@]}
    do
        case "${cmd},${i}" in
            ",$1")
                cmd="delta"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        delta)
            opts="-n -s -w -h -V --blame-code-style --blame-format --blame-palette --blame-separator-format --blame-separator-style --blame-timestamp-format --blame-timestamp-output-format --color-only --config --commit-decoration-style --commit-regex --commit-style --dark --default-language --diff-highlight --diff-so-fancy --diff-stat-align-width --features --file-added-label --file-copied-label --file-decoration-style --file-modified-label --file-removed-label --file-renamed-label --file-style --file-transformation --generate-completion --grep-context-line-style --grep-file-style --grep-header-decoration-style --grep-header-file-style --grep-line-number-style --grep-output-type --grep-match-line-style --grep-match-word-style --grep-separator-symbol --hunk-header-decoration-style --hunk-header-file-style --hunk-header-line-number-style --hunk-header-style --hunk-label --hyperlinks --hyperlinks-commit-link-format --hyperlinks-file-link-format --inline-hint-style --inspect-raw-lines --keep-plus-minus-markers --light --line-buffer-size --line-fill-method --line-numbers --line-numbers-left-format --line-numbers-left-style --line-numbers-minus-style --line-numbers-plus-style --line-numbers-right-format --line-numbers-right-style --line-numbers-zero-style --list-languages --list-syntax-themes --map-styles --max-line-distance --max-line-length --merge-conflict-begin-symbol --merge-conflict-end-symbol --merge-conflict-ours-diff-header-decoration-style --merge-conflict-ours-diff-header-style --merge-conflict-theirs-diff-header-decoration-style --merge-conflict-theirs-diff-header-style --minus-empty-line-marker-style --minus-emph-style --minus-non-emph-style --minus-style --navigate --navigate-regex --no-gitconfig --pager --paging --parse-ansi --plus-emph-style --plus-empty-line-marker-style --plus-non-emph-style --plus-style --raw --relative-paths --right-arrow --show-colors --show-config --show-syntax-themes --show-themes --side-by-side --syntax-theme --tabs --true-color --whitespace-error-style --width --word-diff-regex --wrap-left-symbol --wrap-max-lines --wrap-right-percent --wrap-right-prefix-symbol --wrap-right-symbol --zero-style --24-bit-color --help --version [MINUS_FILE] [PLUS_FILE]"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --blame-code-style)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --blame-format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --blame-palette)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --blame-separator-format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --blame-separator-style)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --blame-timestamp-format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --blame-timestamp-output-format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --commit-decoration-style)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --commit-regex)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --commit-style)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --default-language)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --diff-stat-align-width)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --features)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --file-added-label)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --file-copied-label)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --file-decoration-style)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --file-modified-label)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --file-removed-label)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --file-renamed-label)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --file-style)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --file-transformation)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --generate-completion)
                    COMPREPLY=($(compgen -W "bash elvish fish powershell zsh" -- "${cur}"))
                    return 0
                    ;;
                --grep-context-line-style)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --grep-file-style)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --grep-header-decoration-style)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --grep-header-file-style)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --grep-line-number-style)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --grep-output-type)
                    COMPREPLY=($(compgen -W "ripgrep classic" -- "${cur}"))
                    return 0
                    ;;
                --grep-match-line-style)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --grep-match-word-style)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --grep-separator-symbol)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --hunk-header-decoration-style)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --hunk-header-file-style)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --hunk-header-line-number-style)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --hunk-header-style)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --hunk-label)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --hyperlinks-commit-link-format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --hyperlinks-file-link-format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --inline-hint-style)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --inspect-raw-lines)
                    COMPREPLY=($(compgen -W "true false" -- "${cur}"))
                    return 0
                    ;;
                --line-buffer-size)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --line-fill-method)
                    COMPREPLY=($(compgen -W "ansi spaces" -- "${cur}"))
                    return 0
                    ;;
                --line-numbers-left-format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --line-numbers-left-style)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --line-numbers-minus-style)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --line-numbers-plus-style)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --line-numbers-right-format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --line-numbers-right-style)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --line-numbers-zero-style)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --map-styles)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-line-distance)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-line-length)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --merge-conflict-begin-symbol)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --merge-conflict-end-symbol)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --merge-conflict-ours-diff-header-decoration-style)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --merge-conflict-ours-diff-header-style)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --merge-conflict-theirs-diff-header-decoration-style)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --merge-conflict-theirs-diff-header-style)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --minus-empty-line-marker-style)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --minus-emph-style)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --minus-non-emph-style)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --minus-style)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --navigate-regex)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --paging)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --plus-emph-style)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --plus-empty-line-marker-style)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --plus-non-emph-style)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --plus-style)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --right-arrow)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --syntax-theme)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --tabs)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --true-color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --whitespace-error-style)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --width)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -w)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --word-diff-regex)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --wrap-left-symbol)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --wrap-max-lines)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --wrap-right-percent)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --wrap-right-prefix-symbol)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --wrap-right-symbol)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --zero-style)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --24-bit-color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
    esac
}

complete -F _delta -o nosort -o bashdefault -o default delta
