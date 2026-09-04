#!/bin/sh
# Сборка на macOS: кладёт release-бинарник в Blackout.app с иконкой.
# Рисунки не нужны — .icns пишет cargo build в target/blackout.icns.
set -e
root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
bin="$root/target/release/blackout"
icns="$root/target/blackout.icns"
app="$root/target/release/Blackout.app"

if [ ! -f "$bin" ]; then
    echo "нет $bin — сначала: cargo build --release" >&2
    exit 1
fi
if [ ! -f "$icns" ]; then
    echo "нет $icns — пересоберите: cargo build --release" >&2
    exit 1
fi

rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "$bin" "$app/Contents/MacOS/blackout"
cp "$icns" "$app/Contents/Resources/AppIcon.icns"
chmod +x "$app/Contents/MacOS/blackout"

cat > "$app/Contents/Info.plist" <<'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>blackout</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>CFBundleIdentifier</key>
    <string>local.blackout</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>Blackout</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0.0</string>
    <key>CFBundleVersion</key>
    <string>1.0.0</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.13</string>
    <key>LSUIElement</key>
    <true/>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
EOF

echo "$app"
