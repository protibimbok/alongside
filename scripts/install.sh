#!/usr/bin/env bash
# Install concurtui from GitHub releases (Linux and macOS).
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/protibimbok/concurtui/master/scripts/install.sh | bash
#   curl -fsSL ... | bash -s -- --version v0.1.0
#   curl -fsSL ... | bash -s -- --install-dir ~/.local/bin

set -euo pipefail

REPO="protibimbok/concurtui"
BINARY="concurtui"
RELEASE="latest"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

# ── helpers ────────────────────────────────────────────────────────────────

info()  { printf '\033[0;32m[concurtui]\033[0m %s\n' "$*"; }
warn()  { printf '\033[0;33m[concurtui]\033[0m %s\n' "$*" >&2; }
error() { printf '\033[0;31m[concurtui] error:\033[0m %s\n' "$*" >&2; exit 1; }

need() { command -v "$1" >/dev/null 2>&1 || error "required tool not found: $1"; }

usage() {
  cat <<EOF
Install concurtui from GitHub releases (Linux and macOS).

Usage:
  curl -fsSL https://raw.githubusercontent.com/${REPO}/master/scripts/install.sh | bash
  curl -fsSL ... | bash -s -- [options]

Options:
  -d, --install-dir <dir>   Install directory (default: /usr/local/bin)
  -r, --version <tag>       Release tag to install (default: latest)
  -h, --help                Show this help

Environment:
  INSTALL_DIR               Same as --install-dir

sudo is only used when the install directory isn't writable.
EOF
}

parse_args() {
  while [ $# -gt 0 ]; do
    case "$1" in
      -d | --install-dir)
        [ -n "${2:-}" ] || error "--install-dir requires a path"
        INSTALL_DIR=$2
        shift 2
        ;;
      -r | --version | --release)
        [ -n "${2:-}" ] || error "--version requires a tag"
        RELEASE=$2
        shift 2
        ;;
      -h | --help)
        usage
        exit 0
        ;;
      *)
        error "unknown argument: $1 (try --help)"
        ;;
    esac
  done
}

detect_os() {
  case "$(uname -s)" in
    Linux*)  echo linux ;;
    Darwin*) echo darwin ;;
    MINGW* | MSYS* | CYGWIN*)
      error "on Windows, use scripts/install.ps1 or: npm i -g concurtui" ;;
    *) error "unsupported OS: $(uname -s) (supported: Linux, macOS)" ;;
  esac
}

detect_arch() {
  case "$(uname -m)" in
    x86_64 | amd64)  echo amd64 ;;
    aarch64 | arm64) echo arm64 ;;
    *) error "unsupported architecture: $(uname -m) (supported: amd64, arm64)" ;;
  esac
}

resolve_release_tag() {
  if [ "$RELEASE" = "latest" ]; then
    curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
      | grep -m1 '"tag_name"' \
      | sed -E 's/.*"tag_name"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/'
  else
    case "$RELEASE" in
      v*) printf '%s\n' "$RELEASE" ;;
      *)  printf 'v%s\n' "$RELEASE" ;;
    esac
  fi
}

verify_checksum() {
  local archive=$1 checksums=$2 name expected actual

  name=$(basename "$archive")
  expected=$(awk -v f="$name" '$2 == f { print $1 }' "$checksums")
  [ -n "$expected" ] || error "checksum not found for ${name}"

  if command -v sha256sum >/dev/null 2>&1; then
    actual=$(sha256sum "$archive" | awk '{print $1}')
  elif command -v shasum >/dev/null 2>&1; then
    actual=$(shasum -a 256 "$archive" | awk '{print $1}')
  else
    error "sha256sum or shasum is required to verify downloads"
  fi
  [ "$actual" = "$expected" ] || error "checksum mismatch for ${name}"
}

install_binary() {
  local src=$1 dest="${INSTALL_DIR}/${BINARY}" sudo=""

  if ! mkdir -p "$INSTALL_DIR" 2>/dev/null || [ ! -w "$INSTALL_DIR" ]; then
    command -v sudo >/dev/null 2>&1 \
      || error "${INSTALL_DIR} isn't writable and sudo isn't available (try --install-dir ~/.local/bin)"
    info "Installing to ${dest} (sudo required)"
    sudo=sudo
    $sudo mkdir -p "$INSTALL_DIR"
  fi
  $sudo install -m 755 "$src" "$dest"
}

install_via_release() {
  local tag archive base

  tag=$(resolve_release_tag)
  [ -n "$tag" ] || error "could not resolve the release tag (no release yet, or GitHub rate limit)"

  archive="${BINARY}_${OS}_${ARCH}.tar.gz"
  base="https://github.com/${REPO}/releases/download/${tag}"

  info "Installing concurtui ${tag} (${OS}/${ARCH}) to ${INSTALL_DIR}..."

  # Global, not local: the EXIT trap runs after this function has returned.
  TMP_DIR=$(mktemp -d)
  trap 'rm -rf "$TMP_DIR"' EXIT

  info "Downloading ${base}/${archive}"
  curl -fsSL "${base}/${archive}" -o "${TMP_DIR}/${archive}"
  curl -fsSL "${base}/checksums.txt" -o "${TMP_DIR}/checksums.txt"
  verify_checksum "${TMP_DIR}/${archive}" "${TMP_DIR}/checksums.txt"
  info "Checksum verified"

  tar -xzf "${TMP_DIR}/${archive}" -C "$TMP_DIR"
  install_binary "${TMP_DIR}/${BINARY}"
}

# ── main ───────────────────────────────────────────────────────────────────

parse_args "$@"

need curl
need tar
need awk

OS=$(detect_os)
ARCH=$(detect_arch)

install_via_release

if command -v "$BINARY" >/dev/null 2>&1 && [ "$(command -v "$BINARY")" = "${INSTALL_DIR}/${BINARY}" ]; then
  info "Installed: $("${INSTALL_DIR}/${BINARY}" --version)"
else
  info "Installed: $("${INSTALL_DIR}/${BINARY}" --version) at ${INSTALL_DIR}/${BINARY}"
  warn "${INSTALL_DIR} is not first on your PATH; add it, then run: ${BINARY}"
fi
