//! Task `C19`: the request-target transport.
//!
//! Contract decision `D09` says transport parsing is not defined by a `map()`
//! codec alone, so the web's own GET behaviour was captured
//! (`fixtures/oracle/query-transport.jsonl`, `Plug.Conn.Query.decode/1` behind
//! the router) and this module implements it: percent escapes, `+` as a space, a
//! malformed escape kept literally, the fragment dropped before the query is
//! read, repeated scalars taking the last value, `[]` as a list and `[name]` as
//! a map, and the three list/scalar/map conflict orders the fixture pins.
//!
//! The transport is deliberately *separate from map normalization*: this module
//! answers the decoded query map and the route outcome, and hands the map to
//! [`crate::decode_page_params`] to produce the page. Two rules of the frozen
//! transport are narrower than Plug's full generality and are stated rather than
//! invented:
//!
//! * one nesting level: `a[b][c]` is not a shape the frozen transport carries,
//!   so the remainder of the first bracket is treated as one name;
//! * a keystroke-shaped key (`chords[`, `chords]`) is a scalar whose name is the
//!   literal text, which is what the captured cases show.
//!
//! The origin allowlist and the length limit are **not** decided here: `D09`
//! leaves both to an approved configuration, so [`UrlPolicy`] is supplied by the
//! caller ([`crate::UrlPolicy`]) and no hostname or limit is invented in Rust.
//! Nothing in this module performs I/O, and no import ever fetches a URL.

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use crate::{CoreError, PageState, decode_page_params};

/// The one path the page route serves; every other path is a 404.
const PAGE_ROUTE: &str = "/";

/// The field name the transport's failures belong to.
const TARGET_FIELD: &str = "url";

/// One request target: its path and its raw query, with the fragment removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestTarget {
    /// The path of the target, `/` for the page route.
    pub path: String,
    /// The raw query, without the leading `?` and without any fragment.
    pub query: String,
}

/// One decoded query value: the shapes the pinned decoder produces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryValue {
    /// A single scalar value.
    Scalar(String),
    /// A value the query repeated or opened with `[]`.
    List(Vec<String>),
    /// A value the query nested with `[name]`.
    Map(BTreeMap<String, Self>),
}

/// A decoded query, by key.
pub type QueryParams = BTreeMap<String, QueryValue>;

/// What importing one request target produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportedPage {
    /// The page route answered the page its query names.
    Route(PageState),
    /// The target is not a route this transport serves.
    NotFound,
}

/// The validated share/import configuration, supplied by the caller.
///
/// Every field is a decision this module must not make: the scheme, the exact
/// host and the exact path come from the approved share configuration, and
/// `max_bytes` is the approved input cap. A URL is accepted only when it matches
/// all of them, so a lookalike suffix host, a port-carrying authority, another
/// scheme or another path is refused rather than compared loosely.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UrlPolicy {
    /// The one scheme the policy allows, e.g. `https`.
    pub scheme: String,
    /// The one authority the policy allows, compared exactly.
    pub host: String,
    /// The one path the policy allows, compared exactly.
    pub path: String,
    /// The largest accepted input, in bytes.
    pub max_bytes: usize,
}

/// Split a request target into its path and its raw query.
///
/// The fragment is dropped first: a browser never sends it, and the pinned
/// capture shows an unencoded `#` cutting the query (`chords=Cmaj#frag` reaches
/// the codec as `Cmaj`).
pub fn parse_request_target(target: &str) -> RequestTarget {
    let without_fragment = target.split('#').next().unwrap_or_default();
    let (path, query) = match without_fragment.split_once('?') {
        Some((path, query)) => (path, query),
        None => (without_fragment, ""),
    };
    RequestTarget {
        path: path.to_owned(),
        query: query.to_owned(),
    }
}

/// Decode one raw query the way the pinned transport does.
///
/// # Errors
///
/// [`CoreError::InvalidUrl`] when a percent escape decodes to bytes that are not
/// UTF-8, which is exactly where the baseline raised its own query error. A
/// malformed escape (`%ZZ`, a lone `%`) is *kept literally* instead, as the
/// capture shows; it is not an error.
pub fn decode_query(query: &str) -> Result<QueryParams, CoreError> {
    let mut params = QueryParams::default();
    if query.is_empty() {
        return Ok(params);
    }

    for pair in query.split('&') {
        let (raw_key, raw_value) = match pair.split_once('=') {
            Some((key, value)) => (key, value),
            None => (pair, ""),
        };
        let key = percent_decode(raw_key)?;
        let value = percent_decode(raw_value)?;
        insert_pair(&mut params, &key, value);
    }

    Ok(params)
}

/// Import one request target: its path decides the route, its query the page.
///
/// # Errors
///
/// [`CoreError::InvalidUrl`] when the query cannot be decoded.
pub fn import_request_target(target: &str) -> Result<ImportedPage, CoreError> {
    let target = parse_request_target(target);
    if target.path != PAGE_ROUTE {
        return Ok(ImportedPage::NotFound);
    }
    let params = decode_query(&target.query)?;
    Ok(ImportedPage::Route(decode_page_params(&json_params(
        &params,
    ))))
}

/// Import one absolute URL against the supplied policy.
///
/// The URL must be the policy's scheme, its exact authority and its exact path;
/// credentials in the authority are refused, and the whole input must fit the
/// policy's cap. The import is pure: it never fetches anything.
///
/// # Errors
///
/// * [`CoreError::InputTooLarge`] when the input is longer than the cap.
/// * [`CoreError::InvalidUrl`] when the input is not an absolute URL or its
///   query cannot be decoded.
/// * [`CoreError::UnsupportedOrigin`] when the scheme, the authority or the path
///   is outside the policy, or the authority carries credentials.
pub fn import_absolute_url(url: &str, policy: &UrlPolicy) -> Result<ImportedPage, CoreError> {
    if url.len() > policy.max_bytes {
        return Err(CoreError::input_too_large(TARGET_FIELD));
    }

    let (scheme, rest) = url
        .split_once("://")
        .ok_or_else(|| CoreError::invalid_url(TARGET_FIELD))?;
    if scheme != policy.scheme {
        return Err(CoreError::unsupported_origin(TARGET_FIELD));
    }

    let without_fragment = rest.split('#').next().unwrap_or_default();
    let (authority_and_path, query) = match without_fragment.split_once('?') {
        Some((head, query)) => (head, query),
        None => (without_fragment, ""),
    };
    let (authority, path) = match authority_and_path.split_once('/') {
        Some((authority, path)) => (authority, format!("/{path}")),
        None => (authority_and_path, PAGE_ROUTE.to_owned()),
    };

    if authority.contains('@') || authority != policy.host || path != policy.path {
        return Err(CoreError::unsupported_origin(TARGET_FIELD));
    }

    let params = decode_query(query)?;
    Ok(ImportedPage::Route(decode_page_params(&json_params(
        &params,
    ))))
}

/// Render decoded query parameters as a JSON object.
///
/// This is the bridge into the map codec: every shape is preserved, so a value
/// the query nested or repeated reaches the page codec as the wrong kind it is
/// and is ignored field-locally, exactly as the baseline ignored it.
pub fn query_params_to_json(params: &QueryParams) -> Value {
    Value::Object(json_params(params))
}

fn json_params(params: &QueryParams) -> Map<String, Value> {
    let mut object = Map::new();
    for (key, value) in params {
        object.insert(key.clone(), query_value_to_json(value));
    }
    object
}

fn query_value_to_json(value: &QueryValue) -> Value {
    match value {
        QueryValue::Scalar(scalar) => Value::from(scalar.clone()),
        QueryValue::List(items) => {
            Value::Array(items.iter().map(|item| Value::from(item.clone())).collect())
        }
        QueryValue::Map(entries) => {
            let mut object = Map::new();
            for (key, entry) in entries {
                object.insert(key.clone(), query_value_to_json(entry));
            }
            Value::Object(object)
        }
    }
}

/// The shape a query key asks for.
enum KeyShape<'a> {
    /// A plain key: the last scalar wins, replacing whatever was there.
    Scalar,
    /// `base[]`: append to `base`'s list, replacing any other shape.
    List(&'a str),
    /// `base[name]`: insert into `base`'s map, replacing any other shape.
    Map(&'a str, &'a str),
}

/// Classify one query key by its bracket form.
fn key_shape(key: &str) -> KeyShape<'_> {
    let Some(open) = key.find('[') else {
        return KeyShape::Scalar;
    };
    // `chords[` and `chords]` are literal names, not bracket forms.
    let Some(inner) = key.strip_suffix(']').and_then(|head| head.get(open..)) else {
        return KeyShape::Scalar;
    };
    let base = key.get(..open).unwrap_or_default();
    let name = inner.get(1..).unwrap_or_default();
    if name.is_empty() {
        KeyShape::List(base)
    } else {
        KeyShape::Map(base, name)
    }
}

/// Insert one decoded pair, with the conflict rules the capture pins.
fn insert_pair(params: &mut QueryParams, key: &str, value: String) {
    match key_shape(key) {
        KeyShape::Scalar => {
            params.insert(key.to_owned(), QueryValue::Scalar(value));
        }
        KeyShape::List(base) => {
            if let Some(QueryValue::List(items)) = params.get_mut(base) {
                items.push(value);
            } else {
                params.insert(base.to_owned(), QueryValue::List(vec![value]));
            }
        }
        KeyShape::Map(base, name) => {
            if let Some(QueryValue::Map(entries)) = params.get_mut(base) {
                entries.insert(name.to_owned(), QueryValue::Scalar(value));
            } else {
                let mut entries = BTreeMap::new();
                entries.insert(name.to_owned(), QueryValue::Scalar(value));
                params.insert(base.to_owned(), QueryValue::Map(entries));
            }
        }
    }
}

/// Decode percent escapes and `+` the way the pinned transport does.
///
/// `arithmetic_side_effects` is allowed: the nibble combination cannot overflow
/// a byte (`0xF << 4 | 0xF` is `0xFF`) and every step is bounded by the input's
/// own length.
#[allow(clippy::arithmetic_side_effects)]
fn percent_decode(input: &str) -> Result<String, CoreError> {
    let bytes = input.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while let Some(&byte) = bytes.get(index) {
        match byte {
            b'+' => {
                decoded.push(b' ');
                index += 1;
            }
            b'%' => {
                if let (Some(high), Some(low)) =
                    (nibble(bytes.get(index + 1)), nibble(bytes.get(index + 2)))
                {
                    decoded.push((high << 4) | low);
                    index += 3;
                } else {
                    // A malformed escape is kept literally, as the capture shows.
                    decoded.push(b'%');
                    index += 1;
                }
            }
            byte => {
                decoded.push(byte);
                index += 1;
            }
        }
    }

    String::from_utf8(decoded).map_err(|_| CoreError::invalid_url(TARGET_FIELD))
}

/// The value of one hexadecimal digit.
///
/// `arithmetic_side_effects` is allowed: a digit's value is at most 15 and the
/// `+ 10` bias cannot overflow a byte.
#[allow(clippy::arithmetic_side_effects)]
fn nibble(byte: Option<&u8>) -> Option<u8> {
    match *byte? {
        digit @ b'0'..=b'9' => Some(digit - b'0'),
        letter @ b'a'..=b'f' => Some(letter - b'a' + 10),
        letter @ b'A'..=b'F' => Some(letter - b'A' + 10),
        _ => None,
    }
}
