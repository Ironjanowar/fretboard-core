//! The stable error contract of the domain crate.
//!
//! `02-core-contract.md` section 8 freezes the error code list: every fallible
//! entry point reports one of these codes plus an optional field name, never a
//! panic, never a raw stack trace and never an invented numeric limit.
//!
//! The variant name *is* the stable code, so [`CoreError::code`] and the
//! derived `Debug` rendering agree by construction; the C02 contract tests read
//! the code from the value's `Debug` rendering and the later FFI adapter reads
//! it from [`CoreError::code`].
//!
//! `UnsupportedCapability` is a documented addition to the plan's frozen list
//! (`CORE-D05` in `docs/decisions.md`, made in C03): a catalog capability this
//! engine build does not implement yet must report an explicit capability error
//! instead of masquerading as an invalid action or answering with a plausible
//! but wrong musical result.

use std::fmt;

/// A stable failure of a domain operation.
///
/// Each variant carries an optional field name that names the offending part of
/// the input (`"tuning"`, `"selection"`, `"reference"`, …). The variant name is
/// the wire code; the field name is diagnostic detail and is never parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    /// The value is structurally inconsistent: for example a fretted state that
    /// carries the piano, a wrong pitch count for the instrument, duplicate
    /// string indices, or a highlight that does not occur in the chords.
    InvalidState(Option<&'static str>),
    /// The string is not one of the stable catalog identifiers.
    UnknownIdentifier(Option<&'static str>),
    /// A numeric value is outside the range its position allows: a fret above
    /// 24, an open pitch above 127, a piano key outside 48..=83, a string index
    /// the instrument does not have.
    OutOfRange(Option<&'static str>),
    /// The action cannot apply to the current state.
    InvalidAction(Option<&'static str>),
    /// The input is not a syntactically valid URL.
    InvalidUrl(Option<&'static str>),
    /// The URL origin is outside the configured share policy.
    UnsupportedOrigin(Option<&'static str>),
    /// The input is larger than the accepted import limit.
    InputTooLarge(Option<&'static str>),
    /// The snapshot is corrupt, truncated or of the wrong shape.
    InvalidSnapshot(Option<&'static str>),
    /// The snapshot declares a schema version this build cannot read.
    UnsupportedSchemaVersion(Option<&'static str>),
    /// The requested feature is a known catalog capability this engine build
    /// does not implement yet. A client renders an explicit pending state for
    /// it rather than treating the absent result as an empty musical answer.
    UnsupportedCapability(Option<&'static str>),
}

impl CoreError {
    /// The stable code of this error: the variant name.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidState(_) => "InvalidState",
            Self::UnknownIdentifier(_) => "UnknownIdentifier",
            Self::OutOfRange(_) => "OutOfRange",
            Self::InvalidAction(_) => "InvalidAction",
            Self::InvalidUrl(_) => "InvalidUrl",
            Self::UnsupportedOrigin(_) => "UnsupportedOrigin",
            Self::InputTooLarge(_) => "InputTooLarge",
            Self::InvalidSnapshot(_) => "InvalidSnapshot",
            Self::UnsupportedSchemaVersion(_) => "UnsupportedSchemaVersion",
            Self::UnsupportedCapability(_) => "UnsupportedCapability",
        }
    }

    /// The name of the field the failure belongs to, when the operation can
    /// name one.
    pub const fn field(&self) -> Option<&'static str> {
        match self {
            Self::InvalidState(field)
            | Self::UnknownIdentifier(field)
            | Self::OutOfRange(field)
            | Self::InvalidAction(field)
            | Self::InvalidUrl(field)
            | Self::UnsupportedOrigin(field)
            | Self::InputTooLarge(field)
            | Self::InvalidSnapshot(field)
            | Self::UnsupportedSchemaVersion(field)
            | Self::UnsupportedCapability(field) => *field,
        }
    }

    /// An invalid state, naming the field it belongs to.
    pub const fn invalid_state(field: &'static str) -> Self {
        Self::InvalidState(Some(field))
    }

    /// An unknown catalog identifier, naming the field it belongs to.
    pub const fn unknown_identifier(field: &'static str) -> Self {
        Self::UnknownIdentifier(Some(field))
    }

    /// A value outside its allowed range, naming the field it belongs to.
    pub const fn out_of_range(field: &'static str) -> Self {
        Self::OutOfRange(Some(field))
    }

    /// An action that cannot apply to the current state.
    pub const fn invalid_action(field: &'static str) -> Self {
        Self::InvalidAction(Some(field))
    }

    /// A URL that is not syntactically valid.
    pub const fn invalid_url(field: &'static str) -> Self {
        Self::InvalidUrl(Some(field))
    }

    /// A URL whose origin the share policy does not allow.
    pub const fn unsupported_origin(field: &'static str) -> Self {
        Self::UnsupportedOrigin(Some(field))
    }

    /// An input larger than the accepted import limit.
    pub const fn input_too_large(field: &'static str) -> Self {
        Self::InputTooLarge(Some(field))
    }

    /// A snapshot that cannot be read.
    pub const fn invalid_snapshot(field: &'static str) -> Self {
        Self::InvalidSnapshot(Some(field))
    }

    /// A snapshot of a schema version this build does not support.
    pub const fn unsupported_schema_version(field: &'static str) -> Self {
        Self::UnsupportedSchemaVersion(Some(field))
    }

    /// A known catalog capability this engine build does not implement yet.
    pub const fn unsupported_capability(field: &'static str) -> Self {
        Self::UnsupportedCapability(Some(field))
    }

    /// The English sentence this code stands for; used for user-facing text and
    /// for `serde` diagnostics.
    const fn message(&self) -> &'static str {
        match self {
            Self::InvalidState(_) => "invalid page state",
            Self::UnknownIdentifier(_) => "unknown catalog identifier",
            Self::OutOfRange(_) => "value out of range",
            Self::InvalidAction(_) => "invalid action for this state",
            Self::InvalidUrl(_) => "invalid URL",
            Self::UnsupportedOrigin(_) => "unsupported share origin",
            Self::InputTooLarge(_) => "input is larger than the accepted limit",
            Self::InvalidSnapshot(_) => "invalid snapshot",
            Self::UnsupportedSchemaVersion(_) => "unsupported snapshot schema version",
            Self::UnsupportedCapability(_) => "capability not implemented in this build",
        }
    }
}

impl fmt::Display for CoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.field() {
            Some(field) => write!(formatter, "{}: {field}", self.message()),
            None => formatter.write_str(self.message()),
        }
    }
}

impl std::error::Error for CoreError {}
