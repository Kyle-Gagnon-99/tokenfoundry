//! The `errors` module defines custom error types for the token shift library,
//! which can be used to represent various error conditions that may occur during
//! token parsing, transformation, and generation. These error types can provide
//! more specific and informative error messages to help users understand and resolve issues with their tokens.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DiagnosticCode {
    Parse(ParseDiagnosticCode),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ParseDiagnosticCode {
    JSONSerializationError { message: String },
    InvalidDTCGAlias { alias: String },
    InvalidJSONReferenceObject { object: String },
    MissingRequiredField { field: String },
    InvalidTokenType { found: String },
    InvalidExtendsReference,
    InvalidExtendsJSONReference,
    GroupCircularReference { path: String },
    UnresolvableJSONReference { reference: String },
    InvalidGroupExtensionTarget { reference: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticPhase {
    Parse,
    Analyze,
    Validate,
    Resolve,
    Emit,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Diagnostic {
    pub severity: Severity,
    pub phase: DiagnosticPhase,
    pub code: DiagnosticCode,

    pub help: Option<String>,
}

#[derive(Debug, Default)]
pub struct DiagnosticReporter {
    diagnostics: Vec<Diagnostic>,
}

impl DiagnosticReporter {
    pub fn push(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }

    pub fn finish(self) -> DiagnosticReport {
        DiagnosticReport {
            diagnostics: self.diagnostics,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DiagnosticReport {
    pub diagnostics: Vec<Diagnostic>,
}

impl DiagnosticReport {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }
}

pub struct DiagnosticScope<'a> {
    reporter: &'a mut DiagnosticReporter,
    phase: DiagnosticPhase,
}

impl<'a> DiagnosticScope<'a> {
    pub fn error(&mut self, code: DiagnosticCode) -> DiagnosticBuilder<'_, 'a> {
        DiagnosticBuilder::new(self, Severity::Error, code)
    }

    pub fn warning(&mut self, code: DiagnosticCode) -> DiagnosticBuilder<'_, 'a> {
        DiagnosticBuilder::new(self, Severity::Warning, code)
    }

    pub fn new(reporter: &'a mut DiagnosticReporter, phase: DiagnosticPhase) -> Self {
        Self { reporter, phase }
    }
}

pub struct DiagnosticBuilder<'scope, 'reporter> {
    scope: &'scope mut DiagnosticScope<'reporter>,
    diagnostic: Diagnostic,
}

impl<'scope, 'reporter> DiagnosticBuilder<'scope, 'reporter> {
    fn new(
        scope: &'scope mut DiagnosticScope<'reporter>,
        severity: Severity,
        code: DiagnosticCode,
    ) -> Self {
        Self {
            diagnostic: Diagnostic {
                severity,
                phase: scope.phase,
                code,
                help: None,
            },
            scope,
        }
    }

    pub fn help(mut self, help: impl Into<String>) -> Self {
        self.diagnostic.help = Some(help.into());
        self
    }

    pub fn emit(self) {
        self.scope.reporter.push(self.diagnostic);
    }
}

pub trait DiagnosticInfo {
    fn code(&self) -> &'static str;
    fn message(&self) -> String;
    fn help(&self) -> Option<String> {
        None
    }
}

impl DiagnosticInfo for ParseDiagnosticCode {
    fn code(&self) -> &'static str {
        match self {
            Self::InvalidDTCGAlias { .. } => "parse.invalid_dtgc_alias",
            Self::InvalidJSONReferenceObject { .. } => "parse.invalid_json_reference_object",
            Self::MissingRequiredField { .. } => "parse.missing_required_field",
            Self::InvalidTokenType { .. } => "parse.invalid_token_type",
            Self::InvalidExtendsReference => "parse.invalid_extends_reference",
            Self::InvalidExtendsJSONReference => "parse.invalid_extends_json_reference",
            Self::JSONSerializationError { .. } => "parse.json_serialization_error",
            Self::GroupCircularReference { .. } => "parse.group_circular_reference",
            Self::UnresolvableJSONReference { .. } => "parse.unresolvable_json_reference",
            Self::InvalidGroupExtensionTarget { .. } => "parse.invalid_group_extension_target",
        }
    }

    fn message(&self) -> String {
        match self {
            Self::InvalidDTCGAlias { alias } => {
                format!("Invalid DTCG alias: '{}'", alias)
            }
            Self::InvalidJSONReferenceObject { object } => {
                format!(
                    "Invalid JsonRefObject format: expected only a '$ref' property, but found additional properties: {}",
                    object
                )
            }
            Self::MissingRequiredField { field } => {
                format!("Missing required field: '{}'", field)
            }
            Self::InvalidTokenType { found } => {
                format!(
                    "Invalid token type: expected a valid token type but found '{}'",
                    found
                )
            }
            Self::InvalidExtendsReference => {
                format!("Invalid '$extends' reference: Must be a DTCG alias")
            }
            Self::InvalidExtendsJSONReference => {
                format!(
                    "Group extends reference must use either the '$extends' directive with a DTCG alias or a JSON Reference Object with a '$ref' property"
                )
            }
            Self::JSONSerializationError { message } => {
                format!("JSON serialization error: {}", message)
            }
            Self::GroupCircularReference { path } => {
                format!("Circular reference detected in group hierarchy: {}", path)
            }
            Self::UnresolvableJSONReference { reference } => {
                format!("Unresolvable JSON reference: '{}'", reference)
            }
            Self::InvalidGroupExtensionTarget { reference } => {
                format!("Invalid group extension target: '{}'", reference)
            }
        }
    }

    fn help(&self) -> Option<String> {
        match self {
            Self::InvalidDTCGAlias { .. } => Some(format!(
                "DTCG aliases must be surrounded by curly braces and must in in dot notation to the token they refer to, e.g. '{{foo.bar}}'"
            )),
            Self::InvalidJSONReferenceObject { .. } => Some(format!(
                "A valid JSON Reference Object must have exactly one property named '$ref' whose value is a JSON Pointer string, e.g. {{ \"$ref\": \"#/path/to/token\" }}"
            )),
            Self::InvalidTokenType { .. } => Some(format!(
                "A valid token type must be one of the predefined types, e.g. 'color', 'size', 'font', etc."
            )),
            Self::InvalidExtendsReference => Some(format!(
                "A valid '$extends' reference must be a DTCG alias, e.g. '{{foo.bar}}'"
            )),
            Self::GroupCircularReference { .. } => {
                Some(format!("Circular reference detected in group hierarchy"))
            }
            Self::InvalidGroupExtensionTarget { .. } => Some(format!(
                "A valid group extension target must reference a group, e.g. '{{foo.bar}}'"
            )),
            _ => None,
        }
    }
}
