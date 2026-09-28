# Toolchain Ledger

P0 task `C00` / delivery task `D00`. Every pin below records a value that was
actually observed in the build environment, plus the official source that makes
it compatible. Nothing here is inferred from a device brand, a blog post or a
release announcement alone.

Environment inspected: Linux 6.18 (x86_64), 16 cores, no `sudo`, no `/dev/kvm`.

## Verified in this environment

| Dep | Pin | Verified how |
|---|---|---|
| Rust | `1.98.1` | `rustup-init` stable install; `rustc --version`, `cargo --version`; pinned in `rust-toolchain.toml` |
| Rust components | `rustfmt`, `clippy` | `rust-toolchain.toml`; exercised by the workspace commands |
| Rust targets | `aarch64-linux-android`, `x86_64-linux-android` | pinned in `rust-toolchain.toml` so cross builds cannot pick a float |
| UniFFI | `0.32.2` | highest stable release on crates.io at 2026-09-28; one pin for runtime, macros and generator CLI |
| Elixir / Erlang/OTP (oracle only) | `1.19.5-otp-27` / `OTP 27.3.4.18` | `mise install` using the web repository's own `mise.toml`; `elixir --version` |
| JDK | Oracle OpenJDK `21.0.2` | `mise install java@21`; AGP 9.4 requires JDK >= 17 |
| Android Gradle plugin | `9.4.1` | highest stable on Google's Maven at 2026-09-28 |
| Gradle | `9.8.0` | AGP 9.4 requires Gradle >= 9.6.0; Gradle's matrix supports JVM 17–27 |
| Kotlin | `2.4.20` | highest stable on Maven Central; Gradle is tested with Kotlin 2.0.0–2.4.20-RC2 |
| Compose BOM | `2026.09.00` | highest stable on Google's Maven |
| Android SDK | `cmdline-tools;23.0` (build `16111833`), `platforms;android-37`, `build-tools;36.0.0`, `platform-tools` | `sdkmanager --install` + `--list_installed` |
| Android NDK | `28.2.13676358` | the AGP 9.4 default revision; installed through `sdkmanager` |
| compileSdk / targetSdk | `37` | highest stable platform offered by `sdkmanager`; AGP 9.4 supports up to API 37 |
| minSdk | `26` | plan proposal (DEC-10). Requires user agreement: it sets the device floor |
| cargo-ndk | not selected yet | introduced in task `C05`, where the cross-build script is written |
| ABIs | not measured yet | `arm64-v8a` phone and `x86_64` emulator are the plan's target; the phone ABI must be read with `adb shell getprop ro.product.cpu.abilist` |

Official compatibility sources used for the pins:

- AGP 9.4 release notes: Gradle >= 9.6.0, SDK Build Tools >= 36.0.0, NDK default
  `28.2.13676358`, JDK >= 17, maximum API level 37.
- Gradle compatibility matrix: Gradle runs on JVM 17–27 and is tested with Kotlin
  2.0.0 through 2.4.20-RC2 and AGP 9.0 through 9.5.0-alpha02.
- crates.io release history for `uniffi` (`0.32.2`).

## Environment limitations (blocked, not deferred silently)

| Limitation | Consequence | Status |
|---|---|---|
| `/dev/kvm` is absent; the Android emulator cannot be hardware accelerated | The emulator half of the P1 gate cannot be produced on this machine | Blocker recorded; needs either host-level KVM access or real-device-only evidence |
| No `adb` device is attached to this environment | Device API/ABI/fingerprint evidence (`adb shell getprop …`) cannot be collected here | Needs the phone reachable, or the user running the three `getprop` commands |
| No Android emulator or system image installed | No instrumentation run in P0 | P1 concern; the P0 gate has no APK |
| `sudo` is unavailable | No system-level tool installation | Everything above was installed in user space; nothing needs root |
| Android SDK licenses were accepted non-interactively (`sdkmanager --licenses`) as part of this bootstrap | Licenses for `platform-tools`, `platforms;android-37`, `build-tools;36.0.0`, `ndk;28.2.13676358` are accepted on this machine | Disclosed; the acceptance log is local, not committed |

## Oracle prerequisite evidence

The Elixir baseline is the oracle, so it must actually run:

- `git archive` of `2daa8c665efa268942dda352691f39d78db42512` extracted to a
  scratch directory outside both repositories; the working tree of
  `Ironjanowar/fretboard` was not modified or switched.
- `MIX_ENV=test mix test` in that scratch checkout: **981 tests, 0 failures**.
- This is host-runtime evidence for the exporter, not a claim about the frozen
  fixtures: the fixtures themselves are produced and reviewed by task `C01`.

## Updating a pin

A version change is a decision, not a maintenance chore: record it in
`docs/decisions.md` with the official compatibility reference, update this
ledger, and regenerate `Cargo.lock` (or the Gradle lock/verification metadata)
in the same commit.
