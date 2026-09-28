# Validation and Phase Gates

Acceptance is evidence, not intent. Each row names the artifact a gate produces
and the checks that must pass before the next dependent phase starts. A blocked
check is recorded as blocked; it is never replaced by a plausible-looking result,
and no gate is marked green from a plan document.

| Phase | Scope | Gate evidence | Status |
|---|---|---|---|
| P0 | Toolchains locked, contracts frozen, oracle fixtures exported, provenance recorded | Toolchain ledger; frozen fixtures with manifest and hashes; typed state contract tests green; independent review | in progress (C00 done, C01 fixtures frozen, C02 open) |
| P1 | Minimal real vertical slice: Compose → generated UniFFI → Rust calculation, offline APK | Signed APK published as a release candidate, installed by hand on the phone, airplane mode, native loading proven; the app displays its own API level and ABIs for the device record; emulator coverage is out of scope and disclosed (no `/dev/kvm`) | not started |
| P2 | All chord qualities, all instrument visualizers, duplicate identity | APK gate: quality selector, visualizers, chip interval behavior, duplicate highlight/remove, rotation | not started |
| P3 | Absolute tuning edits and the fretted analyzer | APK gate: four fretted analyzers, tuning Apply/Cancel, Low G vs Standard, inversions and missing-note policy | not started |
| P4 | Piano analyzer and instrument boundaries | APK gate: all 36 keys, edge hit tests, octaves, cross-kind reset | not started |
| P5 | Scales, keys, multi-key coverage, progressions | APK gate: key/progression groups and previews, multi-key membership, stale-result protection | not started |
| P6 | URL import/share, durable session, lifecycle | APK gate: confirmed origin, manual paste, unchanged-web round trip, force-stop/process recreation, same-signer update | not started |
| P7 | Parity, artifact and release audit | Published/redownloaded AAR and APK with checksums, locked runtime dependencies, both ABIs, accessibility/offline/security review | not started |

## Per-task checks

Every task must show, from this repository:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
python3 -m unittest discover -s scripts/tests
```

plus its own focused command, the observed failing assertion before
implementation, and an independent review of the diff.

## Honesty rules

- A missing tool, an absent dependency or a corrupt fixture is blocked setup, not
  a red test.
- Golden expectations are never regenerated from the implementation under test.
- A device is not "verified" from a build succeeding; a device check names the
  command and its output.
- Limitations that arrive with a release (an unsupported environment, an
  unaligned library, a missing ABI) are disclosed in the release evidence.
