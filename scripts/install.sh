#!/usr/bin/env bash
# install.sh — install mom programming language on Linux / macOS
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/jagadeesh32/mom/main/scripts/install.sh | bash
#   curl -fsSL https://raw.githubusercontent.com/jagadeesh32/mom/main/scripts/install.sh | bash -s -- --version v0.2.0
#   curl -fsSL https://raw.githubusercontent.com/jagadeesh32/mom/main/scripts/install.sh | bash -s -- --prefix ~/.local
#
# Options:
#   --version VERSION   Install a specific version (default: latest)
#   --prefix  PATH      Install prefix (default: $HOME/.local)
#   --no-path           Don't modify shell config to add bin to PATH
#
# The installer:
#   1. Detects your platform (OS + arch)
#   2. Downloads the matching release archive from GitHub
#   3. Extracts the binary + runtime to PREFIX/lib/mom/
#   4. Symlinks the binary to PREFIX/bin/mom
#   5. Optionally adds PREFIX/bin to PATH in your shell config
#
# Requirements: curl or wget, tar (Linux/macOS), unzip (Windows WSL)

set -euo pipefail

# ── Configuration ─────────────────────────────────────────────────────────────
REPO="jagadeesh32/mom"
INSTALL_PREFIX="${HOME}/.local"
INSTALL_VERSION="latest"
MODIFY_PATH=true

# ── Parse args ────────────────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
  case "$1" in
    --version)  INSTALL_VERSION="$2"; shift 2 ;;
    --prefix)   INSTALL_PREFIX="$2";  shift 2 ;;
    --no-path)  MODIFY_PATH=false;    shift   ;;
    *)          echo "Unknown option: $1" >&2; exit 1 ;;
  esac
done

BIN_DIR="${INSTALL_PREFIX}/bin"
LIB_DIR="${INSTALL_PREFIX}/lib/mom"

# ── Helpers ───────────────────────────────────────────────────────────────────
info()    { printf '\033[0;32m[mom]\033[0m %s\n' "$*"; }
warn()    { printf '\033[0;33m[mom]\033[0m %s\n' "$*" >&2; }
error()   { printf '\033[0;31m[mom]\033[0m %s\n' "$*" >&2; exit 1; }

need_cmd() { command -v "$1" >/dev/null 2>&1 || error "required command not found: $1"; }
have_cmd() { command -v "$1" >/dev/null 2>&1; }

download() {
  local url="$1" dest="$2"
  if have_cmd curl; then
    curl -fsSL --retry 3 --retry-delay 2 -o "$dest" "$url"
  elif have_cmd wget; then
    wget -q --tries=3 -O "$dest" "$url"
  else
    error "neither curl nor wget found; install one and retry"
  fi
}

# ── Detect platform ───────────────────────────────────────────────────────────
detect_platform() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"

  case "${os}-${arch}" in
    Linux-x86_64)               echo "mom-linux-x86_64" ;;
    Linux-aarch64|Linux-arm64)  echo "mom-linux-aarch64" ;;
    Darwin-arm64)               echo "mom-macos-aarch64" ;;
    Darwin-x86_64)              error "macOS Intel (x86_64) is not a released target. Use Rosetta 2 or build from source." ;;
    *)                          error "Unsupported platform: ${os}-${arch}" ;;
  esac
}

# ── Resolve version ───────────────────────────────────────────────────────────
resolve_version() {
  if [ "${INSTALL_VERSION}" = "latest" ]; then
    info "Checking latest release..."
    local release_url="https://api.github.com/repos/${REPO}/releases/latest"
    local tmp; tmp="$(mktemp)"
    download "${release_url}" "${tmp}"
    INSTALL_VERSION="$(grep '"tag_name"' "${tmp}" | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')"
    rm -f "${tmp}"
    [ -n "${INSTALL_VERSION}" ] || error "Could not determine latest version"
    info "Latest version: ${INSTALL_VERSION}"
  fi
}

# ── Download and install ──────────────────────────────────────────────────────
main() {
  need_cmd uname
  need_cmd tar

  local platform; platform="$(detect_platform)"
  resolve_version

  local asset="${platform}.tar.gz"
  local download_url="https://github.com/${REPO}/releases/download/${INSTALL_VERSION}/${asset}"

  info "Platform:  ${platform}"
  info "Version:   ${INSTALL_VERSION}"
  info "Prefix:    ${INSTALL_PREFIX}"

  local tmp_dir; tmp_dir="$(mktemp -d)"
  trap 'rm -rf "${tmp_dir}"' EXIT

  info "Downloading ${asset}..."
  download "${download_url}" "${tmp_dir}/${asset}"

  # Verify checksum if available
  local checksum_url="https://github.com/${REPO}/releases/download/${INSTALL_VERSION}/SHA256SUMS.txt"
  local checksum_file="${tmp_dir}/SHA256SUMS.txt"
  if download "${checksum_url}" "${checksum_file}" 2>/dev/null; then
    info "Verifying checksum..."
    if have_cmd sha256sum; then
      (cd "${tmp_dir}" && grep "${asset}" "${checksum_file}" | sha256sum --check --status) || \
        error "Checksum verification failed"
      info "Checksum OK ✓"
    elif have_cmd shasum; then
      (cd "${tmp_dir}" && grep "${asset}" "${checksum_file}" | sed 's/ / */' | shasum -a 256 --check --status) || \
        error "Checksum verification failed"
      info "Checksum OK ✓"
    fi
  fi

  info "Extracting..."
  mkdir -p "${tmp_dir}/extract"
  tar -xzf "${tmp_dir}/${asset}" -C "${tmp_dir}/extract"
  local extracted; extracted="$(find "${tmp_dir}/extract" -maxdepth 1 -mindepth 1 -type d | head -1)"

  # Install binary
  mkdir -p "${BIN_DIR}"
  install -m755 "${extracted}/mom" "${BIN_DIR}/mom"

  # Install runtime + std
  mkdir -p "${LIB_DIR}"
  if [ -d "${extracted}/compiler" ]; then
    rm -rf "${LIB_DIR}/compiler"
    cp -r "${extracted}/compiler" "${LIB_DIR}/compiler"
  fi
  if [ -d "${extracted}/std" ]; then
    rm -rf "${LIB_DIR}/std"
    cp -r "${extracted}/std" "${LIB_DIR}/std"
  fi

  info "Installed mom ${INSTALL_VERSION} to ${BIN_DIR}/mom"

  # ── PATH setup ──────────────────────────────────────────────────────────────
  if [ "${MODIFY_PATH}" = true ]; then
    local path_line="export PATH=\"${BIN_DIR}:\$PATH\""
    local added=false

    for rc in "${HOME}/.bashrc" "${HOME}/.zshrc" "${HOME}/.profile"; do
      if [ -f "${rc}" ] && ! grep -qF "${BIN_DIR}" "${rc}" 2>/dev/null; then
        printf '\n# mom programming language\n%s\n' "${path_line}" >> "${rc}"
        info "Added PATH to ${rc}"
        added=true
      fi
    done

    if ! $added && echo ":${PATH}:" | grep -q ":${BIN_DIR}:"; then
      info "${BIN_DIR} is already in PATH"
    fi
  fi

  # ── Done ────────────────────────────────────────────────────────────────────
  echo ""
  echo "  ╔══════════════════════════════════════════╗"
  echo "  ║   mom ${INSTALL_VERSION} installed successfully!    ║"
  echo "  ╚══════════════════════════════════════════╝"
  echo ""

  if ! echo ":${PATH}:" | grep -q ":${BIN_DIR}:"; then
    echo "  Restart your shell or run:"
    echo "    export PATH=\"${BIN_DIR}:\$PATH\""
    echo ""
  fi

  echo "  Try it:"
  echo "    mom version"
  echo "    mom run examples/hello.mom"
  echo ""
}

main "$@"
