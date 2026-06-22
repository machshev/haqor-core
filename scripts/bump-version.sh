# Bump the haqor-core crate version following SemVer.
#
# Usage:
#   bump-version <major|minor|patch>   bump the matching component
#   bump-version <X.Y.Z>               set an explicit version
#   bump-version ... --tag             also create a git commit + vX.Y.Z tag
#
# Updates the [package] version in Cargo.toml and the matching entry in
# Cargo.lock. Does not push or publish — use `cargo release` for that.

set -euo pipefail

usage() {
    echo "usage: bump-version <major|minor|patch|X.Y.Z> [--tag]" >&2
    exit 2
}

[ $# -ge 1 ] || usage

bump=""
do_tag=0
for arg in "$@"; do
    case "$arg" in
        --tag) do_tag=1 ;;
        -h | --help) usage ;;
        -*)
            echo "unknown flag: $arg" >&2
            usage
            ;;
        *)
            [ -z "$bump" ] || usage
            bump="$arg"
            ;;
    esac
done
[ -n "$bump" ] || usage

root="$(git rev-parse --show-toplevel)"
manifest="$root/Cargo.toml"
lock="$root/Cargo.lock"

cur="$(grep -m1 '^version = ' "$manifest" | sed -E 's/^version = "([^"]+)".*/\1/')"
if ! [[ "$cur" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "could not parse current version from $manifest (got '$cur')" >&2
    exit 1
fi
IFS='.' read -r major minor patch <<<"$cur"

case "$bump" in
    major) new="$((major + 1)).0.0" ;;
    minor) new="${major}.$((minor + 1)).0" ;;
    patch) new="${major}.${minor}.$((patch + 1))" ;;
    *)
        if [[ "$bump" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
            new="$bump"
        else
            echo "invalid bump '$bump': expected major|minor|patch or X.Y.Z" >&2
            exit 1
        fi
        ;;
esac

if [ "$new" = "$cur" ]; then
    echo "version already $cur, nothing to do" >&2
    exit 1
fi

# Cargo.toml: replace the first (i.e. [package]) version line only.
sed -i -E "0,/^version = \"$cur\"/ s//version = \"$new\"/" "$manifest"

# Cargo.lock: replace the version inside the haqor-core package block.
if [ -f "$lock" ]; then
    awk -v new="$new" '
        /^name = "haqor-core"$/ { in_pkg = 1 }
        in_pkg && /^version = / { sub(/"[^"]+"/, "\"" new "\""); in_pkg = 0 }
        { print }
    ' "$lock" >"$lock.tmp" && mv "$lock.tmp" "$lock"
fi

echo "bumped $cur -> $new"

if [ "$do_tag" -eq 1 ]; then
    git -C "$root" add Cargo.toml Cargo.lock
    git -C "$root" commit -m "chore: release v$new"
    git -C "$root" tag -a "v$new" -m "v$new"
    echo "committed and tagged v$new (not pushed)"
fi
