//! The `stroke_style` module defines the stroke style token

use super::RefAliasOrLiteral;

use crate::{
    errors::DiagnosticCode,
    ir::{RefOr, TryFromJson, require_enum_string_with_mapping, require_object},
    token::token_types::{composite::parse_composite_token_field, dimension::DimensionTokenValue},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineCap {
    Round,
    Butt,
    Square,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DashArrayValue(Vec<RefAliasOrLiteral<DimensionTokenValue>>);

impl<'a> TryFromJson<'a> for DashArrayValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        match value {
            serde_json::Value::Array(arr) => {
                let mut result = Vec::new();
                for (index, item) in arr.iter().enumerate() {
                    let item_path = format!("{}/{}", path, index);
                    match parse_composite_token_field::<DimensionTokenValue>(ctx, &item_path, item)
                    {
                        Some(value) => result.push(value),
                        None => {
                            ctx.push_to_errors(
                                DiagnosticCode::InvalidPropertyType,
                                "Expected a dimension value, reference, or alias for dashArray"
                                    .to_string(),
                                item_path.into(),
                            );
                            return None;
                        }
                    }
                }
                Some(DashArrayValue(result))
            }
            _ => {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidPropertyType,
                    "Expected an array for dashArray".to_string(),
                    path.into(),
                );
                None
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StrokeStyleObjectValue {
    pub dash_array: RefOr<DashArrayValue>,
    pub line_cap: RefOr<LineCap>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StrokeStyleStringValue {
    Solid,
    Dashed,
    Dotted,
    Groove,
    Ridge,
    Outset,
    Inset,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StrokeStyleTokenValue {
    Object(StrokeStyleObjectValue),
    String(StrokeStyleStringValue),
}

impl<'a> TryFromJson<'a> for LineCap {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        require_enum_string_with_mapping(
            ctx,
            path,
            "lineCap",
            value,
            |s| match s {
                "round" => Some(LineCap::Round),
                "butt" => Some(LineCap::Butt),
                "square" => Some(LineCap::Square),
                _ => None,
            },
            "round, butt, square",
        )
    }
}

impl<'a> TryFromJson<'a> for StrokeStyleStringValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        require_enum_string_with_mapping(
            ctx,
            path,
            "strokeStyle",
            value,
            |s| match s {
                "solid" => Some(StrokeStyleStringValue::Solid),
                "dashed" => Some(StrokeStyleStringValue::Dashed),
                "dotted" => Some(StrokeStyleStringValue::Dotted),
                "groove" => Some(StrokeStyleStringValue::Groove),
                "ridge" => Some(StrokeStyleStringValue::Ridge),
                "outset" => Some(StrokeStyleStringValue::Outset),
                "inset" => Some(StrokeStyleStringValue::Inset),
                _ => None,
            },
            "solid, dashed, dotted, groove, ridge, outset, inset",
        )
    }
}

impl<'a> TryFromJson<'a> for StrokeStyleObjectValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        let obj = require_object(ctx, path, value, "stroke style")?;
        let dash_array = obj.required_field::<RefOr<DashArrayValue>>(ctx, path, "dashArray");
        let line_cap = obj.required_field::<RefOr<LineCap>>(ctx, path, "lineCap");

        match (dash_array, line_cap) {
            (Some(dash_array), Some(line_cap)) => Some(StrokeStyleObjectValue {
                dash_array,
                line_cap,
            }),
            _ => {
                ctx.push_to_errors(
                    DiagnosticCode::MissingRequiredProperty,
                    "Missing required properties for stroke style object. Both dashArray and lineCap are required.".to_string(),
                    path.into(),
                );
                None
            }
        }
    }
}

impl<'a> TryFromJson<'a> for StrokeStyleTokenValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        match value {
            serde_json::Value::String(_) => StrokeStyleStringValue::try_from_json(ctx, path, value)
                .map(StrokeStyleTokenValue::String),
            serde_json::Value::Object(_) => StrokeStyleObjectValue::try_from_json(ctx, path, value)
                .map(StrokeStyleTokenValue::Object),
            _ => {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidPropertyType,
                    "Expected a string or an object for strokeStyle".to_string(),
                    path.into(),
                );
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        FileFormat, ParserContext,
        ir::{JsonPointer, JsonRef, RefOr, TokenAlias},
        token::token_types::composite::{RefAliasOrLiteral, parse_composite_token_field},
    };
    use serde_json::json;

    fn parser_context() -> ParserContext {
        ParserContext::new("tests.json".into(), FileFormat::Json, String::new())
    }

    #[test]
    fn parses_line_cap_values() {
        let mut ctx = parser_context();

        let round = LineCap::try_from_json(&mut ctx, "/token/lineCap", &json!("round"));
        let butt = LineCap::try_from_json(&mut ctx, "/token/lineCap", &json!("butt"));
        let square = LineCap::try_from_json(&mut ctx, "/token/lineCap", &json!("square"));

        assert_eq!(round, Some(LineCap::Round));
        assert_eq!(butt, Some(LineCap::Butt));
        assert_eq!(square, Some(LineCap::Square));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn rejects_invalid_line_cap_value() {
        let mut ctx = parser_context();

        let parsed = LineCap::try_from_json(&mut ctx, "/token/lineCap", &json!("flat"));

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidEnumValue);
        assert_eq!(ctx.errors[0].path, "/token/lineCap");
    }

    #[test]
    fn parses_stroke_style_string_values() {
        let mut ctx = parser_context();

        let solid = StrokeStyleStringValue::try_from_json(&mut ctx, "/token", &json!("solid"));
        let dashed = StrokeStyleStringValue::try_from_json(&mut ctx, "/token", &json!("dashed"));
        let inset = StrokeStyleStringValue::try_from_json(&mut ctx, "/token", &json!("inset"));

        assert_eq!(solid, Some(StrokeStyleStringValue::Solid));
        assert_eq!(dashed, Some(StrokeStyleStringValue::Dashed));
        assert_eq!(inset, Some(StrokeStyleStringValue::Inset));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn rejects_invalid_stroke_style_string_value() {
        let mut ctx = parser_context();

        let parsed = StrokeStyleStringValue::try_from_json(&mut ctx, "/token", &json!("double"));

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidEnumValue);
        assert_eq!(ctx.errors[0].path, "/token");
    }

    #[test]
    fn parses_composite_token_field_as_literal_alias_and_reference() {
        let mut ctx = parser_context();

        let literal = parse_composite_token_field::<DimensionTokenValue>(
            &mut ctx,
            "/token/0",
            &json!({
                "value": 2,
                "unit": "px"
            }),
        );
        let alias = parse_composite_token_field::<DimensionTokenValue>(
            &mut ctx,
            "/token/1",
            &json!("{spacing.small}"),
        );
        let reference = parse_composite_token_field::<DimensionTokenValue>(
            &mut ctx,
            "/token/2",
            &json!({ "$ref": "#/spacing/small" }),
        );

        match literal {
            Some(RefAliasOrLiteral::Literal(DimensionTokenValue { value, unit })) => {
                assert!(matches!(value, RefOr::Literal(_)));
                assert!(matches!(unit, RefOr::Literal(_)));
            }
            _ => panic!("expected literal dimension token value"),
        }
        assert_eq!(
            alias,
            Some(RefAliasOrLiteral::Alias(
                TokenAlias::from_dtcg_alias("{spacing.small}").unwrap()
            ))
        );
        assert_eq!(
            reference,
            Some(RefAliasOrLiteral::Ref(JsonRef::new_local_pointer(
                "#/spacing/small".into(),
                JsonPointer::from("#/spacing/small"),
            )))
        );
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn parses_dash_array_with_literal_alias_and_reference_entries() {
        let mut ctx = parser_context();
        let value = json!([
            { "value": 1, "unit": "px" },
            "{spacing.medium}",
            { "$ref": "#/spacing/large" }
        ]);

        let parsed = DashArrayValue::try_from_json(&mut ctx, "/token/dashArray", &value).unwrap();

        assert_eq!(parsed.0.len(), 3);
        match &parsed.0[0] {
            RefAliasOrLiteral::Literal(DimensionTokenValue { value, unit }) => {
                assert!(matches!(value, RefOr::Literal(_)));
                assert!(matches!(unit, RefOr::Literal(_)));
            }
            _ => panic!("expected first dashArray item to be a literal dimension"),
        }
        assert_eq!(
            parsed.0[1],
            RefAliasOrLiteral::Alias(TokenAlias::from_dtcg_alias("{spacing.medium}").unwrap())
        );
        assert_eq!(
            parsed.0[2],
            RefAliasOrLiteral::Ref(JsonRef::new_local_pointer(
                "#/spacing/large".into(),
                JsonPointer::from("#/spacing/large"),
            ))
        );
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn rejects_non_array_dash_array() {
        let mut ctx = parser_context();

        let parsed = DashArrayValue::try_from_json(&mut ctx, "/token/dashArray", &json!("bad"));

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyType);
        assert_eq!(ctx.errors[0].path, "/token/dashArray");
    }

    #[test]
    fn rejects_invalid_dash_array_item() {
        let mut ctx = parser_context();

        let parsed = DashArrayValue::try_from_json(
            &mut ctx,
            "/token/dashArray",
            &json!([
                { "value": 1, "unit": "px" },
                true
            ]),
        );

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 2);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyType);
        assert_eq!(ctx.errors[0].path, "/token/dashArray/1");
        assert_eq!(ctx.errors[1].code, DiagnosticCode::InvalidPropertyType);
        assert_eq!(ctx.errors[1].path, "/token/dashArray/1");
    }

    #[test]
    fn parses_stroke_style_object_value_with_literal_fields() {
        let mut ctx = parser_context();
        let value = json!({
            "dashArray": [
                { "value": 1, "unit": "px" },
                "{spacing.small}"
            ],
            "lineCap": "round"
        });

        let parsed = StrokeStyleObjectValue::try_from_json(&mut ctx, "/token", &value).unwrap();

        match parsed.dash_array {
            RefOr::Literal(DashArrayValue(entries)) => {
                assert_eq!(entries.len(), 2);
                assert!(matches!(entries[0], RefAliasOrLiteral::Literal(_)));
                assert_eq!(
                    entries[1],
                    RefAliasOrLiteral::Alias(
                        TokenAlias::from_dtcg_alias("{spacing.small}").unwrap()
                    )
                );
            }
            _ => panic!("expected dashArray to parse as a literal array"),
        }
        assert_eq!(parsed.line_cap, RefOr::Literal(LineCap::Round));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn parses_stroke_style_object_value_with_references() {
        let mut ctx = parser_context();
        let value = json!({
            "dashArray": { "$ref": "#/stroke/dashArray" },
            "lineCap": { "$ref": "#/stroke/lineCap" }
        });

        let parsed = StrokeStyleObjectValue::try_from_json(&mut ctx, "/token", &value).unwrap();

        match parsed.dash_array {
            RefOr::Ref(json_ref) => assert_eq!(
                json_ref,
                JsonRef::new_local_pointer(
                    "#/stroke/dashArray".to_string(),
                    JsonPointer::from("#/stroke/dashArray")
                )
            ),
            _ => panic!("expected dashArray to be a reference"),
        }
        match parsed.line_cap {
            RefOr::Ref(json_ref) => assert_eq!(
                json_ref,
                JsonRef::new_local_pointer(
                    "#/stroke/lineCap".to_string(),
                    JsonPointer::from("#/stroke/lineCap")
                )
            ),
            _ => panic!("expected lineCap to be a reference"),
        }
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn reports_missing_required_fields_for_stroke_style_object() {
        let mut ctx = parser_context();

        let parsed = StrokeStyleObjectValue::try_from_json(&mut ctx, "/token", &json!({}));

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 3);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::MissingRequiredProperty);
        assert_eq!(ctx.errors[0].path, "/token/dashArray");
        assert_eq!(ctx.errors[1].code, DiagnosticCode::MissingRequiredProperty);
        assert_eq!(ctx.errors[1].path, "/token/lineCap");
        assert_eq!(ctx.errors[2].code, DiagnosticCode::MissingRequiredProperty);
        assert_eq!(ctx.errors[2].path, "/token");
    }

    #[test]
    fn rejects_invalid_top_level_stroke_style_object_shape() {
        let mut ctx = parser_context();

        let parsed = StrokeStyleObjectValue::try_from_json(&mut ctx, "/token", &json!(false));

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyType);
        assert_eq!(ctx.errors[0].path, "/token");
    }

    #[test]
    fn parses_stroke_style_token_as_string_or_object() {
        let mut ctx = parser_context();
        let string_value =
            StrokeStyleTokenValue::try_from_json(&mut ctx, "/token", &json!("ridge"));
        let object_value = StrokeStyleTokenValue::try_from_json(
            &mut ctx,
            "/token",
            &json!({
                "dashArray": [{ "value": 1, "unit": "px" }],
                "lineCap": "square"
            }),
        );

        assert_eq!(
            string_value,
            Some(StrokeStyleTokenValue::String(StrokeStyleStringValue::Ridge))
        );
        assert!(matches!(
            object_value,
            Some(StrokeStyleTokenValue::Object(_))
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn rejects_invalid_stroke_style_token_type() {
        let mut ctx = parser_context();

        let parsed = StrokeStyleTokenValue::try_from_json(&mut ctx, "/token", &json!(42));

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyType);
        assert_eq!(ctx.errors[0].path, "/token");
    }
}
