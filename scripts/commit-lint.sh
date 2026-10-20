#!/bin/sh
# Lints commit message headers against Conventional Commits
# (https://www.conventionalcommits.org), which release-plz uses to version and
# changelog each plugin.
#
#   scripts/commit-lint.sh <file>             a commit message file (commit-msg hook)
#   scripts/commit-lint.sh --range <a>..<b>   every non-merge commit in a range (CI)
#
# Header format: <type>(<scope>)!: <subject>, at most 100 characters. Scope and
# `!` (breaking change) are optional.

TYPES="build|chore|ci|docs|feat|fix|perf|refactor|revert|style|test"
PATTERN="^($TYPES)(\([a-z0-9._/-]+\))?!?: [^ ].*$"
MAX_LENGTH=100

check() {
    header=$1
    label=$2

    # Commits git generates, or that `git rebase --autosquash` consumes.
    case "$header" in
        "Merge "* | "Revert \""* | "fixup! "* | "squash! "* | "amend! "*) return 0 ;;
    esac

    if ! printf '%s\n' "$header" | grep -Eq "$PATTERN"; then
        printf '✗ %s"%s"\n' "$label" "$header" >&2
        printf '  expected <type>(<scope>)!: <subject>, where <type> is one of: %s\n' "$(echo "$TYPES" | tr '|' ' ')" >&2
        printf '  e.g. "feat(tilt): add readiness probes", "fix: handle missing Tiltfile"\n' >&2
        return 1
    fi

    if [ "${#header}" -gt "$MAX_LENGTH" ]; then
        printf '✗ %sheader is %s characters (max %s): "%s"\n' "$label" "${#header}" "$MAX_LENGTH" "$header" >&2
        return 1
    fi

    return 0
}

if [ "$1" = "--range" ]; then
    status=0

    for sha in $(git rev-list --no-merges "$2"); do
        check "$(git log -1 --format=%s "$sha")" "$(git log -1 --format=%h "$sha"): " || status=1
    done

    exit $status
fi

if [ -z "$1" ] || [ ! -f "$1" ]; then
    echo "usage: $0 <commit-message-file> | --range <a>..<b>" >&2
    exit 2
fi

# The header is the first line that isn't a comment (git strips those).
header=$(grep -v '^#' "$1" | head -n 1)

check "$header" ""
