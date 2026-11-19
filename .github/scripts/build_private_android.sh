#!/usr/bin/env bash
set -euo pipefail

: "${VERSION:?}" "${BUILD_NUMBER:?}" "${MOBILE_APPLICATION_ID:?}"
: "${ANDROID_HOME:?}" "${ANDROID_NDK_HOME:?}" "${VCPKG_ROOT:?}" "${RUNNER_TEMP:?}"
python3 .github/scripts/mobile_build_config.py

bash flutter/build_android_deps.sh arm64-v8a
cargo ndk --platform 21 --target aarch64-linux-android --bindgen \
  build --locked --release --features flutter,hwcodec

native_dir=flutter/android/app/src/main/jniLibs/arm64-v8a
ndk_tools="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64"
mkdir -p "$native_dir" mobile-output
cp target/aarch64-linux-android/release/liblibrustdesk.so "$native_dir/librustdesk.so"
cp "$ndk_tools/sysroot/usr/lib/aarch64-linux-android/libc++_shared.so" "$native_dir/"
"$ndk_tools/bin/llvm-strip" --strip-unneeded "$native_dir/librustdesk.so"

export ORG_GRADLE_PROJECT_rustdeskApplicationId="$MOBILE_APPLICATION_ID"
(
  cd flutter
  flutter build apk --release --target-platform android-arm64 \
    --build-name "$VERSION" --build-number "$BUILD_NUMBER"
)
apk="mobile-output/rustdesk-$VERSION-android-arm64-unsigned.apk"
cp flutter/build/app/outputs/flutter-apk/app-release.apk "$apk"
python3 .github/scripts/verify_mobile_artifacts.py android "$apk" \
  --version "$VERSION" --build-number "$BUILD_NUMBER" \
  --application-id "$MOBILE_APPLICATION_ID" \
  --aapt2 "$ANDROID_HOME/build-tools/36.0.0/aapt2" \
  --metadata mobile-output/android-build.json
