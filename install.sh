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
  Linux)
    case "$ARCH" in
      x86_64|amd64)  TARGET="x86_64-unknown-linux-gnu" ;;
      aarch64|arm64) TARGET="aarch64-unknown-linux-gnu" ;;
      *) echo "Unsupported architecture: $ARCH" >&2; exit 1 ;;
    esac
    ;;
  Darwin)
    case "$ARCH" in
      x86_64|amd64)  TARGET="x86_64-apple-darwin" ;;
      aarch64|arm64) TARGET="aarch64-apple-darwin" ;;
      *) echo "Unsupported architecture: $ARCH" >&2; exit 1 ;;
    esac
    ;;
  *) echo "Unsupported OS: $OS" >&2; exit 1 ;;
esac

ASSET="${BINARY}-${TARGET}.tar.gz"

# Resolve version
if [ "$VERSION" = "latest" ]; then
  DOWNLOAD_URL="https://github.com/${REPO}/releases/latest/download/${ASSET}"
else
  DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${VERSION}/${ASSET}"
fi

# Download and install
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

echo "Downloading ${BINARY} (${TARGET})..."
curl -fsSL "$DOWNLOAD_URL" -o "${TMPDIR}/${ASSET}"

echo "Verifying checksum..."
curl -fsSL "${DOWNLOAD_URL}.sha256" -o "${TMPDIR}/${ASSET}.sha256"
cd "$TMPDIR"
if command -v sha256sum >/dev/null 2>&1; then
  sha256sum -c "${ASSET}.sha256"
elif command -v shasum >/dev/null 2>&1; then
  shasum -a 256 -c "${ASSET}.sha256"
else
  echo "Warning: no sha256sum or shasum found, skipping checksum verification"
fi
cd - >/dev/null

echo "Extracting..."
tar -xzf "${TMPDIR}/${ASSET}" -C "$TMPDIR"

mkdir -p "$PREFIX"
mv "${TMPDIR}/${BINARY}" "${PREFIX}/${BINARY}"
chmod +x "${PREFIX}/${BINARY}"

echo "Installed ${BINARY} to ${PREFIX}/${BINARY}"

# Install man page if present in tarball
if [ -f "${TMPDIR}/man/hashline.1" ]; then
  MANDIR="${HOME}/.local/share/man/man1"
  mkdir -p "$MANDIR"
  cp "${TMPDIR}/man/hashline.1" "${MANDIR}/hashline.1"
  echo "Installed man page to ${MANDIR}/hashline.1"
fi

# Check PATH
case ":$PATH:" in
  *":${PREFIX}:"*) ;;
  *)
    echo ""
    echo "NOTE: ${PREFIX} is not in your PATH. Add it:"
    echo "  export PATH=\"${PREFIX}:\$PATH\""
    ;;
esac
