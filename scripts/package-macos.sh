#!/bin/bash
# Seal the final bundle, archive it, then verify the extracted download.
set -euo pipefail
app="$1"
archive="$2"
identity="${APPLE_SIGNING_IDENTITY:--}"
if [[ "$identity" == "-" ]]; then
  /usr/bin/codesign --force --sign - "$app"
else
  # Tauri signs nested code and notarizes before this script. Never re-sign a notarized app.
  /usr/bin/xcrun stapler validate "$app"
  /usr/sbin/spctl --assess --type execute --verbose=2 "$app"
fi
/usr/bin/codesign --verify --deep --strict --verbose=2 "$app"
/usr/bin/ditto -c -k --sequesterRsrc --keepParent "$app" "$archive"
check_dir=$(mktemp -d)
trap 'rm -rf "$check_dir"' EXIT
/usr/bin/ditto -x -k "$archive" "$check_dir"
extracted="$check_dir/$(basename "$app")"
/usr/bin/codesign --verify --deep --strict --verbose=2 "$extracted"
test -x "$extracted/Contents/MacOS/riverlens"
if [[ "$identity" != "-" ]]; then
  /usr/bin/xcrun stapler validate "$extracted"
  /usr/sbin/spctl --assess --type execute --verbose=2 "$extracted"
fi
