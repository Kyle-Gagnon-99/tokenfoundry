//! The `reference` module in `ast` provides the definitions and types for references used in the abstract syntax tree (AST) representation of DTCG.
//! This includes JSON reference pointers, JSON reference pointer objects, and DTCG aliases.

use percent_encoding::{AsciiSet, CONTROLS, percent_decode_str, utf8_percent_encode};

use crate::utils::uri::{self, UriParseError};

/// Characters that must be percent-encoded inside a URI fragment (RFC 3986 allows `/`, `?`, `:` and `@` there).
const FRAGMENT_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'<')
    .add(b'>')
    .add(b'[')
    .add(b'\\')
    .add(b']')
    .add(b'^')
    .add(b'`')
    .add(b'{')
    .add(b'|')
    .add(b'}');

/// Represents the possible errors that can occur when parsing a JSON reference from a string.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum JsonReferenceParseError {
    /// Indicates that the provided string is not a valid URI.
    InvalidUri,
    /// The URI parser found a specific, user-fixable problem (Windows path, backslashes, bad percent-encoding).
    /// The inner error is kept so diagnostics can explain it instead of saying "invalid URI".
    Uri(UriParseError),
    /// Indicates that the provided string is not a valid JSON pointer.
    InvalidPointer,
    /// Indicates that the provided string has an unsupported URI scheme.
    UnsupportedScheme { scheme: String, raw: String },
    /// Indicates that the provided string is a non-portable path, which is not supported by the library (absolute paths, Windows paths, etc.).
    /// This error is returned when the path is not a relative file path, which could end up failing on different platforms or environments.
    /// While the file may exist on the current platform, it may not exist on other platforms or environments, leading to potential issues when
    /// sharing or distributing the DTCG document.
    NonPortablePath { path: String },
}

impl JsonReferenceParseError {
    /// A suggestion for the user, when the error has a known fix.
    /// References must be portable, so absolute paths are never suggested, only relative ones.
    pub fn hint(&self) -> Option<String> {
        match self {
            Self::Uri(UriParseError::RawWindowsAbsolutePath { .. }) => Some(
                "absolute paths are not portable; use a path relative to this file, such as \"./colors.json#/blue\" (use `/` as the separator)".into(),
            ),
            Self::Uri(UriParseError::InvalidBackslashes) => {
                Some("use `/` instead of `\\` as the path separator".into())
            }
            Self::Uri(UriParseError::InvalidWindowsUriStructure { .. }) => Some(
                "file URIs are not portable; use a path relative to this file instead".into(),
            ),
            Self::Uri(UriParseError::InvalidPercentEncoding { .. }) => Some(
                "`%` must be followed by two hex digits; write a literal percent sign as `%25`".into(),
            ),
            Self::NonPortablePath { .. } => Some(
                "use a path relative to this file, such as \"./colors.json#/blue\"".into(),
            ),
            _ => None,
        }
    }
}

/// Represents a JSON reference fragment, which is a string that points to a specific location within a JSON document.
/// The string is expected to follow RFC 6901 (JSON Pointer) syntax.
/// This does not contain the full URI, but only the fragment part that starts with `#` and is followed by a series of segments separated by `/`.
/// For the full URI, use [`JsonReference`](struct.JsonReference.html).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct JsonPointer {
    segments: Vec<String>,
}

impl JsonPointer {
    pub fn new(segments: Vec<String>) -> Self {
        Self { segments }
    }

    /// The unescaped reference tokens of the pointer, in order.
    pub fn segments(&self) -> &[String] {
        &self.segments
    }

    /// Converts the pointer into its RFC 6901 string form (e.g. `/a~1b/c`), escaping `~` as `~0` and `/` as `~1`.
    /// The root pointer is the empty string.
    pub fn to_pointer_string(&self) -> String {
        let mut out = String::new();

        for segment in &self.segments {
            out.push('/');
            out.push_str(&escape_segment(segment));
        }

        out
    }

    /// Converts the pointer into a URI fragment (`#` plus the percent-encoded pointer string).
    pub fn to_uri_fragment(&self) -> String {
        format!(
            "#{}",
            utf8_percent_encode(&self.to_pointer_string(), FRAGMENT_ENCODE_SET)
        )
    }

    /// Parses an RFC 6901 JSON Pointer string such as `/a~1b/c`. The empty string is the root.
    /// Every segment is unescaped (`~1` becomes `/`, then `~0` becomes `~`).
    pub fn parse_pointer(s: &str) -> Result<Self, JsonReferenceParseError> {
        if s.is_empty() {
            return Ok(Self::default());
        }

        let rest = s
            .strip_prefix('/')
            .ok_or(JsonReferenceParseError::InvalidPointer)?;

        let segments = rest
            .split('/')
            .map(unescape_segment)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self { segments })
    }

    /// Parses a URI fragment (the part after `#`, which is still percent-encoded) into a pointer.
    /// Percent-decoding happens first, then the pointer is parsed, as RFC 6901 section 6 requires.
    pub fn from_fragment(fragment: &str) -> Result<Self, JsonReferenceParseError> {
        let decoded = percent_decode_str(fragment)
            .decode_utf8()
            .map_err(|_| JsonReferenceParseError::InvalidPointer)?;

        Self::parse_pointer(&decoded)
    }

    /// Parses a same-document reference such as `#` (root) or `#/colors/blue`.
    pub fn from_str(s: &str) -> Result<Self, JsonReferenceParseError> {
        let fragment = s
            .strip_prefix('#')
            .ok_or(JsonReferenceParseError::InvalidPointer)?;

        Self::from_fragment(fragment)
    }

    /// Checks if the JSON pointer has no segments, indicating that it points to the root of the JSON document.
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    /// Returns the number of segments in the JSON pointer.
    pub fn len(&self) -> usize {
        self.segments.len()
    }

    /// Checks if the JSON pointer points to the root of the JSON document.
    pub fn is_root(&self) -> bool {
        self.is_empty()
    }

    /// Appends a new segment to the JSON pointer, effectively extending the path it represents.
    pub fn append(&mut self, segment: String) {
        self.segments.push(segment);
    }

    /// Removes and returns the last segment, if any.
    pub fn pop(&mut self) -> Option<String> {
        self.segments.pop()
    }

    /// Creates a `JsonPointer` from an iterator of segments.
    pub fn from_segments<I, S>(segments: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            segments: segments.into_iter().map(Into::into).collect(),
        }
    }
}

fn escape_segment(segment: &str) -> String {
    // `~` must be escaped first so the `~` in `~1` is not escaped again.
    segment.replace('~', "~0").replace('/', "~1")
}

fn unescape_segment(segment: &str) -> Result<String, JsonReferenceParseError> {
    let mut out = String::with_capacity(segment.len());
    let mut chars = segment.chars();

    while let Some(c) = chars.next() {
        if c != '~' {
            out.push(c);
            continue;
        }

        match chars.next() {
            Some('0') => out.push('~'),
            Some('1') => out.push('/'),
            _ => return Err(JsonReferenceParseError::InvalidPointer),
        }
    }

    Ok(out)
}

/// Contains the various external reference types (those that are not in the document itself)
/// This contains references to files (absolute or relative) or URLs (http, https, ftp, etc.) that point to external resources.
/// In tokenfoundry, we only support files, but this is here to allow for future expansion to support other types of external references.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DocumentReference {
    /// Relative file reference, which is an external reference to a file that is relative to the current document's location.
    RelativeFile(String),
    /// Any scheme URI reference, which is an external reference, but something that is not planned on being supported.
    UnsupportedScheme { scheme: String, raw: String },
}

impl DocumentReference {
    /// Checks if the external reference is supported by the tokenfoundry library. Currently, only file references (relative, absolute, and file URI) are supported.
    pub fn is_supported(&self) -> bool {
        match self {
            DocumentReference::RelativeFile(_) => true,
            DocumentReference::UnsupportedScheme { .. } => false,
        }
    }

    /// Checks if the external reference is a file reference (relative, absolute, or file URI).
    pub fn is_file_reference(&self) -> bool {
        matches!(self, Self::RelativeFile(_))
    }

    /// Returns the path of the external reference if it is a file reference (relative, absolute, or file URI). Otherwise, returns None.
    pub fn path(&self) -> Option<&str> {
        match self {
            DocumentReference::RelativeFile(path) => Some(path),
            _ => None,
        }
    }

    /// Checks if the scheme of the external reference is supported. Currently, only the "file" scheme is supported.
    pub fn scheme_is_supported(scheme: &str) -> bool {
        matches!(scheme, "file")
    }
}

/// Represents a JSON reference, which is a pointer to a specific location within a JSON document.
/// A JSON reference is composed of an optional external reference and a JSON pointer fragment.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JsonReference {
    /// The external reference part of the JSON reference, which can be a relative file, absolute file, or URI. If None, it indicates that the reference is internal to the document.
    document: Option<DocumentReference>,
    /// The JSON pointer fragment part of the JSON reference, which points to a specific location within the JSON document.
    fragment: JsonPointer,
}

impl JsonReference {
    pub fn new(document: Option<DocumentReference>, fragment: JsonPointer) -> Self {
        Self { document, fragment }
    }

    /// Returns the external reference part of the JSON reference, if any.
    pub fn document(&self) -> Option<&DocumentReference> {
        self.document.as_ref()
    }

    /// Returns the JSON pointer fragment part of the JSON reference.
    pub fn fragment(&self) -> &JsonPointer {
        &self.fragment
    }

    /// Parses a `$ref` string into an optional document and a JSON pointer fragment.
    ///
    /// - `#/a/b` and `#` are same-document references (`document` is `None`).
    /// - `colors.json#/a` is a relative file reference.
    /// - Anything non-portable (absolute paths, `file:` URIs, authorities) or with another scheme is an error.
    pub fn from_str<S: AsRef<str>>(s: S) -> Result<Self, JsonReferenceParseError> {
        let s = s.as_ref();

        let uri = uri::Uri::from_str(s).map_err(|e| match e {
            UriParseError::InvalidUri => JsonReferenceParseError::InvalidUri,
            other => JsonReferenceParseError::Uri(other),
        })?;

        if let Some(scheme) = &uri.scheme {
            return Err(if uri::UriScheme::is_supported_scheme(scheme) {
                JsonReferenceParseError::NonPortablePath {
                    path: s.to_string(),
                }
            } else {
                JsonReferenceParseError::UnsupportedScheme {
                    scheme: scheme.clone(),
                    raw: s.to_string(),
                }
            });
        }

        if uri.authority.is_some() || uri.path.starts_with('/') {
            return Err(JsonReferenceParseError::NonPortablePath {
                path: s.to_string(),
            });
        }

        // A query has no meaning for a local file reference.
        if uri.query.is_some() {
            return Err(JsonReferenceParseError::InvalidUri);
        }

        let fragment = match &uri.fragment {
            Some(f) => JsonPointer::from_fragment(f)?,
            None => JsonPointer::default(),
        };

        let document = if uri.path.is_empty() {
            None
        } else {
            let path = uri
                .decoded_path()
                .map_err(|_| JsonReferenceParseError::InvalidUri)?;
            Some(DocumentReference::RelativeFile(path))
        };

        Ok(Self { document, fragment })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> JsonPointer {
        JsonPointer::parse_pointer(s).unwrap()
    }

    #[test]
    fn root_pointer_is_empty_string() {
        assert!(p("").is_root());
        assert_eq!(p("").to_pointer_string(), "");
        assert_eq!(p("").to_uri_fragment(), "#");
    }

    #[test]
    fn splits_segments() {
        assert_eq!(p("/a/b/c").segments(), ["a", "b", "c"]);
    }

    #[test]
    fn slash_alone_is_one_empty_key() {
        assert_eq!(p("/").segments(), [""]);
        assert_eq!(p("//").segments(), ["", ""]);
    }

    #[test]
    fn unescapes_tilde_sequences() {
        assert_eq!(p("/a~1b").segments(), ["a/b"]);
        assert_eq!(p("/m~0n").segments(), ["m~n"]);
        // RFC 6901: `~01` is `~1`, not `/`.
        assert_eq!(p("/~01").segments(), ["~1"]);
    }

    #[test]
    fn rejects_bad_pointers() {
        for bad in ["a/b", "/a~", "/a~2", "/~x"] {
            assert_eq!(
                JsonPointer::parse_pointer(bad),
                Err(JsonReferenceParseError::InvalidPointer),
                "input: {bad}"
            );
        }
    }

    #[test]
    fn pointer_string_round_trips() {
        for s in ["", "/a", "/a~1b/m~0n", "/~01/~10", "/", "/ü/%"] {
            assert_eq!(p(s).to_pointer_string(), s);
        }
    }

    #[test]
    fn from_str_handles_root_and_paths() {
        assert!(JsonPointer::from_str("#").unwrap().is_root());
        assert_eq!(
            JsonPointer::from_str("#/a/b").unwrap().segments(),
            ["a", "b"]
        );
        assert!(JsonPointer::from_str("/a").is_err());
        assert!(JsonPointer::from_str("a").is_err());
        assert!(JsonPointer::from_str("#a").is_err());
    }

    #[test]
    fn fragment_is_percent_decoded_before_parsing() {
        assert_eq!(
            JsonPointer::from_str("#/a%20b").unwrap().segments(),
            ["a b"]
        );
        assert_eq!(
            JsonPointer::from_str("#/c%25d").unwrap().segments(),
            ["c%d"]
        );
        // %2F decodes to `/`, which then splits segments (RFC 6901 section 6 example).
        assert_eq!(
            JsonPointer::from_str("#/a%2Fb").unwrap().segments(),
            ["a", "b"]
        );
        assert_eq!(JsonPointer::from_str("#/a~1b").unwrap().segments(), ["a/b"]);
    }

    #[test]
    fn percent_decoding_to_invalid_utf8_is_an_error() {
        assert!(JsonPointer::from_str("#/%FF").is_err());
    }

    #[test]
    fn uri_fragment_round_trips() {
        for segs in [
            vec!["a b", "c%d"],
            vec!["a/b", "m~n"],
            vec!["ü", "[x]"],
            vec![],
        ] {
            let ptr = JsonPointer::from_segments(segs);
            assert_eq!(JsonPointer::from_str(&ptr.to_uri_fragment()).unwrap(), ptr);
        }
        assert_eq!(p("/a b").to_uri_fragment(), "#/a%20b");
        assert_eq!(p("/a~1b").to_uri_fragment(), "#/a~1b");
    }

    #[test]
    fn reference_same_document() {
        let r = JsonReference::from_str("#/colors/blue/$value").unwrap();
        assert!(r.document().is_none());
        assert_eq!(r.fragment().segments(), ["colors", "blue", "$value"]);

        let root = JsonReference::from_str("#").unwrap();
        assert!(root.document().is_none() && root.fragment().is_root());
    }

    #[test]
    fn reference_relative_file() {
        let r = JsonReference::from_str("tokens/colors.json#/blue").unwrap();
        assert_eq!(r.document().unwrap().path(), Some("tokens/colors.json"));
        assert_eq!(r.fragment().segments(), ["blue"]);

        let whole = JsonReference::from_str("colors.json").unwrap();
        assert_eq!(whole.document().unwrap().path(), Some("colors.json"));
        assert!(whole.fragment().is_root());

        let spaced = JsonReference::from_str("my%20tokens.json#/a").unwrap();
        assert_eq!(spaced.document().unwrap().path(), Some("my tokens.json"));
    }

    #[test]
    fn reference_rejects_non_portable_and_unsupported() {
        for s in ["/abs/colors.json#/a", "file:///a.json#/a", "//host/a.json"] {
            assert!(
                matches!(
                    JsonReference::from_str(s),
                    Err(JsonReferenceParseError::NonPortablePath { .. })
                ),
                "input: {s}"
            );
        }
        assert!(matches!(
            JsonReference::from_str("https://example.com/a.json#/a"),
            Err(JsonReferenceParseError::UnsupportedScheme { .. })
        ));
    }

    #[test]
    fn reference_rejects_malformed_input() {
        assert_eq!(
            JsonReference::from_str("a.json#a"),
            Err(JsonReferenceParseError::InvalidPointer)
        );
        assert_eq!(
            JsonReference::from_str("a.json#/a~2"),
            Err(JsonReferenceParseError::InvalidPointer)
        );
        assert!(JsonReference::from_str("a.json?x=1#/a").is_err());
    }

    #[test]
    fn raw_windows_path_keeps_its_specific_error() {
        let err = JsonReference::from_str("C:\\tokens\\colors.json#/blue").unwrap_err();
        assert!(matches!(
            err,
            JsonReferenceParseError::Uri(UriParseError::RawWindowsAbsolutePath { .. })
        ));
        assert!(err.hint().unwrap().contains("relative"));
        // Drive letter with forward slashes is still a raw absolute path.
        assert!(matches!(
            JsonReference::from_str("C:/tokens/colors.json").unwrap_err(),
            JsonReferenceParseError::Uri(UriParseError::RawWindowsAbsolutePath { .. })
        ));
    }

    #[test]
    fn backslashes_keep_their_specific_error() {
        let err = JsonReference::from_str("tokens\\colors.json#/blue").unwrap_err();
        assert_eq!(
            err,
            JsonReferenceParseError::Uri(UriParseError::InvalidBackslashes)
        );
        assert!(err.hint().unwrap().contains('/'));
        // A backslash hidden as %5C in the path is caught too.
        assert_eq!(
            JsonReference::from_str("tokens%5Ccolors.json").unwrap_err(),
            JsonReferenceParseError::Uri(UriParseError::InvalidBackslashes)
        );
    }

    #[test]
    fn malformed_windows_file_uri_keeps_its_specific_error() {
        let err = JsonReference::from_str("file://C:/tokens/colors.json").unwrap_err();
        assert!(matches!(
            err,
            JsonReferenceParseError::Uri(UriParseError::InvalidWindowsUriStructure { .. })
        ));
        assert!(err.hint().is_some());
    }

    #[test]
    fn bad_percent_encoding_keeps_its_specific_error() {
        for s in ["my%2.json", "a.json#/b%zz"] {
            let err = JsonReference::from_str(s).unwrap_err();
            assert!(
                matches!(
                    err,
                    JsonReferenceParseError::Uri(UriParseError::InvalidPercentEncoding { .. })
                ),
                "input: {s}"
            );
            assert!(err.hint().is_some());
        }
    }

    #[test]
    fn hints_exist_only_for_fixable_errors() {
        let non_portable = JsonReference::from_str("/abs/a.json").unwrap_err();
        assert!(non_portable.hint().unwrap().contains("relative"));
        assert!(JsonReferenceParseError::InvalidPointer.hint().is_none());
    }
}
