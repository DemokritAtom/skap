#!/usr/bin/env sh
# Shell installer for skap.
#
# Usage: curl -fsSL https://raw.githubusercontent.com/DemokritAtom/skap/main/install.sh | sh
set -eu

REPO="DemokritAtom/skap"
INSTALL_DIR="${SKAP_INSTALL_DIR:-$HOME/.local/bin}"

OS="$(uname -s)"
ARCH="$(uname -m)"
case "$OS-$ARCH" in
  Linux-x86_64)   TARGET="x86_64-unknown-linux-gnu" ;;
  Linux-aarch64)  TARGET="aarch64-unknown-linux-gnu" ;;
  Darwin-x86_64)  TARGET="x86_64-apple-darwin" ;;
  Darwin-arm64)   TARGET="aarch64-apple-darwin" ;;
  *) echo "unsupported platform: $OS $ARCH" >&2; exit 1 ;;
esac

VERSION="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
  | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p')"
if [ -z "$VERSION" ]; then
  echo "could not determine latest version" >&2
  exit 1
fi

URL="https://github.com/${REPO}/releases/download/${VERSION}/skap-${VERSION}-${TARGET}.tar.gz"
echo "downloading $URL"

mkdir -p "$INSTALL_DIR"
TMP="$(mktemp -d)"
curl -fsSL "$URL" | tar -C "$TMP" -xzf -
mv "$TMP/skap" "$INSTALL_DIR/skap"
chmod +x "$INSTALL_DIR/skap"
rm -rf "$TMP"

echo "installed to $INSTALL_DIR/skap"
case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *) echo "Hint: add $INSTALL_DIR to your PATH" ;;
esac
