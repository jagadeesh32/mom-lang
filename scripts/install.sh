#!/usr/bin/env bash
# install.sh — install a tagged `mom` release for the current host.
#
# Usage:
#   curl -fsSL https://mom-lang.dev/install.sh | bash
#   curl -fsSL https://mom-lang.dev/install.sh | bash -s -- --version 0.6.0
#   curl -fsSL https://mom-lang.dev/install.sh | bash -s -- --prefix ~/.local
#
# The script never elevates. It writes to --prefix/bin (default
# $HOME/.mom/bin) and prints the line you should add to PATH.

set -euo pipefail

VERSION="latest"
PREFIX="$HOME/.mom"
BASE_URL="${MOM_INSTALL_BASE_URL:-https://github.com/mom-lang/mom/releases/download}"

while [ $# -gt 0 ]; do
    case "$1" in
        --version) VERSION="$2"; shift 2 ;;
        --prefix)  PREFIX="$2";  shift 2 ;;
        --help|-h)
            sed -n '2,12p' "$0"
            exit 0
            ;;
        *)
            echo "install.sh: unknown flag '$1'" >&2
            exit 2
            ;;
    esac
done

detect_triple() {
    local os arch
    case "$(uname -s)" in
        Linux)   os="linux"   ;;
        Darwin)  os="darwin"  ;;
        MINGW*|MSYS*|CYGWIN*) os="windows" ;;
        *) echo "install.sh: unsupported OS '$(uname -s)'" >&2; exit 1 ;;
    esac
    case "$(uname -m)" in
        x86_64|amd64) arch="x86_64" ;;
        aarch64|arm64) arch="aarch64" ;;
        *) echo "install.sh: unsupported arch '$(uname -m)'" >&2; exit 1 ;;
    esac
    printf '%s-%s' "$arch" "$os"
}

resolve_version() {
    if [ "$VERSION" != "latest" ]; then
        printf 'v%s' "${VERSION#v}"
        return
    fi
    # Fall back to the GitHub releases redirect that always points at
    # the newest stable tag.
    local resolved
    resolved=$(curl -fsSLI -o /dev/null -w '%{url_effective}' \
        "https://github.com/mom-lang/mom/releases/latest")
    printf '%s' "${resolved##*/}"
}

main() {
    local triple tag url tmp bin
    triple=$(detect_triple)
    tag=$(resolve_version)
    url="$BASE_URL/$tag/mom-$triple.tar.gz"
    tmp=$(mktemp -d)
    trap 'rm -rf "$tmp"' EXIT

    echo "==> downloading $url"
    curl -fsSL "$url" -o "$tmp/mom.tar.gz"

    echo "==> extracting"
    tar -xzf "$tmp/mom.tar.gz" -C "$tmp"

    mkdir -p "$PREFIX/bin"
    bin="$PREFIX/bin/mom"
    install -m 0755 "$tmp/mom" "$bin"

    echo "==> installed $tag → $bin"
    "$bin" version

    cat <<EOF

Add the following to your shell profile if it isn't there already:

    export PATH="$PREFIX/bin:\$PATH"

Then run \`mom new my-project\` to start. Docs: https://mom-lang.dev/docs
EOF
}

main
