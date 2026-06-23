//! Build CSS AST documents from lowered tokens.

use crate::{
    ir::IrTokenValue,
    output::{
        EmittableToken, EmittableValue,
        css::{
            ast::{CssDeclaration, CssDocument, CssNode, CssRule},
            context::{CssEmitContext, CssEmitError},
            utils::ToCssDeclarations,
        },
    },
};

pub fn to_css_document(
    tokens: &[EmittableToken],
    ctx: &CssEmitContext,
) -> Result<CssDocument, CssEmitError> {
    let mut declarations = Vec::<CssDeclaration>::new();

    for token in tokens {
        let base_name = ctx.var_name_for_path(&token.path);

        match &token.value {
            EmittableValue::Alias { target_path } => {
                declarations.push(CssDeclaration {
                    property: base_name,
                    value: crate::output::css::ast::CssValue::Var(
                        ctx.var_ref_for_path(target_path),
                    ),
                });
            }
            EmittableValue::Literal(value) => {
                declarations.extend(ir_value_to_declarations(value, &base_name, ctx)?);
            }
        }
    }

    Ok(CssDocument {
        nodes: vec![CssNode::Rule(CssRule {
            selector: ctx.options.root_selector.clone(),
            declarations,
        })],
    })
}

fn ir_value_to_declarations(
    value: &IrTokenValue,
    base_name: &crate::output::css::ast::CssProperty,
    ctx: &CssEmitContext,
) -> Result<Vec<CssDeclaration>, CssEmitError> {
    match value {
        IrTokenValue::Color(v) => v.to_css_declarations(base_name, ctx),
        IrTokenValue::Dimension(v) => v.to_css_declarations(base_name, ctx),
        IrTokenValue::FontFamily(v) => v.to_css_declarations(base_name, ctx),
        IrTokenValue::FontWeight(v) => v.to_css_declarations(base_name, ctx),
        IrTokenValue::Duration(v) => v.to_css_declarations(base_name, ctx),
        IrTokenValue::CubicBezier(v) => v.to_css_declarations(base_name, ctx),
        IrTokenValue::Number(v) => v.to_css_declarations(base_name, ctx),
        IrTokenValue::StrokeStyle(v) => v.to_css_declarations(base_name, ctx),
        IrTokenValue::Border(v) => v.to_css_declarations(base_name, ctx),
        IrTokenValue::Transition(v) => v.to_css_declarations(base_name, ctx),
        IrTokenValue::Shadow(v) => v.to_css_declarations(base_name, ctx),
        IrTokenValue::Gradient(v) => v.to_css_declarations(base_name, ctx),
        IrTokenValue::Typography(v) => v.to_css_declarations(base_name, ctx),
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        ir::token::{
            BorderTokenValue, ColorTokenValue, DimensionTokenValue,
            color::{ColorComponentArray, ColorComponentArrayElement, ColorSpaceString},
            dimension::{DimensionUnit, DimensionValue},
            token_types::composite::{
                border::{BorderColor, BorderStyle, BorderWidth},
                stroke_style::{
                    StrokeStyleDashArrayValue, StrokeStyleLineCapValue, StrokeStyleObjectValue,
                    StrokeStyleTokenValue,
                },
            },
        },
        ir::{IrTokenValue, JsonNumber, RefAliasOrLiteral, RefOrLiteral, TokenId, TokenPath},
        output::{
            EmittableToken, EmittableValue,
            css::{
                ast::{CssNode, CssProperty, CssValue},
                context::{CssCompositeOptions, CssConfigOptions, CssEmitContext},
                document::to_css_document,
            },
        },
    };

    #[test]
    fn to_css_document_expands_border_with_object_stroke_style() {
        let ctx = CssEmitContext::new(CssConfigOptions {
            composite_options: CssCompositeOptions {
                expand_border: false,
                expand_stroke_style: false,
                ..CssCompositeOptions::default()
            },
            ..CssConfigOptions::default()
        });

        let token = EmittableToken {
            token_id: TokenId(1),
            path: TokenPath::from_segments(["border", "primary"]),
            value: EmittableValue::Literal(IrTokenValue::Border(BorderTokenValue {
                color: BorderColor(RefAliasOrLiteral::Literal(ColorTokenValue {
                    color_space: RefOrLiteral::Literal(ColorSpaceString::SRGB),
                    components: RefOrLiteral::Literal(ColorComponentArray([
                        ColorComponentArrayElement::Number(JsonNumber(serde_json::Number::from(
                            255,
                        ))),
                        ColorComponentArrayElement::Number(JsonNumber(serde_json::Number::from(0))),
                        ColorComponentArrayElement::Number(JsonNumber(serde_json::Number::from(0))),
                    ])),
                    alpha: None,
                    hex: None,
                })),
                width: BorderWidth(RefAliasOrLiteral::Literal(DimensionTokenValue {
                    value: RefOrLiteral::Literal(DimensionValue(JsonNumber(
                        serde_json::Number::from(1),
                    ))),
                    unit: RefOrLiteral::Literal(DimensionUnit::Px),
                })),
                style: BorderStyle(RefAliasOrLiteral::Literal(StrokeStyleTokenValue::Object(
                    RefOrLiteral::Literal(StrokeStyleObjectValue {
                        dash_array: RefOrLiteral::Literal(StrokeStyleDashArrayValue(vec![
                            RefAliasOrLiteral::Literal(DimensionTokenValue {
                                value: RefOrLiteral::Literal(DimensionValue(JsonNumber(
                                    serde_json::Number::from(2),
                                ))),
                                unit: RefOrLiteral::Literal(DimensionUnit::Px),
                            }),
                        ])),
                        line_cap: RefOrLiteral::Literal(StrokeStyleLineCapValue::Round),
                    }),
                ))),
            })),
        };

        let document = to_css_document(&[token], &ctx).expect("expected css document");
        let CssNode::Rule(rule) = &document.nodes[0] else {
            panic!("expected rule node");
        };

        assert!(rule.declarations.iter().any(|d| {
            matches!(
                d.property,
                CssProperty::Custom(ref name) if name == "--border-primary-style-dash-array"
            )
        }));
        assert!(rule.declarations.iter().any(|d| {
            matches!(
                d.property,
                CssProperty::Custom(ref name) if name == "--border-primary-style-line-cap"
            )
        }));
    }

    #[test]
    fn to_css_document_emits_alias_var_declaration() {
        let ctx = CssEmitContext::new(CssConfigOptions::default());

        let token = EmittableToken {
            token_id: TokenId(2),
            path: TokenPath::from_segments(["color", "secondary"]),
            value: EmittableValue::Alias {
                target_path: TokenPath::from_segments(["color", "primary"]),
            },
        };

        let document = to_css_document(&[token], &ctx).expect("expected css document");
        let CssNode::Rule(rule) = &document.nodes[0] else {
            panic!("expected rule node");
        };

        assert_eq!(rule.declarations.len(), 1);
        assert!(matches!(
            rule.declarations[0].value,
            CssValue::Var(ref var) if var.name == "--color-primary"
        ));
    }
}
