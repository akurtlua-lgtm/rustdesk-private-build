#!/usr/bin/env bash
set -euo pipefail

: "${PROBE_TARGET:?}" "${PROBE_SDK:?}" "${PROBE_TRIPLET:?}"
: "${VCPKG_ROOT:?}" "${RUNNER_TEMP:?}"
test "$(uname -m)" = arm64
case "$PROBE_TARGET:$PROBE_SDK:$PROBE_TRIPLET" in
  aarch64-apple-ios:iphoneos:arm64-ios|aarch64-apple-ios-sim:iphonesimulator:arm64-ios-simulator) ;;
  *) echo 'Unexpected extension probe target' >&2; exit 1 ;;
esac
manifest="$RUNNER_TEMP/ios-host-probe-manifest"
mkdir -p "$manifest" probe-output
python3 - "$manifest" <<'PY'
import json
from pathlib import Path
import sys
root = Path.cwd()
config = json.loads((root / 'vcpkg.json').read_text())
config['dependencies'] += [{'name': 'libsodium', 'host': host} for host in (False, True)]
config['vcpkg-configuration']['overlay-ports'] = [str(root / 'res/vcpkg')]
config['vcpkg-configuration']['overlay-triplets'] = [str(root / 'res/ios-host-probe/triplets'), str(root / 'res/vcpkg-triplets')]
(Path(sys.argv[1]) / 'vcpkg.json').write_text(json.dumps(config))
PY
"$VCPKG_ROOT/vcpkg" install --triplet "$PROBE_TRIPLET" \
  --x-manifest-root="$manifest" --x-install-root="$VCPKG_ROOT/installed"
if [[ "$PROBE_SDK" == iphonesimulator ]]; then
  test ! -e "$VCPKG_ROOT/installed/arm64-ios"
  ln -s arm64-ios-simulator "$VCPKG_ROOT/installed/arm64-ios"
fi
export SODIUM_USE_PKG_CONFIG=1 LIBSODIUM_STATIC=1 IPHONEOS_DEPLOYMENT_TARGET=15.0
export PKG_CONFIG_LIBDIR_aarch64_apple_darwin="$VCPKG_ROOT/installed/arm64-osx/lib/pkgconfig"
sdk=$(xcrun --sdk "$PROBE_SDK" --show-sdk-path)
if [[ "$PROBE_SDK" == iphonesimulator ]]; then
  export PKG_CONFIG_ALLOW_CROSS_aarch64_apple_ios_sim=1
  export PKG_CONFIG_LIBDIR_aarch64_apple_ios_sim="$VCPKG_ROOT/installed/$PROBE_TRIPLET/lib/pkgconfig"
  export BINDGEN_EXTRA_CLANG_ARGS_aarch64_apple_ios_sim="--target=arm64-apple-ios15.0-simulator -isysroot \"$sdk\""
else
  export PKG_CONFIG_ALLOW_CROSS_aarch64_apple_ios=1
  export PKG_CONFIG_LIBDIR_aarch64_apple_ios="$VCPKG_ROOT/installed/$PROBE_TRIPLET/lib/pkgconfig"
  export BINDGEN_EXTRA_CLANG_ARGS_aarch64_apple_ios="--target=arm64-apple-ios15.0 -isysroot \"$sdk\""
fi
cargo build --locked --release --lib --target "$PROBE_TARGET" --features flutter,ios-host-link-probe
derived="$RUNNER_TEMP/ios-host-link-probe-derived"
if ! xcodebuild -project res/ios-host-probe/Probe.xcodeproj -target BroadcastProbe \
  -configuration Release -sdk "$PROBE_SDK" SYMROOT="$derived" \
  ARCHS=arm64 ONLY_ACTIVE_ARCH=YES CODE_SIGNING_ALLOWED=NO \
  RUSTDESK_PROBE_LIBRARY="$PWD/target/$PROBE_TARGET/release/liblibrustdesk.a" build \
  > "$RUNNER_TEMP/ios-host-link-probe.log" 2>&1; then
  python3 .github/scripts/mobile_build_error.py "$RUNNER_TEMP/ios-host-link-probe.log"
  exit 1
fi
python3 .github/scripts/verify_ios_host_probe.py \
  "$derived/Release-$PROBE_SDK/BroadcastProbe.appex" "$PROBE_SDK"
