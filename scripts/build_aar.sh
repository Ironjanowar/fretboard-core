#!/usr/bin/env bash
# Build, package and check the Android engine artifact (task C05).
#
# The AAR is a build output: this script is the committed record of how it is
# produced, never the artifact itself. Every tool is pinned (docs/toolchains.md).
#
#   scripts/build_aar.sh [output-directory]
#
# Requirements, from the pinned toolchain:
#   * cargo, rustc 1.98.1 (rust-toolchain.toml) and the pinned UniFFI 0.32.2
#   * cargo-ndk and ANDROID_HOME pointing at the pinned SDK/NDK
#   * JDK 21 (JAVA_HOME) and the pinned Kotlin compiler (KOTLIN_HOME)
#   * python3 for scripts/check_aar.py
#
# The artifact is `fretboard-engine-<version>.aar` plus its published sidecar
# `artifact-manifest.json`. Both carry the same frozen metadata; the sidecar is
# checked against the artifact bytes by scripts/check_aar.py.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# The exported API revision and the snapshot schema are read from the frozen
# contract, so the artifact metadata cannot drift from the fixture (CORE-D08).
CONTRACT_FILE="${REPO_ROOT}/fixtures/contract/api-v7.json"
OUT_DIR="${1:-${REPO_ROOT}/dist}"
NDK_VERSION="28.2.13676358"
UNIFFI_RUNTIME_DEPENDENCY="net.java.dev.jna:jna:5.17.0"
BINDING_PACKAGE="dev.ironjanowar.fretboard.core"
MIN_SDK="29"
ABI="arm64-v8a"
LICENSES='["MIT"]'

: "${ANDROID_HOME:?ANDROID_HOME must point at the pinned Android SDK}"
: "${JAVA_HOME:?JAVA_HOME must point at the pinned JDK 21}"
: "${KOTLIN_HOME:?KOTLIN_HOME must point at the pinned Kotlin compiler}"
JNA_JAR="${JNA_JAR:-${REPO_ROOT}/../../../tools/maven/jna-5.17.0.jar}"

cd "${REPO_ROOT}"
ARTIFACT_VERSION="$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -1)"
SOURCE_COMMIT="$(git rev-parse HEAD)"
WORK="$(mktemp -d)"
trap 'rm -rf "${WORK}"' EXIT

echo "==> cross-building ${ABI} (NDK ${NDK_VERSION})"
ANDROID_NDK_HOME="${ANDROID_HOME}/ndk/${NDK_VERSION}" \
  cargo ndk -t "${ABI}" -o "${WORK}/jniLibs" build -p fretboard-mobile-ffi --locked --release

echo "==> generating the Kotlin bindings from the host library metadata"
cargo build -p fretboard-mobile-ffi --locked
cargo run -q -p fretboard-bindgen --bin uniffi-bindgen --locked -- generate \
  --library target/debug/libfretboard_mobile_ffi.so \
  --language kotlin \
  --out-dir "${WORK}/kotlin"

echo "==> compiling the generated bindings (kotlinc, JNA on the classpath)"
mkdir -p "${WORK}/classes"
"${KOTLIN_HOME}/bin/kotlinc" \
  -classpath "${JNA_JAR}" \
  -d "${WORK}/classes" \
  $(find "${WORK}/kotlin" -name '*.kt')
(cd "${WORK}/classes" && "${JAVA_HOME}/bin/jar" cf "${WORK}/classes.jar" .)

echo "==> assembling the AAR"
mkdir -p "${WORK}/aar/META-INF/fretboard-engine"
cp "${WORK}/classes.jar" "${WORK}/aar/classes.jar"
mkdir -p "${WORK}/aar/jni/${ABI}"
cp "${WORK}/jniLibs/${ABI}"/libfretboard_mobile_ffi.so "${WORK}/aar/jni/${ABI}/"
printf '' > "${WORK}/aar/R.txt"
cat > "${WORK}/aar/AndroidManifest.xml" <<'MANIFEST'
<manifest xmlns:android="http://schemas.android.com/apk/res/android" package="dev.ironjanowar.fretboard.core" />
MANIFEST
cat > "${WORK}/aar/proguard.txt" <<'RULES'
# Consumer rules of the Fretboard engine AAR.
-keep class dev.ironjanowar.fretboard.core.** { *; }
-keep class com.sun.jna.** { *; }
-dontwarn java.awt.**
RULES

RUSTC_VERSION="$(rustc --version | cut -d' ' -f2)"
CARGO_NDK_VERSION="$(cargo ndk --version | cut -d' ' -f2)"
KOTLIN_VERSION="$("${KOTLIN_HOME}/bin/kotlinc" -version 2>&1 | sed -n 's/.*kotlinc-jvm \([0-9.]*\).*/\1/p')"
JAVA_VERSION="$("${JAVA_HOME}/bin/java" -version 2>&1 | sed -n '1s/.*"\(.*\)".*/\1/p')"

python3 - "${WORK}/aar/META-INF/fretboard-engine/metadata.json" "${CONTRACT_FILE}" <<PY
import json, sys
CONTRACT = json.load(open(sys.argv[2], encoding="utf-8"))
metadata = {
    "artifact_version": "${ARTIFACT_VERSION}",
    "source_commit": "${SOURCE_COMMIT}",
    "tool_versions": {
        "rust": "${RUSTC_VERSION}",
        "uniffi": "0.32.2",
        "cargo_ndk": "${CARGO_NDK_VERSION}",
        "ndk": "${NDK_VERSION}",
        "kotlin": "${KOTLIN_VERSION}",
        "java": "${JAVA_VERSION}",
    },
    "uniffi_runtime_dependency": "${UNIFFI_RUNTIME_DEPENDENCY}",
    "api_version": CONTRACT["api_version"],
    "snapshot_schema_version": CONTRACT["snapshot_schema_version"],
    "binding_package": "${BINDING_PACKAGE}",
    "abis": ["${ABI}"],
    "min_sdk": int("${MIN_SDK}"),
    "licenses": json.loads('${LICENSES}'),
}
with open(sys.argv[1], "w", encoding="utf-8") as handle:
    handle.write(json.dumps(metadata, sort_keys=True, indent=2) + "\n")
PY

mkdir -p "${OUT_DIR}"
AAR_PATH="${OUT_DIR}/fretboard-engine-${ARTIFACT_VERSION}.aar"
# Pack with Python's zipfile: the environment has no `zip` binary, and a sorted
# entry order keeps the archive reproducible for the same inputs.
python3 - "${WORK}/aar" "${AAR_PATH}" <<'PACK'
import os, sys, zipfile
root, target = sys.argv[1], sys.argv[2]
entries = []
for directory, _, files in os.walk(root):
    for name in files:
        path = os.path.join(directory, name)
        entries.append((os.path.relpath(path, root), path))
with zipfile.ZipFile(target, "w", zipfile.ZIP_DEFLATED) as archive:
    for relative, path in sorted(entries):
        archive.write(path, relative)
PACK
cp "${WORK}/aar/META-INF/fretboard-engine/metadata.json" "${OUT_DIR}/artifact-manifest.json"

echo "==> checking the artifact"
python3 "${REPO_ROOT}/scripts/check_aar.py" \
  --aar "${AAR_PATH}" \
  --manifest "${OUT_DIR}/artifact-manifest.json"

echo "==> ${AAR_PATH}"
sha256sum "${AAR_PATH}" "${OUT_DIR}/artifact-manifest.json"
