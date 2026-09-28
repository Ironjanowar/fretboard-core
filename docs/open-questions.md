# Open questions and autonomous decisions

This file is the running record for work done while the user was away. It lists
the decisions taken without a live confirmation, why, and the questions that
still need the user's word. Decisions that change a contract are recorded in
`docs/decisions.md`; this file carries the doubts and the sequencing choices.

Status: work in progress — the target is the first signed APK the user can
install (the plan's P1 gate).

## Decisions taken while the user was away

| # | Decision | Why |
|---|---|---|
| A1 | `C06` (the 47-quality chord catalog) and `C07` (the instrument/preset catalog) were implemented **ahead of the P2 gate** | The user asked to advance the plan instead of waiting; both are pure domain work with no dependency on P1 approval. The plan's P2 gate still owns their chip/UI acceptance (`Contract.D01` stays open for that). |
| A2 | Every change is delivered as a PR into `p1/primitives` (stacked), and the PRs are merged without a live confirmation | AGENTS.md asks for branch + PR + approval; the user said "mergea esta PR y continúa" and then "no voy a estar para confirmar absolutamente nada, continúa el plan". `main` is untouched: PR #5 (`p1/primitives` → `main`) still needs the user. |
| A3 | The core↔wire rule of `CORE-D06` was applied to the adapter (`Db` is rejected) | User decision: the port does what the current project does, and section 2 of the contract already says URL parsing is sharp-only. |
| A4 | The C05 artifacts another session left uncommitted (`scripts/tests/test_check_aar.py`, the `cargo-ndk` row in `docs/toolchains.md`) are adopted and finished as task `C05` | They are on the critical path to the APK and were abandoned mid-flight; the checker test defines a frozen contract, and `scripts/check_aar.py` itself was never written. |
| A5 | The release keystore is generated in this environment, outside every repository, per `DEC-08` | The plan's recommendation for the open signing decision; the user must copy the two files somewhere safe. Path and password are never printed in the chat or committed. |

## Questions for the user

1. **Capability flag set.** `docs/contracts.md` says the flag set is "frozen
   together with the adapter API in task `C04`", but `fixtures/contract/api-v1.json`
   still carries `"capabilities": {}` and no entry point reports capabilities.
   *Recommendation:* freeze it in `C21` (the complete FFI), where `catalogs()`
   lands, and correct the sentence in `docs/contracts.md` now. Needs your word.
2. **Flat-root label.** The pinned baseline's `chord_label("Db", :major)` is
   `Dbmaj`; the port can only produce `C#maj`. With `CORE-D06` a flat root is
   rejected on every wire surface, so the difference is now unreachable.
   *Recommendation:* no deviation record is needed; say the word if you want one
   anyway for the record.
3. **Signing custody.** The agent-generated keystore (A5) is a real release key.
   You will need to keep a copy outside this environment; losing it means the
   next APK cannot update the installed one.
4. **ABIs.** The delivered APK is `arm64-v8a` only (DEC-02/DEC-03: no emulator
   evidence; your phone's ABI is confirmed by the app itself). Say the word if
   you also want `x86_64` for an emulator.
5. **Share origin (`DEC-07`)** stays open and only blocks P6. The interim APK
   must disable sharing with an explicit explanation rather than invent a host.

## Blocked setup to report honestly

- If Gradle cannot resolve AGP/Kotlin/Compose artifacts from the network, the
  APK build is *blocked setup*, not a red test, and will be reported as such
  with the actual command output.
- `java` is available through `mise`, but no version is set in a bare shell;
  the Android build needs the pinned JDK 21 (`docs/toolchains.md`).
