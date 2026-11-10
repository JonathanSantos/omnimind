#!/bin/sh
# Monta dist/Omnimind.app sem Xcode: binários, ícone, Info.plist e assinatura ad-hoc.
set -e
cd "$(dirname "$0")/.."
# A versão do pacote é a do workspace: um lugar só para mudar.
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
cargo build --release -p app --bin omnimind-app
cargo build --release -p omnimind-cli --bin omnimind
APP="dist/Omnimind.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
# APFS não diferencia caixa: o app não pode se chamar "Omnimind" ao lado de "omnimind".
cp target/release/omnimind-app "$APP/Contents/MacOS/OmnimindApp"
cp target/release/omnimind "$APP/Contents/MacOS/omnimind"
SET=$(mktemp -d)/AppIcon.iconset
mkdir -p "$SET"
# Até 32 pontos o ícone usa a versão de três fios: cinco se misturam nesse tamanho.
for s in 16 32 128 256 512; do
  src=assets/icon/icon-1024.png
  [ "$s" -le 32 ] && src=assets/icon/icon-small.png
  sips -z $s $s "$src" --out "$SET/icon_${s}x${s}.png" >/dev/null
  d=$((s * 2))
  [ "$d" -le 32 ] || src=assets/icon/icon-1024.png
  sips -z $d $d "$src" --out "$SET/icon_${s}x${s}@2x.png" >/dev/null
done
iconutil -c icns "$SET" -o "$APP/Contents/Resources/AppIcon.icns"
cat > "$APP/Contents/Info.plist" <<P
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>Omnimind</string>
<key>CFBundleDisplayName</key><string>Omnimind</string>
<key>CFBundleIdentifier</key><string>app.omnimind.desktop</string>
<key>CFBundleExecutable</key><string>OmnimindApp</string>
<key>CFBundleIconFile</key><string>AppIcon</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>$VERSION</string>
<key>CFBundleVersion</key><string>$VERSION</string>
<key>LSMinimumSystemVersion</key><string>13.0</string>
<key>NSHighResolutionCapable</key><true/>
<key>LSApplicationCategoryType</key><string>public.app-category.productivity</string>
</dict></plist>
P
# Cofre de exemplo, fictício: o botão "Ver um exemplo" copia daqui.
EX="$APP/Contents/Resources/example"
"$APP/Contents/MacOS/omnimind" init "$EX" --name Exemplo >/dev/null
"$APP/Contents/MacOS/omnimind" import-markdown fixtures/brains/personal --brain "$EX" >/dev/null
"$APP/Contents/MacOS/omnimind" import-markdown fixtures/brains/work --brain "$EX" >/dev/null
cp assets/fonts/OFL-*.txt "$APP/Contents/Resources/"
codesign --force --deep --sign - "$APP"
du -sh "$APP"
