//! Portable music domain for Fretboard.
//!
//! This crate owns the musical rules, the catalogs, the canonical page state,
//! the state transitions and the URL/snapshot codecs for every Fretboard
//! client. It is deliberately pure: no UniFFI, Android, JNI, network, storage,
//! clock, randomness or rendering dependency, and no dependency at all.
//!
//! Baseline behavior is frozen from the Elixir web application at commit
//! `2daa8c665efa268942dda352691f39d78db42512`; the web application remains
//! unchanged and is the migration oracle.
//!
//! Status: the workspace skeleton exists as of the P0 bootstrap task. Musical
//! modules (pitch primitives, catalogs, state, codecs, evaluation) arrive in
//! later tasks, each of them tests first.

#![forbid(unsafe_code)]
