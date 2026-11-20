#!/usr/bin/env bash
set -euo pipefail
umask 022

# OUTPUT.zip (optional): the app alone, zipped as the Finder would.
if [[ $# -ne 4 && $# -ne 5 ]]; then
    echo "Usage: $0 VERSION DATE TARGET_DIR OUTPUT.dmg [OUTPUT.zip]" >&2
    exit 2
fi

version=$1
date=$2
target_dir=$3
output=$4
zip_output=${5:-}

if [[ ! $version =~ ^[0-9][A-Za-z0-9.+~-]*$ ]]; then
    echo "Invalid macOS package version: $version" >&2
    exit 2
fi
if [[ ! $date =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]]; then
    echo "Invalid release date: $date" >&2
    exit 2
fi

root=$(cd "$(dirname "$0")/../.." && pwd)
cd "$root"

stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT

app="$stage/CreaBIM.app"
contents="$app/Contents"
install -d "$contents/MacOS" "$contents/Resources"
install -m 0755 "$target_dir/creabim-app" "$contents/MacOS/CreaBIM"
install -m 0755 "$target_dir/creabim" "$contents/MacOS/creabim-cli"
install -m 0644 crates/creabim-app/assets/logo/creabim.icns \
    "$contents/Resources/creabim.icns"
install -m 0644 LICENSE "$stage/LICENSE"
install -m 0644 docs/RELEASE_README.txt "$stage/READ ME FIRST.txt"
ln -s /Applications "$stage/Applications"

bundle_version=${version%%[-+~]*}
cat > "$contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>CreaBIM</string>
    <key>CFBundleIdentifier</key>
    <string>io.github.chopinregis.creabim</string>
    <key>CFBundleName</key>
    <string>CreaBIM</string>
    <key>CFBundleDisplayName</key>
    <string>CreaBIM</string>
    <key>CFBundleIconFile</key>
    <string>creabim</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>$bundle_version</string>
    <key>CFBundleVersion</key>
    <string>$bundle_version</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.graphics-design</string>
    <key>NSSupportsAutomaticGraphicsSwitching</key>
    <true/>
</dict>
</plist>
PLIST

# Ad-hoc signed: Apple Silicon runs no unsigned code, and a bundle signed
# as a whole opens with right-click › Open instead of "is damaged".
if command -v codesign >/dev/null; then
    codesign --force --deep --sign - "$app"
    codesign --verify --deep --strict "$app"
fi

mkdir -p "$(dirname "$output")"
if [[ -n $zip_output ]]; then
    ditto -c -k --keepParent "$app" "$zip_output"
fi
hdiutil create -volname "CreaBIM $version" -srcfolder "$stage" \
    -ov -format UDZO "$output"
echo "Created $output for CreaBIM $version (release date $date)"
