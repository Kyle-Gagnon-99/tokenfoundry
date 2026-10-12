//! The `uri` module provides utility functions and types for working with URIs. It contains a Regex pattern for splitting URIs into their components.

use std::sync::OnceLock;

use percent_encoding::percent_decode_str;
use regex::Regex;

/// Represents the scheme of a URI, which indicates the protocol or method used to access the resource identified by the URI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UriScheme(String);

impl UriScheme {
    /// Creates a new `UriScheme` from a string representation of the scheme.
    /// If the scheme is "file", it returns `UriScheme::File`. Otherwise, it returns `UriScheme::Other` with the provided scheme string.
    pub fn new(scheme: &str) -> Self {
        UriScheme(scheme.to_string())
    }

    /// Checks if the given scheme string is a valid URI scheme according to RFC 3986.
    pub fn is_valid_scheme(scheme: &str) -> bool {
        let mut chars = scheme.chars();

        match chars.next() {
            Some(c) if c.is_ascii_alphabetic() => {}
            _ => return false,
        }

        chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
    }

    /// Checks if the given scheme string is a supported URI scheme.
    pub fn is_supported_scheme(scheme: &str) -> bool {
        matches!(scheme, "file")
    }
}

/// Represents errors that can occur while parsing a URI.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum UriParseError {
    InvalidUri,
    /// Used to indicate that the URI contains backslashes, which are not valid in URIs. This is a common mistake when users accidentally
    /// use absolute Windows paths in their JSON references. This error is returned to provide a more helpful error message to the user.
    InvalidBackslashes,
    /// Indicates that the URI contains invalid percent-encoding, which is not allowed in URIs. This error is returned when the URI contains
    /// percent-encoded characters that are not valid UTF-8 sequences, which can lead to issues when parsing or interpreting the URI.
    InvalidPercentEncoding {
        raw: String,
    },
    /// Indicates that the URI does not conform to the expected format, which is defined by RFC 3986, but instead is a raw Windows absolute path (e.g., "C:\path\to\file"). This error is returned to provide a more helpful error message to the user, as raw Windows absolute paths are not valid URIs and can lead to issues when parsing or interpreting the URI.
    /// This is a common mistake when users accidentally use absolute Windows paths in their JSON references. This error is returned to provide a more helpful error message to the user.
    RawWindowsAbsolutePath {
        path: String,
        suggested_uri: String,
    },
    InvalidWindowsUriStructure {
        raw: String,
        suggested_uri: String,
    },
}

/// Represents a URI (Uniform Resource Identifier) with its components: scheme, authority, path, query, and fragment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Uri {
    /// The scheme component of the URI (e.g., "http", "https", "ftp").
    pub scheme: Option<String>,
    /// The authority component of the URI (e.g., "user:pass@host:port").
    pub authority: Option<String>,
    /// The path component of the URI (e.g., "/path/to/resource").
    pub path: String,
    /// The query component of the URI (e.g., "?key=value&key2=value2").
    pub query: Option<String>,
    /// The fragment component of the URI (e.g., "#section1").
    pub fragment: Option<String>,
}

impl Uri {
    pub fn new(
        scheme: Option<String>,
        authority: Option<String>,
        path: String,
        query: Option<String>,
        fragment: Option<String>,
    ) -> Self {
        Self {
            scheme,
            authority,
            path,
            query,
            fragment,
        }
    }

    pub fn decoded_path(&self) -> Result<String, UriParseError> {
        Self::decode_path(&self.path)
    }

    fn decode_path(path: &str) -> Result<String, UriParseError> {
        validate_percent_encoding(path)?;

        percent_decode_str(path)
            .decode_utf8()
            .map(|s| s.into_owned())
            .map_err(|_| UriParseError::InvalidPercentEncoding {
                raw: path.to_string(),
            })
    }

    /// Parses a URI string into its components using the RFC 3986 regex pattern.
    pub fn from_str(uri_str: &str) -> Result<Self, UriParseError> {
        // First, detect any common mistakes
        detect_common_mistakes(uri_str)?;

        // Use the regex to capture the components of the URI string
        let capture = rfc3986_regex()
            .captures(uri_str)
            .ok_or(UriParseError::InvalidUri)?;

        // If the regex does not match any components, return an error indicating that the URI is invalid
        if capture.len() == 0 {
            return Err(UriParseError::InvalidUri);
        }

        let scheme = capture.name("scheme").map(|m| m.as_str().to_string());
        let authority = capture.name("authority").map(|m| m.as_str().to_string());
        let path = capture
            .name("path")
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();
        let query = capture.name("query").map(|m| m.as_str().to_string());
        let fragment = capture.name("fragment").map(|m| m.as_str().to_string());

        // Post-process validate: Decode components to inspect content
        // We look inside the decoded path to see if any Windows were hidden as %5C (backslash)
        if let Some(scheme) = &scheme {
            if !UriScheme::is_valid_scheme(scheme) {
                return Err(UriParseError::InvalidUri);
            }
        }

        validate_percent_encoding(&path)?;
        if let Some(query) = &query {
            validate_percent_encoding(query)?;
        }

        if let Some(fragment) = &fragment {
            validate_percent_encoding(fragment)?;
        }

        let decoded_path = Self::decode_path(&path)?;
        if decoded_path.contains('\\') {
            return Err(UriParseError::InvalidBackslashes);
        }

        Ok(Self {
            scheme,
            authority,
            path,
            query,
            fragment,
        })
    }
}

/// Returns a reference to the compiled RFC 3986 regex pattern for parsing URIs.
fn rfc3986_regex() -> &'static Regex {
    static RFC3986_REGEX: OnceLock<Regex> = OnceLock::new();
    RFC3986_REGEX.get_or_init(|| {
        Regex::new(r#"^((?P<scheme>[^:/?#]+):)?(//(?P<authority>[^/?#]*))?(?P<path>[^?#]*)(\?(?P<query>[^#]*))?(#(?P<fragment>.*))?$"#).unwrap()
    })
}

/// Checks if the given path is a raw Windows absolute path
fn is_raw_windows_absolute_path(path: &str) -> bool {
    let bytes = path.as_bytes();

    // Guard against paths that are too short to be valid Windows absolute paths (e.g., "C:")
    if bytes.len() < 3 {
        return false;
    }

    // Check for drive letter (e.g., "C:") followed by a colon, and then a backslash or forward slash
    let has_drive = bytes[0].is_ascii_alphabetic();
    let has_colon = bytes[1] == b':';
    let has_slash = bytes[2] == b'\\' || bytes[2] == b'/';
    has_drive && has_colon && has_slash
}

/// Checks if the given path is a Windows drive prefix (e.g., "C:"). This is used to identify paths that are not portable across different platforms.
fn is_windows_drive_prefix(path: &str) -> bool {
    let bytes = path.as_bytes();

    // Guard against paths that are too short to be valid Windows drive prefixes (e.g., "C")
    if bytes.len() < 2 {
        return false;
    }

    // Check for drive letter (e.g., "C:") followed by a colon
    let has_drive = bytes[0].is_ascii_alphabetic();
    let has_colon = bytes[1] == b':';
    has_drive && has_colon
}

/// Detects common mistakes in the input URI string and returns a corresponding `UriParseError` if any issues are found.
/// This function checks for raw Windows absolute paths, backslashes in the URI, and invalid Windows URI structures. If any of these issues are detected, a corresponding `UriParseError` is returned.
fn detect_common_mistakes(uri_str: &str) -> Result<(), UriParseError> {
    // First, do an eager check to see if the string is in a raw Windows absolute path format
    // We do this to provide a more helpful error message to the user, as raw Windows absolute paths are not valid URIs and can lead to issues
    // but a diagnostic of "Invalid URI" is not helpful to the user if we can inspect and help
    let path_part = uri_str.split('#').next().unwrap_or(uri_str);
    if is_raw_windows_absolute_path(path_part) {
        let normalized_path = path_part.replace('\\', "/");
        let suggested_uri = format!("file:///{}", normalized_path);
        return Err(UriParseError::RawWindowsAbsolutePath {
            path: path_part.to_string(),
            suggested_uri,
        });
    }

    // Next, check if the string contains any back slashes, which are not valid in URIs. We are doing a check to help
    // assist the users who may accidentally use Windows paths in their JSON references and that we can provide a more helpful error message
    if uri_str.contains('\\') {
        return Err(UriParseError::InvalidBackslashes);
    }

    if path_part.starts_with("file://") && !path_part.starts_with("file:///") {
        let remainder = &path_part["file://".len()..];
        if is_windows_drive_prefix(remainder) {
            let suggested = path_part.replace("file://", "file:///");
            return Err(UriParseError::InvalidWindowsUriStructure {
                raw: path_part.to_string(),
                suggested_uri: suggested,
            });
        }
    }

    Ok(())
}

/// Validates percent-encoding in the given raw URI string. This function checks for valid percent-encoded sequences (e.g., "%20")
/// and returns a `UriParseError::InvalidPercentEncoding` if any invalid sequences are found.
fn validate_percent_encoding(raw: &str) -> Result<(), UriParseError> {
    let bytes = raw.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len()
                || !bytes[i + 1].is_ascii_hexdigit()
                || !bytes[i + 2].is_ascii_hexdigit()
            {
                return Err(UriParseError::InvalidPercentEncoding {
                    raw: raw.to_string(),
                });
            }

            i += 3; // Skip the percent-encoded sequence
        } else {
            i += 1;
        }
    }

    Ok(())
}
