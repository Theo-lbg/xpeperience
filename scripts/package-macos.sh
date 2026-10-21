#!/usr/bin/env bash
# Construit « XpePerience.app » et son image disque .dmg pour macOS.
#
# Usage : ./scripts/package-macos.sh                      (architecture de cette machine)
#         TARGET=x86_64-apple-darwin ./scripts/package-macos.sh   (compilation pour Mac Intel)
# Résultat : dist/XpePerience.app  et  dist/XpePerience-<version>-<arch>.dmg
#
# L'application n'est pas signée par un certificat Apple Developer : voir README.md
# (« Ouvrir l'application la première fois ») pour les autres Mac.
set -euo pipefail

cd "$(dirname "$0")/.."
ROOT="$(pwd)"
NAME="XpePerience"
BIN="xpeperience"
VERSION="$(grep -m1 '^version' Cargo.toml | sed 's/.*"\(.*\)"/\1/')"
TARGET="${TARGET:-}"
if [ -n "$TARGET" ]; then ARCH="${TARGET%%-*}"; [ "$ARCH" = "aarch64" ] && ARCH="arm64"; BUILD_DIR="target/$TARGET/release"; else ARCH="$(uname -m)"; BUILD_DIR="target/release"; fi
APP="dist/$NAME.app"
DMG="dist/XpePerience-$VERSION-$ARCH.dmg"

echo "▸ Interface Vue"
(cd web && npm ci --no-audit --no-fund && npm run build)

echo "▸ Application Rust (release)"
cargo build --release -p xpeperience-desktop ${TARGET:+--target "$TARGET"}

echo "▸ Assemblage de $APP"
rm -rf "$APP" "$DMG"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BUILD_DIR/$BIN" "$APP/Contents/MacOS/$BIN"
cp desktop/icons/icon.icns "$APP/Contents/Resources/icon.icns"

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>$NAME</string>
  <key>CFBundleDisplayName</key><string>$NAME</string>
  <key>CFBundleIdentifier</key><string>fr.xpeho.xpeperience</string>
  <key>CFBundleExecutable</key><string>$BIN</string>
  <key>CFBundleIconFile</key><string>icon</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$VERSION</string>
  <key>CFBundleVersion</key><string>$VERSION</string>
  <key>LSMinimumSystemVersion</key><string>12.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>LSApplicationCategoryType</key><string>public.app-category.business</string>
  <key>NSHumanReadableCopyright</key><string>XPEHO</string>
</dict>
</plist>
PLIST

# Signature « ad hoc » : sans identité Apple, mais l'application reste cohérente et lançable.
codesign --force --deep --sign - "$APP" >/dev/null 2>&1 || echo "  (signature ad hoc indisponible, ignorée)"

echo "▸ Image disque $DMG"
STAGE="$(mktemp -d)"
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications"
hdiutil create -volname "$NAME" -srcfolder "$STAGE" -ov -format UDZO "$DMG" >/dev/null
rm -rf "$STAGE"

echo
echo "Terminé :"
du -sh "$APP" "$DMG" | sed 's/^/  /'
echo "Pour l'installer : ouvrez le .dmg et glissez l'application dans Applications."
