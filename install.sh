#!/bin/sh
set -eu

REPO="lispmeister/hashline"
BINARY="hashline"
DEFAULT_PREFIX="$HOME/.local/bin"

# Parse args
PREFIX="$DEFAULT_PREFIX"
VERSION="latest"
while [ $# -gt 0 ]; do
  case "$1" in
    --prefix) PREFIX="$2"; shift 2 ;;
    --version) VERSION="$2"; shift 2 ;;
    *) echo "Unknown option: $1" >&2; exit 1 ;;
  esac
done

# Detect platform
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux)  OS_TAG="linux" ;;
  Darwin) OS_TAG="darwin" ;;
  *) echo "Unsupported OS: $OS" >&2; exit 1 ;;
esac

case "$ARCH" in
  x86_64|amd64)  ARCH_TAG="x86_64" ;;
  aarch64|arm64) ARCH_TAG="aarch64" ;;
  *) echo "Unsupported architecture: $ARCH" >&2; exit 1 ;;
esac

ASSET="${BINARY}-${OS_TAG}-${ARCH_TAG}.tar.gz"

# Resolve version
if [ "$VERSION" = "latest" ]; then
  DOWNLOAD_URL="https://github.com/${REPO}/releases/latest/download/${ASSET}"
else
  DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${VERSION}/${ASSET}"
fi

# Download and install
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

echo "Downloading ${BINARY} (${OS_TAG}/${ARCH_TAG})..."
curl -fsSL "$DOWNLOAD_URL" -o "${TMPDIR}/${ASSET}"

echo "Extracting..."
tar -xzf "${TMPDIR}/${ASSET}" -C "$TMPDIR"

mkdir -p "$PREFIX"
mv "${TMPDIR}/${BINARY}" "${PREFIX}/${BINARY}"
chmod +x "${PREFIX}/${BINARY}"

echo "Installed ${BINARY} to ${PREFIX}/${BINARY}"

# Check PATH
case ":$PATH:" in
  *":${PREFIX}:"*) ;;
  *)
    echo ""
    echo "NOTE: ${PREFIX} is not in your PATH. Add it:"
    echo "  export PATH=\"${PREFIX}:\$PATH\""
    ;;
esac
