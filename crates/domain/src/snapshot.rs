//! Task `C20`: the snapshot envelope.
//!
//! The contract fixes the snapshot as `{"schema_version": 1, "page": …}`, where
//! the `page` object is exactly the typed page state's own strict serialised
//! shape (`fixtures/contract/snapshot-v1.json` is the frozen example, pinned by
//! `tests/state_contract.rs`). This module owns the envelope only:
//!
//! * [`encode_snapshot`] validates the page first, so an inconsistent state is
//!   never persisted, and writes the frozen envelope;
//! * [`decode_snapshot`] reads the version, refuses a version this build cannot
//!   read, and otherwise runs the page through the same strict, validating
//!   reader the rest of the crate uses.
//!
//! Two deliberate decisions:
//!
//! * **The bytes are never consumed.** `decode_snapshot` takes `&[u8]` and never
//!   rewrites what it was given, so a caller whose snapshot was refused still
//!   holds the original bytes for recovery.
//! * **No migration is claimed that cannot be performed.** No native `v0`
//!   snapshot format exists (`C20`: "do not invent old-schema migration
//!   success"), so [`SNAPSHOT_MIGRATIONS`] is empty and any version other than
//!   the current one is [`CoreError::UnsupportedSchemaVersion`] rather than a
//!   silent upgrade. A real old format is added here together with a case
//!   captured from it.
//!
//! Only the committed page is persisted: the page's own strict schema carries
//! colours, results, drafts, timestamps or platform values nowhere, so there is
//! nothing else for this envelope to hold. Atomic storage and boot arbitration
//! belong to the platform (`C20`), not to the engine: this module is pure.

use serde::{Deserialize, Serialize};

use crate::{CoreError, PageState, validate_state};

/// The snapshot schema version this build writes and reads.
pub const CURRENT_SNAPSHOT_SCHEMA: u32 = 1;

/// The field name a snapshot failure belongs to.
const SNAPSHOT_FIELD: &str = "snapshot";

/// One explicit migration step between two snapshot schema versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapshotMigration {
    /// The version the step reads.
    pub from: u32,
    /// The version the step writes.
    pub to: u32,
}

/// The explicit migration registry, in application order.
///
/// Empty on purpose: there is no native `v0` snapshot to migrate from, and a
/// step is added only together with the real format it reads and a captured case
/// that proves it. Until then any other version is refused.
pub const SNAPSHOT_MIGRATIONS: &[SnapshotMigration] = &[];

/// The frozen envelope: the schema version and the committed page.
///
/// Strict on the way in — an unknown field is an error — so a snapshot written
/// by another build can never quietly extend the durable format.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotEnvelope {
    schema_version: u32,
    page: PageState,
}

/// Encode one page state as a snapshot string.
///
/// The output is byte-stable: the envelope's fields are written in the frozen
/// order and no value depends on the environment.
///
/// # Errors
///
/// The page's own validation codes when the state is inconsistent, so an
/// invalid page is never persisted.
pub fn encode_snapshot(state: &PageState) -> Result<String, CoreError> {
    validate_state(state)?;
    let envelope = SnapshotEnvelope {
        schema_version: CURRENT_SNAPSHOT_SCHEMA,
        page: state.clone(),
    };
    serde_json::to_string(&envelope).map_err(|_| CoreError::invalid_snapshot(SNAPSHOT_FIELD))
}

/// Decode one snapshot into a validated page state.
///
/// The input is borrowed, never consumed: a refused snapshot leaves the caller
/// holding the original bytes.
///
/// # Errors
///
/// * [`CoreError::UnsupportedSchemaVersion`] when the envelope declares a
///   version this build cannot read and no registered migration step reaches it.
/// * [`CoreError::InvalidSnapshot`] when the input is not JSON, the envelope is
///   missing its version or page, the version is not an integer, an unknown
///   field appears, a value has the wrong type, a required key is absent, an
///   identifier is outside the catalog, or the page is structurally
///   inconsistent.
pub fn decode_snapshot(bytes: &[u8]) -> Result<PageState, CoreError> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| CoreError::invalid_snapshot(SNAPSHOT_FIELD))?;

    // The version is read first, so a snapshot from a later build is reported as
    // an unsupported version rather than as a corrupt one, whatever its page
    // looks like.
    let version = value
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        .and_then(|version| u32::try_from(version).ok())
        .ok_or_else(|| CoreError::invalid_snapshot(SNAPSHOT_FIELD))?;
    if version != CURRENT_SNAPSHOT_SCHEMA {
        return Err(CoreError::unsupported_schema_version(SNAPSHOT_FIELD));
    }

    // The strict envelope read rejects an unknown field, a wrong type, an absent
    // required key and an identifier outside the catalog; the page's own reader
    // also refuses a state the engine's validation would reject.
    let envelope: SnapshotEnvelope =
        serde_json::from_value(value).map_err(|_| CoreError::invalid_snapshot(SNAPSHOT_FIELD))?;
    validate_state(&envelope.page)?;

    Ok(envelope.page)
}
