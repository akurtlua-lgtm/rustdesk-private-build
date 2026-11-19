#!/usr/bin/env bash
set -euo pipefail

: "${VERSION:?}" "${BUILD_NUMBER:?}" "${MOBILE_APPLICATION_ID:?}"
: "${VCPKG_ROOT:?}" "${RUNNER_TEMP:?}"
python3 .github/scripts/mobile_build_config.py

manifest_dir="$RUNNER_TEMP/private-ios-manifest"
mkdir -p "$manifest_dir" mobile-output
python3 - "$manifest_dir" <<'PY'
import json
from pathlib import Path
import sys
root = Path.cwd()
manifest = json.loads((root / 'vcpkg.json').read_text())
manifest['dependencies'].append({'name': 'libsodium', 'host': False})
manifest['dependencies'].append({'name': 'libsodium', 'host': True})
config = manifest['vcpkg-configuration']
config['overlay-ports'] = [str(root / 'res/vcpkg')]
config['overlay-triplets'] = [str(root / 'res/vcpkg-triplets')]
(Path(sys.argv[1]) / 'vcpkg.json').write_text(json.dumps(manifest))
PY
"$VCPKG_ROOT/vcpkg" install --triplet arm64-ios-simulator \
  --x-manifest-root="$manifest_dir" --x-install-root="$VCPKG_ROOT/installed"
# The locked magnum-opus build script derives arm64-ios and ignores VCPKG_TARGET_TRIPLET.
test ! -e "$VCPKG_ROOT/installed/arm64-ios"
ln -s arm64-ios-simulator "$VCPKG_ROOT/installed/arm64-ios"
export VCPKG_TARGET_TRIPLET=arm64-ios-simulator
export SODIUM_USE_PKG_CONFIG=1
export LIBSODIUM_STATIC=1
export PKG_CONFIG_ALLOW_CROSS_aarch64_apple_ios_sim=1
export PKG_CONFIG_LIBDIR_aarch64_apple_ios_sim="$VCPKG_ROOT/installed/arm64-ios-simulator/lib/pkgconfig"
export PKG_CONFIG_LIBDIR_aarch64_apple_darwin="$VCPKG_ROOT/installed/arm64-osx/lib/pkgconfig"
export IPHONEOS_DEPLOYMENT_TARGET=15.0
cargo build --locked --release --lib --target aarch64-apple-ios-sim --features flutter
test -s target/aarch64-apple-ios-sim/release/liblibrustdesk.a
(
  cd flutter
  flutter pub get
  flutter build ios --simulator --debug --no-codesign --config-only \
    --build-name "$VERSION" --build-number "$BUILD_NUMBER"
  cd ios
  pod install
)
derived="$RUNNER_TEMP/private-ios-derived"
if ! xcodebuild -workspace flutter/ios/Runner.xcworkspace -scheme Runner \
  -configuration Debug -sdk iphonesimulator \
  -destination 'generic/platform=iOS Simulator' -derivedDataPath "$derived" \
  PRODUCT_BUNDLE_IDENTIFIER="$MOBILE_APPLICATION_ID" \
  CODE_SIGNING_ALLOWED=NO CODE_SIGN_ENTITLEMENTS= build \
  > "$RUNNER_TEMP/private-ios-xcode.log" 2>&1; then
  python3 .github/scripts/mobile_build_error.py "$RUNNER_TEMP/private-ios-xcode.log"
  exit 1
fi
app="$derived/Build/Products/Debug-iphonesimulator/Runner.app"
test -d "$app"
python3 .github/scripts/verify_mobile_artifacts.py ios-simulator "$app" \
  --version "$VERSION" --build-number "$BUILD_NUMBER" \
  --application-id "$MOBILE_APPLICATION_ID" \
  --metadata mobile-output/ios-simulator-build.json
ditto -c -k --sequesterRsrc --keepParent "$app" \
  "mobile-output/rustdesk-$VERSION-ios-simulator-arm64.app.zip"
python3 - "$VERSION" <<'PY'
import hashlib
import json
from pathlib import Path
import sys
root = Path('mobile-output')
archive = root / f'rustdesk-{sys.argv[1]}-ios-simulator-arm64.app.zip'
metadata = root / 'ios-simulator-build.json'
record = json.loads(metadata.read_text())
record.update(artifact=archive.name, size=archive.stat().st_size,
              sha256=hashlib.sha256(archive.read_bytes()).hexdigest())
metadata.write_text(json.dumps(record, indent=2) + '\n')
PY
python3 .github/scripts/smoke_ios_simulator.py "$app" "$MOBILE_APPLICATION_ID"
