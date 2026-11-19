#!/usr/bin/env bash
set -euo pipefail

: "${VERSION:?}" "${BUILD_NUMBER:?}" "${MOBILE_APPLICATION_ID:?}"
: "${OHOS_SDK_ROOT:?}" "${VCPKG_ROOT:?}" "${RUNNER_TEMP:?}"
: "${HVIGOR_NODE_MODULES:?}" "${JAVA_HOME:?}"
python3 .github/scripts/mobile_build_config.py
python3 - <<'PY'
import json
import os
from pathlib import Path
import subprocess
root = Path.cwd()
common = root / 'libs/hbb_common'
if subprocess.check_output(['git', '-C', str(common), 'rev-parse', 'HEAD'], text=True).strip() != 'fae47e024dd83cef6aaecde0aa7d11731df11649':
    raise ValueError('Unexpected shared protocol source; review the exact submodule range first')
patch = root / 'res/ohos/hbb_common-ohos.patch'
diff = subprocess.check_output(['git', '-C', str(common), 'diff', '--binary', 'HEAD'])
if not diff:
    subprocess.run(['git', '-C', str(common), 'apply', '--check', str(patch)], check=True)
    subprocess.run(['git', '-C', str(common), 'apply', str(patch)], check=True)
elif diff != patch.read_bytes():
    raise ValueError('Refusing to modify unrelated shared protocol changes')
for component in ('native', 'ets', 'js', 'previewer', 'toolchains'):
    record = json.loads((Path(os.environ['OHOS_SDK_ROOT']) / component / 'oh-uni-package.json').read_text())
    if record['apiVersion'] != '20' or record['version'] != '6.0.0.47':
        raise ValueError('Unexpected public SDK component')
if subprocess.check_output(['git', '-C', os.environ['VCPKG_ROOT'], 'rev-parse', 'HEAD'], text=True).strip() != '9e593bb18ea69cc5095e012465dcd675a822ed0d':
    raise ValueError('Unexpected native dependency baseline')
PY
export VCPKG_DISABLE_METRICS=1
"$VCPKG_ROOT/vcpkg" install --triplet arm64-ohos \
  --x-manifest-root="$PWD/res/ohos" --x-install-root="$VCPKG_ROOT/installed"
# The locked magnum-opus build script derives arm64-linux before target_env.
if [[ -L "$VCPKG_ROOT/installed/arm64-linux" ]]; then
  test "$(readlink "$VCPKG_ROOT/installed/arm64-linux")" = arm64-ohos
else
  test ! -e "$VCPKG_ROOT/installed/arm64-linux"
  ln -s arm64-ohos "$VCPKG_ROOT/installed/arm64-linux"
fi
native="$OHOS_SDK_ROOT/native"
export CC_aarch64_unknown_linux_ohos="$native/llvm/bin/clang"
export CXX_aarch64_unknown_linux_ohos="$native/llvm/bin/clang++"
export AR_aarch64_unknown_linux_ohos="$native/llvm/bin/llvm-ar"
export CFLAGS_aarch64_unknown_linux_ohos="--target=aarch64-linux-ohos --sysroot=$native/sysroot"
export CXXFLAGS_aarch64_unknown_linux_ohos="$CFLAGS_aarch64_unknown_linux_ohos"
export BINDGEN_EXTRA_CLANG_ARGS_aarch64_unknown_linux_ohos="$CFLAGS_aarch64_unknown_linux_ohos"
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_OHOS_LINKER="$native/llvm/bin/clang"
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_OHOS_RUSTFLAGS="-C link-arg=--target=aarch64-linux-ohos -C link-arg=--sysroot=$native/sysroot -C link-arg=-lc++_shared -C rpath=no"
export SODIUM_USE_PKG_CONFIG=1
export LIBSODIUM_STATIC=1
export PKG_CONFIG_ALLOW_CROSS_aarch64_unknown_linux_ohos=1
export PKG_CONFIG_LIBDIR_aarch64_unknown_linux_ohos="$VCPKG_ROOT/installed/arm64-ohos/lib/pkgconfig"
export PKG_CONFIG_LIBDIR_aarch64_apple_darwin="$VCPKG_ROOT/installed/arm64-osx/lib/pkgconfig"
cargo build --locked --release --lib --target aarch64-unknown-linux-ohos --no-default-features --features use_dasp
mkdir -p harmonyos/entry/libs/arm64-v8a harmonyos/entry/oh_modules mobile-output
cp target/aarch64-unknown-linux-ohos/release/liblibrustdesk.so harmonyos/entry/libs/arm64-v8a/
cp "$native/llvm/lib/aarch64-linux-ohos/libc++_shared.so" harmonyos/entry/libs/arm64-v8a/
if [[ ! -e harmonyos/entry/oh_modules/libentry.so ]]; then
  ln -s ../src/main/cpp/types/libentry harmonyos/entry/oh_modules/libentry.so
fi
export OHOS_BASE_SDK_HOME
OHOS_BASE_SDK_HOME=$(dirname "$OHOS_SDK_ROOT")
export HVIGOR_USER_HOME="$RUNNER_TEMP/private-hvigor-home"
export NODE_PATH="$HVIGOR_NODE_MODULES"
(
  cd harmonyos
  node "$HVIGOR_NODE_MODULES/@ohos/hvigor/bin/hvigor.js" --mode module \
    -p product=default -p module=entry@default -p buildMode=release assembleHap --no-daemon
)
hap="mobile-output/rustdesk-$VERSION-ohos-arm64-unsigned.hap"
cp harmonyos/entry/build/default/outputs/default/entry-default-unsigned.hap "$hap"
python3 .github/scripts/verify_mobile_artifacts.py ohos "$hap" \
  --version "$VERSION" --build-number "$BUILD_NUMBER" --application-id "$MOBILE_APPLICATION_ID" \
  --readelf "$native/llvm/bin/llvm-readelf" --nm "$native/llvm/bin/llvm-nm" \
  --metadata mobile-output/ohos-build.json
