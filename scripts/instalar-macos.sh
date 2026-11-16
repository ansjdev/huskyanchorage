#!/bin/sh
# Monta o KeyGlow.app e instala em /Applications, para abrir pelo Spotlight ou
# pelo Launchpad. Rode de novo depois de mudar o código para atualizar o app.
#
#   ./scripts/instalar-macos.sh
set -eu

cd "$(dirname "$0")/.."

BUNDLE_ID="br.com.filipesampaio.keyglow"
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
APP="dist/KeyGlow.app"
ICON="assets/macos/KeyGlow.icns"

cargo build --release

# Ícone: gerado uma vez e guardado no repositório.
if [ ! -f "$ICON" ]; then
    tmp=$(mktemp -d)
    swift scripts/icone-macos.swift "$tmp/1024.png"
    mkdir -p "$tmp/KeyGlow.iconset" "$(dirname "$ICON")"
    for s in 16 32 128 256 512; do
        sips -z $s $s "$tmp/1024.png" --out "$tmp/KeyGlow.iconset/icon_${s}x${s}.png" >/dev/null
        d=$((s * 2))
        sips -z $d $d "$tmp/1024.png" --out "$tmp/KeyGlow.iconset/icon_${s}x${s}@2x.png" >/dev/null
    done
    iconutil -c icns "$tmp/KeyGlow.iconset" -o "$ICON"
    rm -rf "$tmp"
fi

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp target/release/keyglow "$APP/Contents/MacOS/keyglow"
cp "$ICON" "$APP/Contents/Resources/KeyGlow.icns"

cat > "$APP/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>KeyGlow</string>
    <key>CFBundleDisplayName</key><string>KeyGlow</string>
    <key>CFBundleIdentifier</key><string>$BUNDLE_ID</string>
    <key>CFBundleExecutable</key><string>keyglow</string>
    <key>CFBundleIconFile</key><string>KeyGlow</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
    <key>CFBundleShortVersionString</key><string>$VERSION</string>
    <key>CFBundleVersion</key><string>$VERSION</string>
    <key>LSMinimumSystemVersion</key><string>11.0</string>
    <key>LSApplicationCategoryType</key><string>public.app-category.utilities</string>
    <key>NSHighResolutionCapable</key><true/>
    <key>NSAudioCaptureUsageDescription</key>
    <string>O keyglow escuta o som do Mac para os efeitos de música no teclado.</string>
</dict>
</plist>
EOF

# Assinatura local (ad-hoc): por último, porque ela cobre o Info.plist.
codesign --force --sign - --identifier "$BUNDLE_ID" "$APP"
codesign --verify --strict "$APP"

if [ -w /Applications ]; then DEST=/Applications; else DEST="$HOME/Applications"; mkdir -p "$DEST"; fi

# Fecha o app instalado, se estiver aberto, para poder trocar o binário.
osascript -e "if application id \"$BUNDLE_ID\" is running then tell application id \"$BUNDLE_ID\" to quit" 2>/dev/null || true

rm -rf "$DEST/KeyGlow.app"
ditto "$APP" "$DEST/KeyGlow.app"
# Sem a cópia de montagem o Spotlight não mostra o app duas vezes.
rm -rf "$APP"

# Avisa o Spotlight e o Launch Services do app novo.
mdimport "$DEST/KeyGlow.app" 2>/dev/null || true
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f "$DEST/KeyGlow.app" 2>/dev/null || true

echo "Instalado em $DEST/KeyGlow.app"
