//! The `token` module is responsible for defining on how to convert serde representations of tokens into the IR tokens.

use std::collections::HashMap;

use crate::{
    PROVENANCE_EXTENSION_KEY, ParserContext,
    errors::DiagnosticCode,
    ir::token::{
        BorderTokenValue, ColorTokenValue, CubicBezierTokenValue, DimensionTokenValue,
        DurationTokenValue, FontFamilyTokenValue, FontWeightTokenValue, GradientTokenValue,
        NumberTokenValue, ShadowTokenValue, StrokeStyleTokenValue, TransitionTokenValue,
        TypographyTokenValue,
    },
    ir::{
        Deprecation, DiagnosticEmission, InvalidReason, IrToken, IrTokenType, IrTokenValue,
        JsonRefObject, ParseState, TokenAlias, TokenCommon, TokenId, TokenName, TokenPath,
        TokenProvenance, TokenValue, TryFromJson,
    },
    parsing::{RawCommon, RawToken},
};

pub fn parse_common_fields(
    raw_common: &RawCommon,
    ctx: &mut ParserContext,
    token_id: TokenId,
    token_name: TokenName,
) -> TokenCommon {
    let path = TokenPath::from_segments(ctx.current_path.segments.iter().cloned());
    let description = raw_common.description.clone();
    let deprecation = raw_common
        .deprecated
        .clone()
        .map::<Deprecation, _>(|d| d.into());
    let extensions = raw_common
        .extensions
        .clone()
        .map::<HashMap<String, serde_json::Value>, _>(|map| map.into_iter().collect());
    // Keep typed provenance readily available while preserving raw extensions.
    let provenance = extensions.as_ref().and_then(|ext| {
        let provenance = ext.get(PROVENANCE_EXTENSION_KEY)?;
        let source = provenance.get("source")?.as_str()?;
        let pointer = provenance.get("pointer")?.as_str()?;
        Some(TokenProvenance {
            source: source.to_string(),
            pointer: pointer.to_string(),
        })
    });
    TokenCommon {
        id: token_id,
        path,
        provenance,
        name: token_name,
        description: description,
        deprecation: deprecation,
        extensions: extensions,
    }
}

pub fn parse_raw_token_to_ir_token(
    raw_token: &RawToken,
    ctx: &mut ParserContext,
    _token_name: TokenName,
    common: TokenCommon,
) -> ParseState<IrToken> {
    let path = ctx.get_current_path();
    let value_path = format!("{}/$value", path);
    // First, we need to determine the token type. We can get this from first, the `$type`. If it is not present,
    // we will try to get it from the ParserContext's current group type. If that is also not present, we can't
    // determine the token type, and we should throw an error.
    let token_type = if let Some(token_type) = &raw_token.token_type {
        match IrTokenType::from_str(token_type) {
            Some(t) => t,
            None => {
                ctx.push_to_errors(
                    DiagnosticCode::Other,
                    format!("Invalid token type: {}", token_type),
                    path.clone(),
                );
                return ParseState::invalid_emitted(InvalidReason::InvalidFieldType);
            }
        }
    } else if let Some(group_type) = ctx.get_current_type() {
        group_type
    } else {
        ctx.push_to_errors(
            DiagnosticCode::Other,
            "Missing token type".to_string(),
            path.clone(),
        );
        return ParseState::invalid_emitted(InvalidReason::MissingRequiredField);
    };

    let value = match parse_token_value(&raw_token.value, ctx, &value_path, token_type) {
        ParseState::Parsed(val) => val,
        ParseState::Failed(inv) => {
            if inv.ownership == DiagnosticEmission::Silent {
                ctx.push_to_errors(
                    DiagnosticCode::Other,
                    format!("Invalid token value for token type {:?}", token_type),
                    value_path.clone(),
                );
                return ParseState::invalid_emitted(InvalidReason::InvalidFieldType);
            }
            return ParseState::Failed(inv);
        }
        ParseState::NoMatch => {
            ctx.push_to_errors(
                DiagnosticCode::Other,
                format!(
                    "Token value does not match expected format for token type {:?}",
                    token_type
                ),
                value_path,
            );
            return ParseState::invalid_emitted(InvalidReason::InvalidFieldType);
        }
    };

    ParseState::Parsed(IrToken {
        common,
        token_type,
        value,
    })
}

/// Given the token type, parse the raw JSON value into the corresponding token value. ParseState from the parsers will be returned, which include errors if any.
///
/// # Arguments
///
/// * `value` - The raw JSON value to be parsed into a token value.
/// * `ctx` - The parser context, which can be used to emit errors during parsing if the value is invalid or does not match the expected format for the token type.
/// * `path` - The path to the token being parsed, used for error reporting to indicate where in the input the error occurred.
/// * `token_type` - The type of the token being parsed, which determines how the raw JSON value should be interpreted and parsed into a specific token value type.
///
/// # Returns
///
/// A `ParseState<IrTokenValue>` representing the result of parsing the raw JSON value into the corresponding token value based on the specified token type. The `ParseState` can indicate whether
/// the parsing was successful (with the parsed token value), or if it was invalid (with an error reason), or if there was no match for the expected format.
pub fn parse_by_token_type(
    value: &serde_json::Value,
    ctx: &mut ParserContext,
    path: &str,
    token_type: IrTokenType,
) -> ParseState<IrTokenValue> {
    match token_type {
        IrTokenType::Border => BorderTokenValue::try_from_json(ctx, path, value).into(),
        IrTokenType::Color => ColorTokenValue::try_from_json(ctx, path, value).into(),
        IrTokenType::CubicBezier => CubicBezierTokenValue::try_from_json(ctx, path, value).into(),
        IrTokenType::Dimension => DimensionTokenValue::try_from_json(ctx, path, value).into(),
        IrTokenType::Duration => DurationTokenValue::try_from_json(ctx, path, value).into(),
        IrTokenType::FontFamily => FontFamilyTokenValue::try_from_json(ctx, path, value).into(),
        IrTokenType::FontWeight => FontWeightTokenValue::try_from_json(ctx, path, value).into(),
        IrTokenType::Gradient => GradientTokenValue::try_from_json(ctx, path, value).into(),
        IrTokenType::Number => NumberTokenValue::try_from_json(ctx, path, value).into(),
        IrTokenType::Shadow => ShadowTokenValue::try_from_json(ctx, path, value).into(),
        IrTokenType::StrokeStyle => StrokeStyleTokenValue::try_from_json(ctx, path, value).into(),
        IrTokenType::Transition => TransitionTokenValue::try_from_json(ctx, path, value).into(),
        IrTokenType::Typography => TypographyTokenValue::try_from_json(ctx, path, value).into(),
    }
}

pub fn parse_token_value(
    value: &serde_json::Value,
    ctx: &mut ParserContext,
    path: &str,
    token_type: IrTokenType,
) -> ParseState<TokenValue> {
    // The value can be either a DTCG alias string, a JSON reference object, or a direct literal value.
    match TokenAlias::try_from_json(ctx, path, value) {
        ParseState::Parsed(alias) => ParseState::Parsed(TokenValue::Alias(alias)),
        ParseState::Failed(inv) => {
            if inv.ownership == DiagnosticEmission::Silent {
                // If it's an invalid alias, we should emit an error and return early, since the value is not valid.
                ctx.push_to_errors(
                    DiagnosticCode::Other,
                    format!("Invalid token alias"),
                    path.to_string(),
                );
                return ParseState::invalid_emitted(InvalidReason::InvalidFieldType);
            }
            return ParseState::Failed(inv);
        }
        ParseState::NoMatch => match JsonRefObject::try_from_json(ctx, path, value) {
            ParseState::Parsed(json_ref) => ParseState::Parsed(TokenValue::Ref(json_ref.reference)),
            ParseState::Failed(inv) => {
                if inv.ownership == DiagnosticEmission::Silent {
                    // If it's an invalid reference, we should emit an error and return early, since the value is not valid.
                    ctx.push_to_errors(
                        DiagnosticCode::Other,
                        format!("Invalid token reference"),
                        path.to_string(),
                    );
                    return ParseState::invalid_emitted(InvalidReason::InvalidFieldType);
                }
                return ParseState::Failed(inv);
            }
            ParseState::NoMatch => match parse_by_token_type(value, ctx, path, token_type) {
                ParseState::Parsed(token_value) => {
                    ParseState::Parsed(TokenValue::Value(token_value))
                }
                ParseState::Failed(inv) => {
                    if inv.ownership == DiagnosticEmission::Silent {
                        ctx.push_to_errors(
                            DiagnosticCode::Other,
                            format!("Invalid token value"),
                            path.to_string(),
                        );
                        return ParseState::invalid_emitted(InvalidReason::InvalidFieldType);
                    }
                    return ParseState::Failed(inv);
                }
                ParseState::NoMatch => {
                    ctx.push_to_errors(
                        DiagnosticCode::Other,
                        "Token value does not match expected format".to_string(),
                        path.to_string(),
                    );
                    ParseState::invalid_emitted(InvalidReason::InvalidFieldType)
                }
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use test_log::test;

    use super::*;
    use crate::{ParserContext, ir::TokenPath};

    fn test_ctx() -> ParserContext {
        ParserContext::new("{}".to_string())
    }

    #[test]
    fn parse_token_value_accepts_dtcg_alias_string() {
        let mut ctx = test_ctx();

        let state = parse_token_value(
            &json!("{colors.brand.primary}"),
            &mut ctx,
            "/brand/primary",
            IrTokenType::Color,
        );

        match state {
            ParseState::Parsed(TokenValue::Alias(alias)) => {
                assert_eq!(alias.raw_value, "{colors.brand.primary}");
                assert_eq!(
                    alias.target_path,
                    TokenPath::from_segments(["colors", "brand", "primary"])
                );
            }
            _ => panic!("Expected alias token value"),
        }

        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn parse_token_value_accepts_json_ref_object() {
        let mut ctx = test_ctx();

        let state = parse_token_value(
            &json!({ "$ref": "#/colors/brand/primary" }),
            &mut ctx,
            "/brand/primary",
            IrTokenType::Color,
        );

        match state {
            ParseState::Parsed(TokenValue::Ref(json_ref)) => {
                assert_eq!(json_ref.document, None);
                assert_eq!(
                    json_ref.pointer,
                    crate::ir::JsonPointer::from_segments(["colors", "brand", "primary"])
                );
            }
            _ => panic!("Expected JSON reference token value"),
        }

        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn parse_token_value_rejects_json_ref_string() {
        let mut ctx = test_ctx();

        let state = parse_token_value(
            &json!("#/colors/brand/primary"),
            &mut ctx,
            "/brand/primary",
            IrTokenType::Color,
        );

        assert!(matches!(state, ParseState::Failed(_)));
        assert!(!ctx.errors.is_empty());
    }
}
