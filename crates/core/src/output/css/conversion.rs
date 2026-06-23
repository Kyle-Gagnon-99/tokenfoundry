//! The `conversion` module provides the trait implementations that convert various IR types into their corresponding CSS AST representations.

use crate::{
    ir::token::{
        BorderTokenValue, ColorTokenValue, CubicBezierTokenValue, DimensionTokenValue,
        DurationTokenValue, FontFamilyTokenValue, FontWeightTokenValue, GradientTokenValue,
        NumberTokenValue, ShadowTokenValue, TransitionTokenValue, TypographyTokenValue,
        color::{ColorComponentArrayElement, ColorSpaceString},
        duration::{DurationUnit, DurationValue},
        font_family::FontFamilyValue,
        font_weight::{FontWeightValue, FontWeightValueNumber, FontWeightValueString},
        token_types::composite::{
            border::{BorderColor, BorderStyle, BorderWidth},
            gradient::{GradientObject, GradientObjectColor, GradientObjectPosition},
            shadow::{
                ShadowObjectBlur, ShadowObjectColor, ShadowObjectInset, ShadowObjectOffsetX,
                ShadowObjectOffsetY, ShadowObjectSpread, ShadowObjectValue,
            },
            stroke_style::{
                StrokeStyleDashArrayValue, StrokeStyleLineCapValue, StrokeStyleObjectValue,
                StrokeStyleStringValue, StrokeStyleTokenValue,
            },
            transition::{TransitionDelay, TransitionDuration, TransitionTimingFunction},
            typography::{
                TypographyFontFamily, TypographyFontSize, TypographyFontWeight,
                TypographyLetterSpacing, TypographyLineHeight,
            },
        },
    },
    ir::{RefAliasOrLiteral, RefOrLiteral},
    output::css::{
        ast::{CssDeclaration, CssProperty, CssSeparator, CssUnit, CssValue},
        context::{CssEmitContext, CssEmitError},
        utils::{
            CssValueResult, ToCssDeclarations, ToCssValue, css_ref_alias_or_literal,
            css_ref_or_literal, extract_value_from_ref_or_literal, single_value_declaration,
        },
    },
};

impl<T> ToCssValue for RefAliasOrLiteral<T>
where
    T: ToCssValue,
{
    fn to_css_value(&self, ctx: &CssEmitContext) -> CssValueResult {
        match self {
            RefAliasOrLiteral::Literal(literal) => literal.to_css_value(ctx),
            RefAliasOrLiteral::Ref(_) => Err(CssEmitError::UnexpectedJsonReference {
                pointer: "JSON references are not supported during CSS emission".to_string(),
            }),
            RefAliasOrLiteral::Alias(alias) => {
                Ok(CssValue::Var(ctx.var_ref_for_path(&alias.target_path)))
            }
        }
    }
}

impl<T> ToCssDeclarations for RefAliasOrLiteral<T>
where
    T: ToCssValue,
{
    fn to_css_declarations(
        &self,
        base_name: &CssProperty,
        ctx: &CssEmitContext,
    ) -> Result<Vec<CssDeclaration>, CssEmitError> {
        single_value_declaration(base_name, self.to_css_value(ctx)?)
    }
}

fn suffixed_property(base_name: &CssProperty, suffix: &str) -> CssProperty {
    match base_name {
        CssProperty::Custom(name) => CssProperty::Custom(format!("{}-{}", name, suffix)),
        CssProperty::Standard(name) => CssProperty::Standard(format!("{}-{}", name, suffix)),
    }
}

macro_rules! impl_single_value_declarations {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl ToCssDeclarations for $ty {
                fn to_css_declarations(
                    &self,
                    base_name: &CssProperty,
                    ctx: &CssEmitContext,
                ) -> Result<Vec<CssDeclaration>, CssEmitError> {
                    single_value_declaration(base_name, self.to_css_value(ctx)?)
                }
            }
        )+
    };
}

impl ToCssValue for DimensionTokenValue {
    fn to_css_value(&self, _ctx: &CssEmitContext) -> CssValueResult {
        Ok(CssValue::Dimension {
            value: extract_value_from_ref_or_literal(&self.value)?.into(),
            unit: extract_value_from_ref_or_literal(&self.unit)?.into(),
        })
    }
}

impl ToCssValue for ColorTokenValue {
    fn to_css_value(&self, _ctx: &CssEmitContext) -> CssValueResult {
        // First, we need to get the function name based on the color space. The color space must map to a valid CSS color function.
        // If there is no valid mapping, we will use the `color` function as a fallback, which can accept all other forms of color values as a string.
        let color_space = extract_value_from_ref_or_literal(&self.color_space)?;
        let function_name = match color_space {
            ColorSpaceString::SRGB => "rgb",
            ColorSpaceString::HSL => "hsl",
            ColorSpaceString::HWB => "hwb",
            ColorSpaceString::CIELAB => "lab",
            ColorSpaceString::LCH => "lch",
            ColorSpaceString::OKLAB => "oklab",
            ColorSpaceString::OKLCH => "oklch",
            // If we encounter a color space that doesn't have a direct CSS function mapping, we will
            // use the `color` function as a fallback, which can accept a string representation of the color value.
            _ => "color",
        };

        // Create an empty list of arguments for the function
        let mut args = Vec::new();

        // First, if the function name is `color`, the first argument should be the color space as a string. For example: `color(srgb 255 0 0)`.
        // For other function names like `rgb`, `hsl`, etc., the color space is already determined by the function name, so it is not included
        if function_name == "color" {
            // Convert the color space to the appropriate string format for CSS
            let color_space_str = match color_space {
                ColorSpaceString::SRGB => "srgb",
                ColorSpaceString::SRGBLinear => "srgb-linear",
                ColorSpaceString::DisplayP3 => "display-p3",
                ColorSpaceString::A98RGB => "a98-rgb",
                ColorSpaceString::ProPhotoRGB => "prophoto-rgb",
                ColorSpaceString::Rec2020 => "rec2020",
                ColorSpaceString::XYZD50 => "xyz-d50",
                ColorSpaceString::XYZD65 => "xyz-d65",
                _ => {
                    // This case should never happen because we already handle the case above, but this is here to satisfy exhaustiveness
                    // and defensive programming principles.
                    return Err(CssEmitError::InvalidColorSpace);
                }
            };

            args.push(CssValue::Ident(color_space_str.to_string()));
        }

        // Next, we need to add the color components as arguments to the function
        let components = extract_value_from_ref_or_literal(&self.components)?;
        for component in components.0.iter() {
            args.push(CssValue::Raw(match component {
                ColorComponentArrayElement::Number(num) => num.0.to_string(),
                ColorComponentArrayElement::None => "none".to_string(),
            }))
        }

        // Finally, if alpha is present and specificed, we need to add "/" and the alpha value as the last arguments to the function
        if self.alpha.is_some() {
            let alpha_value = extract_value_from_ref_or_literal(&self.alpha.as_ref().unwrap())?;
            args.push(CssValue::Raw("/".to_string()));
            args.push(CssValue::Raw(alpha_value.0.0.to_string()));
        }

        Ok(CssValue::Function {
            name: function_name.to_string(),
            args,
            separator: CssSeparator::Space,
        })
    }
}

impl ToCssValue for CubicBezierTokenValue {
    fn to_css_value(&self, _ctx: &CssEmitContext) -> CssValueResult {
        let mut args: Vec<CssValue> = Vec::new();
        for component in self.0.iter() {
            let value = extract_value_from_ref_or_literal(component)?;
            args.push(CssValue::Raw(value.0.to_string()));
        }

        Ok(CssValue::Function {
            name: "cubic-bezier".to_string(),
            args,
            separator: CssSeparator::Comma,
        })
    }
}

impl ToCssValue for FontFamilyTokenValue {
    fn to_css_value(&self, _ctx: &CssEmitContext) -> CssValueResult {
        let mut items: Vec<CssValue> = Vec::new();
        match &self.0 {
            FontFamilyValue::Single(single) => {
                let value = extract_value_from_ref_or_literal(&single)?;
                items.push(CssValue::Raw(value.into()));
            }
            FontFamilyValue::Multiple(multiple) => {
                let value = extract_value_from_ref_or_literal(&multiple)?;
                for font_items in value.0.iter() {
                    let item_value = extract_value_from_ref_or_literal(font_items)?;
                    items.push(CssValue::Raw(item_value.into()));
                }
            }
        }

        Ok(CssValue::List {
            separator: CssSeparator::Comma,
            items,
        })
    }
}

impl Into<String> for FontWeightValueString {
    fn into(self) -> String {
        match self {
            FontWeightValueString::Thin => "thin".to_string(),
            FontWeightValueString::Hairline => "hairline".to_string(),
            FontWeightValueString::ExtraLight => "extra-light".to_string(),
            FontWeightValueString::UltraLight => "ultra-light".to_string(),
            FontWeightValueString::Light => "light".to_string(),
            FontWeightValueString::Normal => "normal".to_string(),
            FontWeightValueString::Medium => "medium".to_string(),
            FontWeightValueString::SemiBold => "semi-bold".to_string(),
            FontWeightValueString::Bold => "bold".to_string(),
            FontWeightValueString::ExtraBold => "extra-bold".to_string(),
            FontWeightValueString::UltraBold => "ultra-bold".to_string(),
            FontWeightValueString::Black => "black".to_string(),
            FontWeightValueString::Heavy => "heavy".to_string(),
            FontWeightValueString::ExtraBlack => "extra-black".to_string(),
            FontWeightValueString::UltraBlack => "ultra-black".to_string(),
        }
    }
}

impl Into<String> for FontWeightValueNumber {
    fn into(self) -> String {
        self.0.0.to_string()
    }
}

impl ToCssValue for FontWeightTokenValue {
    fn to_css_value(&self, _ctx: &CssEmitContext) -> CssValueResult {
        let value = extract_value_from_ref_or_literal(&self.0)?;
        match value {
            FontWeightValue::String(str) => Ok(CssValue::Raw(str.into())),
            FontWeightValue::Number(num) => Ok(CssValue::Raw(num.into())),
        }
    }
}

impl Into<String> for DurationValue {
    fn into(self) -> String {
        self.0.0.to_string()
    }
}

impl ToCssValue for DurationTokenValue {
    fn to_css_value(&self, _ctx: &CssEmitContext) -> CssValueResult {
        let value = extract_value_from_ref_or_literal(&self.value)?;
        let unit = extract_value_from_ref_or_literal(&self.unit)?;

        // Map the duration unit to the corresponding CSS unit
        let css_unit = match unit {
            DurationUnit::Ms => CssUnit::Ms,
            DurationUnit::S => CssUnit::S,
        };

        Ok(CssValue::Duration {
            value: value.into(),
            unit: css_unit,
        })
    }
}

impl ToCssValue for NumberTokenValue {
    fn to_css_value(&self, _ctx: &CssEmitContext) -> CssValueResult {
        let value = extract_value_from_ref_or_literal(&self.0)?;
        Ok(CssValue::Raw(value.0.to_string()))
    }
}

impl ToCssValue for BorderColor {
    fn to_css_value(&self, ctx: &CssEmitContext) -> CssValueResult {
        css_ref_alias_or_literal(&self.0, ctx)
    }
}

impl ToCssValue for BorderWidth {
    fn to_css_value(&self, ctx: &CssEmitContext) -> CssValueResult {
        css_ref_alias_or_literal(&self.0, ctx)
    }
}

impl ToCssValue for BorderStyle {
    fn to_css_value(&self, ctx: &CssEmitContext) -> CssValueResult {
        css_ref_alias_or_literal(&self.0, ctx)
    }
}

impl ToCssValue for BorderTokenValue {
    fn to_css_value(&self, ctx: &CssEmitContext) -> CssValueResult {
        Ok(CssValue::List {
            separator: CssSeparator::Space,
            items: vec![
                self.color.to_css_value(ctx)?,
                self.width.to_css_value(ctx)?,
                self.style.to_css_value(ctx)?,
            ],
        })
    }
}

impl ToCssValue for StrokeStyleStringValue {
    fn to_css_value(&self, _ctx: &CssEmitContext) -> CssValueResult {
        let value = match self {
            StrokeStyleStringValue::Solid => "solid",
            StrokeStyleStringValue::Dashed => "dashed",
            StrokeStyleStringValue::Dotted => "dotted",
            StrokeStyleStringValue::Double => "double",
            StrokeStyleStringValue::Groove => "groove",
            StrokeStyleStringValue::Ridge => "ridge",
            StrokeStyleStringValue::Outset => "outset",
            StrokeStyleStringValue::Inset => "inset",
        };

        Ok(CssValue::Ident(value.to_string()))
    }
}

impl ToCssValue for StrokeStyleLineCapValue {
    fn to_css_value(&self, _ctx: &CssEmitContext) -> CssValueResult {
        let value = match self {
            StrokeStyleLineCapValue::Round => "round",
            StrokeStyleLineCapValue::Butt => "butt",
            StrokeStyleLineCapValue::Square => "square",
        };

        Ok(CssValue::Ident(value.to_string()))
    }
}

impl ToCssValue for StrokeStyleDashArrayValue {
    fn to_css_value(&self, ctx: &CssEmitContext) -> CssValueResult {
        let mut items = Vec::new();

        for item in &self.0 {
            items.push(item.to_css_value(ctx)?);
        }

        Ok(CssValue::List {
            separator: CssSeparator::Space,
            items,
        })
    }
}

impl ToCssValue for StrokeStyleObjectValue {
    fn to_css_value(&self, ctx: &CssEmitContext) -> CssValueResult {
        let mut items = Vec::new();

        match css_ref_or_literal(&self.dash_array, ctx)? {
            CssValue::List {
                items: dash_items, ..
            } => items.extend(dash_items),
            dash_array => items.push(dash_array),
        }

        items.push(css_ref_or_literal(&self.line_cap, ctx)?);

        Ok(CssValue::List {
            separator: CssSeparator::Space,
            items,
        })
    }
}

impl ToCssValue for StrokeStyleTokenValue {
    fn to_css_value(&self, ctx: &CssEmitContext) -> CssValueResult {
        match self {
            StrokeStyleTokenValue::String(value) => css_ref_or_literal(value, ctx),
            StrokeStyleTokenValue::Object(value) => css_ref_or_literal(value, ctx),
        }
    }
}

impl ToCssValue for GradientObjectColor {
    fn to_css_value(&self, ctx: &CssEmitContext) -> CssValueResult {
        css_ref_alias_or_literal(&self.0, ctx)
    }
}

impl ToCssValue for GradientObjectPosition {
    fn to_css_value(&self, _ctx: &CssEmitContext) -> CssValueResult {
        let value = extract_value_from_ref_or_literal(&self.0)?;
        Ok(CssValue::Raw(value.0.to_string()))
    }
}

impl ToCssValue for GradientObject {
    fn to_css_value(&self, ctx: &CssEmitContext) -> CssValueResult {
        Ok(CssValue::List {
            separator: CssSeparator::Space,
            items: vec![
                self.color.to_css_value(ctx)?,
                self.position.to_css_value(ctx)?,
            ],
        })
    }
}

impl ToCssValue for GradientTokenValue {
    fn to_css_value(&self, ctx: &CssEmitContext) -> CssValueResult {
        let mut items = Vec::new();

        for stop in extract_value_from_ref_or_literal(&self.0)? {
            items.push(css_ref_alias_or_literal(&stop, ctx)?);
        }

        Ok(CssValue::List {
            separator: CssSeparator::Comma,
            items,
        })
    }
}

impl ToCssValue for ShadowObjectColor {
    fn to_css_value(&self, ctx: &CssEmitContext) -> CssValueResult {
        css_ref_alias_or_literal(&self.0, ctx)
    }
}

impl ToCssValue for ShadowObjectOffsetX {
    fn to_css_value(&self, ctx: &CssEmitContext) -> CssValueResult {
        css_ref_alias_or_literal(&self.0, ctx)
    }
}

impl ToCssValue for ShadowObjectOffsetY {
    fn to_css_value(&self, ctx: &CssEmitContext) -> CssValueResult {
        css_ref_alias_or_literal(&self.0, ctx)
    }
}

impl ToCssValue for ShadowObjectBlur {
    fn to_css_value(&self, ctx: &CssEmitContext) -> CssValueResult {
        css_ref_alias_or_literal(&self.0, ctx)
    }
}

impl ToCssValue for ShadowObjectSpread {
    fn to_css_value(&self, ctx: &CssEmitContext) -> CssValueResult {
        css_ref_alias_or_literal(&self.0, ctx)
    }
}

impl ToCssValue for ShadowObjectInset {
    fn to_css_value(&self, _ctx: &CssEmitContext) -> CssValueResult {
        let is_inset = extract_value_from_ref_or_literal(&self.0)?;
        if is_inset {
            Ok(CssValue::Ident("inset".to_string()))
        } else {
            Ok(CssValue::Raw(String::new()))
        }
    }
}

impl ToCssValue for ShadowObjectValue {
    fn to_css_value(&self, ctx: &CssEmitContext) -> CssValueResult {
        let mut items = Vec::new();

        if let Some(inset) = &self.inset {
            let inset_value = inset.to_css_value(ctx)?;
            if !matches!(inset_value, CssValue::Raw(ref s) if s.is_empty()) {
                items.push(inset_value);
            }
        }

        items.push(self.offset_x.to_css_value(ctx)?);
        items.push(self.offset_y.to_css_value(ctx)?);
        items.push(self.blur.to_css_value(ctx)?);
        items.push(self.spread.to_css_value(ctx)?);
        items.push(self.color.to_css_value(ctx)?);

        Ok(CssValue::List {
            separator: CssSeparator::Space,
            items,
        })
    }
}

impl ToCssValue for ShadowTokenValue {
    fn to_css_value(&self, ctx: &CssEmitContext) -> CssValueResult {
        let mut items = Vec::new();

        for shadow in extract_value_from_ref_or_literal(&self.0)? {
            items.push(css_ref_alias_or_literal(&shadow, ctx)?);
        }

        Ok(CssValue::List {
            separator: CssSeparator::Comma,
            items,
        })
    }
}

impl ToCssValue for TransitionDuration {
    fn to_css_value(&self, ctx: &CssEmitContext) -> CssValueResult {
        css_ref_alias_or_literal(&self.0, ctx)
    }
}

impl ToCssValue for TransitionDelay {
    fn to_css_value(&self, ctx: &CssEmitContext) -> CssValueResult {
        css_ref_alias_or_literal(&self.0, ctx)
    }
}

impl ToCssValue for TransitionTimingFunction {
    fn to_css_value(&self, ctx: &CssEmitContext) -> CssValueResult {
        css_ref_alias_or_literal(&self.0, ctx)
    }
}

impl ToCssValue for TransitionTokenValue {
    fn to_css_value(&self, ctx: &CssEmitContext) -> CssValueResult {
        Ok(CssValue::List {
            separator: CssSeparator::Space,
            items: vec![
                self.duration.to_css_value(ctx)?,
                self.timing_function.to_css_value(ctx)?,
                self.delay.to_css_value(ctx)?,
            ],
        })
    }
}

impl ToCssValue for TypographyFontFamily {
    fn to_css_value(&self, ctx: &CssEmitContext) -> CssValueResult {
        css_ref_alias_or_literal(&self.0, ctx)
    }
}

impl ToCssValue for TypographyFontSize {
    fn to_css_value(&self, ctx: &CssEmitContext) -> CssValueResult {
        css_ref_alias_or_literal(&self.0, ctx)
    }
}

impl ToCssValue for TypographyFontWeight {
    fn to_css_value(&self, ctx: &CssEmitContext) -> CssValueResult {
        css_ref_alias_or_literal(&self.0, ctx)
    }
}

impl ToCssValue for TypographyLetterSpacing {
    fn to_css_value(&self, ctx: &CssEmitContext) -> CssValueResult {
        css_ref_alias_or_literal(&self.0, ctx)
    }
}

impl ToCssValue for TypographyLineHeight {
    fn to_css_value(&self, ctx: &CssEmitContext) -> CssValueResult {
        css_ref_alias_or_literal(&self.0, ctx)
    }
}

impl ToCssValue for TypographyTokenValue {
    fn to_css_value(&self, ctx: &CssEmitContext) -> CssValueResult {
        Ok(CssValue::List {
            separator: CssSeparator::Space,
            items: vec![
                self.font_family.to_css_value(ctx)?,
                self.font_size.to_css_value(ctx)?,
                self.font_weight.to_css_value(ctx)?,
                self.letter_spacing.to_css_value(ctx)?,
                self.line_height.to_css_value(ctx)?,
            ],
        })
    }
}

impl ToCssDeclarations for BorderTokenValue {
    fn to_css_declarations(
        &self,
        base_name: &CssProperty,
        ctx: &CssEmitContext,
    ) -> Result<Vec<CssDeclaration>, CssEmitError> {
        let style_supports_shorthand = !matches!(
            self.style.0,
            RefAliasOrLiteral::Literal(StrokeStyleTokenValue::Object(_))
        );

        if !ctx.options.composite_options.expand_border && style_supports_shorthand {
            return single_value_declaration(base_name, self.to_css_value(ctx)?);
        }

        let mut declarations = Vec::new();
        declarations.push(CssDeclaration {
            property: suffixed_property(base_name, "width"),
            value: self.width.to_css_value(ctx)?,
        });
        declarations.push(CssDeclaration {
            property: suffixed_property(base_name, "color"),
            value: self.color.to_css_value(ctx)?,
        });

        let style_base = suffixed_property(base_name, "style");
        match &self.style.0 {
            RefAliasOrLiteral::Literal(style) => {
                declarations.extend(style.to_css_declarations(&style_base, ctx)?);
            }
            RefAliasOrLiteral::Alias(alias) => {
                declarations.push(CssDeclaration {
                    property: style_base,
                    value: CssValue::Var(ctx.var_ref_for_path(&alias.target_path)),
                });
            }
            RefAliasOrLiteral::Ref(json_ref) => {
                return Err(CssEmitError::UnexpectedJsonReference {
                    pointer: json_ref.reference.pointer.to_string(),
                });
            }
        }

        Ok(declarations)
    }
}

impl ToCssDeclarations for StrokeStyleTokenValue {
    fn to_css_declarations(
        &self,
        base_name: &CssProperty,
        ctx: &CssEmitContext,
    ) -> Result<Vec<CssDeclaration>, CssEmitError> {
        let is_literal_object = matches!(
            self,
            StrokeStyleTokenValue::Object(RefOrLiteral::Literal(_))
        );

        if !ctx.options.composite_options.expand_stroke_style && !is_literal_object {
            return single_value_declaration(base_name, self.to_css_value(ctx)?);
        }

        match self {
            StrokeStyleTokenValue::String(value) => {
                single_value_declaration(base_name, css_ref_or_literal(value, ctx)?)
            }
            StrokeStyleTokenValue::Object(value) => {
                let object = extract_value_from_ref_or_literal(value)?;
                Ok(vec![
                    CssDeclaration {
                        property: suffixed_property(base_name, "dash-array"),
                        value: css_ref_or_literal(&object.dash_array, ctx)?,
                    },
                    CssDeclaration {
                        property: suffixed_property(base_name, "line-cap"),
                        value: css_ref_or_literal(&object.line_cap, ctx)?,
                    },
                ])
            }
        }
    }
}

impl ToCssDeclarations for TransitionTokenValue {
    fn to_css_declarations(
        &self,
        base_name: &CssProperty,
        ctx: &CssEmitContext,
    ) -> Result<Vec<CssDeclaration>, CssEmitError> {
        if !ctx.options.composite_options.expand_transition {
            return single_value_declaration(base_name, self.to_css_value(ctx)?);
        }

        Ok(vec![
            CssDeclaration {
                property: suffixed_property(base_name, "duration"),
                value: self.duration.to_css_value(ctx)?,
            },
            CssDeclaration {
                property: suffixed_property(base_name, "timing-function"),
                value: self.timing_function.to_css_value(ctx)?,
            },
            CssDeclaration {
                property: suffixed_property(base_name, "delay"),
                value: self.delay.to_css_value(ctx)?,
            },
        ])
    }
}

impl ToCssDeclarations for ShadowTokenValue {
    fn to_css_declarations(
        &self,
        base_name: &CssProperty,
        ctx: &CssEmitContext,
    ) -> Result<Vec<CssDeclaration>, CssEmitError> {
        if !ctx.options.composite_options.expand_shadow {
            return single_value_declaration(base_name, self.to_css_value(ctx)?);
        }

        let mut declarations = Vec::new();
        for (index, shadow) in extract_value_from_ref_or_literal(&self.0)?
            .into_iter()
            .enumerate()
        {
            let shadow_base = suffixed_property(base_name, &format!("layer-{}", index));
            match shadow {
                RefAliasOrLiteral::Literal(value) => {
                    declarations.push(CssDeclaration {
                        property: suffixed_property(&shadow_base, "offset-x"),
                        value: value.offset_x.to_css_value(ctx)?,
                    });
                    declarations.push(CssDeclaration {
                        property: suffixed_property(&shadow_base, "offset-y"),
                        value: value.offset_y.to_css_value(ctx)?,
                    });
                    declarations.push(CssDeclaration {
                        property: suffixed_property(&shadow_base, "blur"),
                        value: value.blur.to_css_value(ctx)?,
                    });
                    declarations.push(CssDeclaration {
                        property: suffixed_property(&shadow_base, "spread"),
                        value: value.spread.to_css_value(ctx)?,
                    });
                    declarations.push(CssDeclaration {
                        property: suffixed_property(&shadow_base, "color"),
                        value: value.color.to_css_value(ctx)?,
                    });

                    if let Some(inset) = value.inset {
                        let inset_value = inset.to_css_value(ctx)?;
                        if !matches!(inset_value, CssValue::Raw(ref raw) if raw.is_empty()) {
                            declarations.push(CssDeclaration {
                                property: suffixed_property(&shadow_base, "inset"),
                                value: inset_value,
                            });
                        }
                    }
                }
                RefAliasOrLiteral::Alias(alias) => {
                    declarations.push(CssDeclaration {
                        property: shadow_base,
                        value: CssValue::Var(ctx.var_ref_for_path(&alias.target_path)),
                    });
                }
                RefAliasOrLiteral::Ref(json_ref) => {
                    return Err(CssEmitError::UnexpectedJsonReference {
                        pointer: json_ref.reference.pointer.to_string(),
                    });
                }
            }
        }

        Ok(declarations)
    }
}

impl ToCssDeclarations for GradientTokenValue {
    fn to_css_declarations(
        &self,
        base_name: &CssProperty,
        ctx: &CssEmitContext,
    ) -> Result<Vec<CssDeclaration>, CssEmitError> {
        if !ctx.options.composite_options.expand_gradient {
            return single_value_declaration(base_name, self.to_css_value(ctx)?);
        }

        let mut declarations = Vec::new();
        for (index, stop) in extract_value_from_ref_or_literal(&self.0)?
            .into_iter()
            .enumerate()
        {
            let stop_base = suffixed_property(base_name, &format!("stop-{}", index));
            match stop {
                RefAliasOrLiteral::Literal(stop_value) => {
                    declarations.push(CssDeclaration {
                        property: suffixed_property(&stop_base, "color"),
                        value: stop_value.color.to_css_value(ctx)?,
                    });
                    declarations.push(CssDeclaration {
                        property: suffixed_property(&stop_base, "position"),
                        value: stop_value.position.to_css_value(ctx)?,
                    });
                }
                RefAliasOrLiteral::Alias(alias) => {
                    declarations.push(CssDeclaration {
                        property: stop_base,
                        value: CssValue::Var(ctx.var_ref_for_path(&alias.target_path)),
                    });
                }
                RefAliasOrLiteral::Ref(json_ref) => {
                    return Err(CssEmitError::UnexpectedJsonReference {
                        pointer: json_ref.reference.pointer.to_string(),
                    });
                }
            }
        }

        Ok(declarations)
    }
}

impl ToCssDeclarations for TypographyTokenValue {
    fn to_css_declarations(
        &self,
        base_name: &CssProperty,
        ctx: &CssEmitContext,
    ) -> Result<Vec<CssDeclaration>, CssEmitError> {
        single_value_declaration(base_name, self.to_css_value(ctx)?)
    }
}

impl_single_value_declarations!(
    DimensionTokenValue,
    ColorTokenValue,
    CubicBezierTokenValue,
    FontFamilyTokenValue,
    FontWeightTokenValue,
    DurationTokenValue,
    NumberTokenValue,
    BorderColor,
    BorderWidth,
    BorderStyle,
    StrokeStyleStringValue,
    StrokeStyleLineCapValue,
    StrokeStyleDashArrayValue,
    StrokeStyleObjectValue,
    GradientObjectColor,
    GradientObjectPosition,
    GradientObject,
    ShadowObjectColor,
    ShadowObjectOffsetX,
    ShadowObjectOffsetY,
    ShadowObjectBlur,
    ShadowObjectSpread,
    ShadowObjectInset,
    ShadowObjectValue,
    TransitionDuration,
    TransitionDelay,
    TransitionTimingFunction,
    TypographyFontFamily,
    TypographyFontSize,
    TypographyFontWeight,
    TypographyLetterSpacing,
    TypographyLineHeight
);

#[cfg(test)]
mod tests {
    use crate::{
        ir::token::{
            BorderTokenValue, ColorTokenValue, CubicBezierTokenValue, DimensionTokenValue,
            DurationTokenValue, FontFamilyTokenValue, FontWeightTokenValue, GradientTokenValue,
            NumberTokenValue, ShadowTokenValue, TransitionTokenValue, TypographyTokenValue,
            color::{
                ColorAlpha, ColorComponentArray, ColorComponentArrayElement, ColorSpaceString,
            },
            dimension::{DimensionUnit, DimensionValue},
            duration::{DurationUnit, DurationValue},
            font_family::{FontFamilyMultipleValue, FontFamilyValue},
            font_weight::{FontWeightValue, FontWeightValueNumber, FontWeightValueString},
            token_types::composite::{
                border::{BorderColor, BorderStyle, BorderWidth},
                gradient::{GradientObject, GradientObjectColor, GradientObjectPosition},
                shadow::{
                    ShadowObjectBlur, ShadowObjectColor, ShadowObjectInset, ShadowObjectOffsetX,
                    ShadowObjectOffsetY, ShadowObjectSpread, ShadowObjectValue,
                },
                stroke_style::{
                    StrokeStyleDashArrayValue, StrokeStyleLineCapValue, StrokeStyleObjectValue,
                    StrokeStyleStringValue, StrokeStyleTokenValue,
                },
                transition::{TransitionDelay, TransitionDuration, TransitionTimingFunction},
                typography::{
                    TypographyFontFamily, TypographyFontSize, TypographyFontWeight,
                    TypographyLetterSpacing, TypographyLineHeight,
                },
            },
        },
        ir::{JsonNumber, JsonRef, JsonRefObject, RefAliasOrLiteral, RefOrLiteral},
        output::css::{
            ast::{CssProperty, CssSeparator, CssUnit, CssValue},
            context::{CssCompositeOptions, CssConfigOptions, CssEmitContext, CssEmitError},
            utils::{ToCssDeclarations, ToCssValue},
        },
    };

    fn test_ctx() -> CssEmitContext {
        CssEmitContext::new(CssConfigOptions::default())
    }

    fn test_ctx_with_composites(composite_options: CssCompositeOptions) -> CssEmitContext {
        CssEmitContext::new(CssConfigOptions {
            composite_options,
            ..CssConfigOptions::default()
        })
    }

    fn json_num_f64(value: f64) -> JsonNumber {
        JsonNumber(serde_json::Number::from_f64(value).expect("expected finite number"))
    }

    fn ref_value(pointer: &str) -> JsonRefObject {
        JsonRefObject {
            reference: JsonRef::parse(pointer).expect("expected valid JSON pointer"),
        }
    }

    #[test]
    fn dimension_to_css_value_returns_dimension_node() {
        let token = DimensionTokenValue {
            value: RefOrLiteral::Literal(DimensionValue(JsonNumber(serde_json::Number::from(16)))),
            unit: RefOrLiteral::Literal(DimensionUnit::Px),
        };

        let css_value = token
            .to_css_value(&test_ctx())
            .expect("dimension should emit");

        assert_eq!(
            css_value,
            CssValue::Dimension {
                value: "16".to_string(),
                unit: "px".to_string(),
            }
        );
    }

    #[test]
    fn dimension_to_css_value_errors_for_json_reference_literal_extraction() {
        let token = DimensionTokenValue {
            value: RefOrLiteral::Ref(ref_value("#/tokens/spacing/md")),
            unit: RefOrLiteral::Literal(DimensionUnit::Rem),
        };

        let err = token
            .to_css_value(&test_ctx())
            .expect_err("json references are unsupported during css emission");

        assert!(matches!(
            err,
            CssEmitError::UnexpectedJsonReference { pointer }
            if pointer == "/tokens/spacing/md"
        ));
    }

    #[test]
    fn color_to_css_value_maps_srgb_to_rgb_with_space_separator() {
        let token = ColorTokenValue {
            color_space: RefOrLiteral::Literal(ColorSpaceString::SRGB),
            components: RefOrLiteral::Literal(ColorComponentArray([
                ColorComponentArrayElement::Number(JsonNumber(serde_json::Number::from(255))),
                ColorComponentArrayElement::Number(JsonNumber(serde_json::Number::from(0))),
                ColorComponentArrayElement::Number(JsonNumber(serde_json::Number::from(128))),
            ])),
            alpha: None,
            hex: None,
        };

        let css_value = token.to_css_value(&test_ctx()).expect("color should emit");

        assert_eq!(
            css_value,
            CssValue::Function {
                name: "rgb".to_string(),
                args: vec![
                    CssValue::Raw("255".to_string()),
                    CssValue::Raw("0".to_string()),
                    CssValue::Raw("128".to_string()),
                ],
                separator: CssSeparator::Space,
            }
        );
    }

    #[test]
    fn color_to_css_value_uses_color_function_for_non_direct_color_space() {
        let token = ColorTokenValue {
            color_space: RefOrLiteral::Literal(ColorSpaceString::DisplayP3),
            components: RefOrLiteral::Literal(ColorComponentArray([
                ColorComponentArrayElement::Number(json_num_f64(0.8)),
                ColorComponentArrayElement::None,
                ColorComponentArrayElement::Number(json_num_f64(0.2)),
            ])),
            alpha: Some(RefOrLiteral::Literal(ColorAlpha(json_num_f64(0.5)))),
            hex: None,
        };

        let css_value = token.to_css_value(&test_ctx()).expect("color should emit");

        assert_eq!(
            css_value,
            CssValue::Function {
                name: "color".to_string(),
                args: vec![
                    CssValue::Ident("display-p3".to_string()),
                    CssValue::Raw("0.8".to_string()),
                    CssValue::Raw("none".to_string()),
                    CssValue::Raw("0.2".to_string()),
                    CssValue::Raw("/".to_string()),
                    CssValue::Raw("0.5".to_string()),
                ],
                separator: CssSeparator::Space,
            }
        );
    }

    #[test]
    fn cubic_bezier_to_css_value_emits_comma_separated_function() {
        let token = CubicBezierTokenValue([
            RefOrLiteral::Literal(JsonNumber(serde_json::Number::from(0))),
            RefOrLiteral::Literal(json_num_f64(0.42)),
            RefOrLiteral::Literal(json_num_f64(0.58)),
            RefOrLiteral::Literal(JsonNumber(serde_json::Number::from(1))),
        ]);

        let css_value = token
            .to_css_value(&test_ctx())
            .expect("cubic-bezier should emit");

        assert_eq!(
            css_value,
            CssValue::Function {
                name: "cubic-bezier".to_string(),
                args: vec![
                    CssValue::Raw("0".to_string()),
                    CssValue::Raw("0.42".to_string()),
                    CssValue::Raw("0.58".to_string()),
                    CssValue::Raw("1".to_string()),
                ],
                separator: CssSeparator::Comma,
            }
        );
    }

    #[test]
    fn font_family_to_css_value_emits_single_value_list() {
        let token = FontFamilyTokenValue(FontFamilyValue::Single(RefOrLiteral::Literal(
            "Inter".to_string(),
        )));

        let css_value = token
            .to_css_value(&test_ctx())
            .expect("font-family should emit");

        assert_eq!(
            css_value,
            CssValue::List {
                separator: CssSeparator::Comma,
                items: vec![CssValue::Raw("Inter".to_string())],
            }
        );
    }

    #[test]
    fn font_family_to_css_value_emits_multiple_values_list() {
        let token = FontFamilyTokenValue(FontFamilyValue::Multiple(RefOrLiteral::Literal(
            FontFamilyMultipleValue(vec![
                RefOrLiteral::Literal("Inter".to_string()),
                RefOrLiteral::Literal("Arial".to_string()),
                RefOrLiteral::Literal("sans-serif".to_string()),
            ]),
        )));

        let css_value = token
            .to_css_value(&test_ctx())
            .expect("font-family should emit");

        assert_eq!(
            css_value,
            CssValue::List {
                separator: CssSeparator::Comma,
                items: vec![
                    CssValue::Raw("Inter".to_string()),
                    CssValue::Raw("Arial".to_string()),
                    CssValue::Raw("sans-serif".to_string()),
                ],
            }
        );
    }

    #[test]
    fn font_weight_to_css_value_emits_string_value() {
        let token = FontWeightTokenValue(RefOrLiteral::Literal(FontWeightValue::String(
            FontWeightValueString::Bold,
        )));

        let css_value = token
            .to_css_value(&test_ctx())
            .expect("font-weight should emit");

        assert_eq!(css_value, CssValue::Raw("bold".to_string()));
    }

    #[test]
    fn font_weight_to_css_value_emits_numeric_value() {
        let token = FontWeightTokenValue(RefOrLiteral::Literal(FontWeightValue::Number(
            FontWeightValueNumber(JsonNumber(serde_json::Number::from(700))),
        )));

        let css_value = token
            .to_css_value(&test_ctx())
            .expect("font-weight should emit");

        assert_eq!(css_value, CssValue::Raw("700".to_string()));
    }

    #[test]
    fn duration_to_css_value_emits_duration_node() {
        let token = DurationTokenValue {
            value: RefOrLiteral::Literal(DurationValue(json_num_f64(0.3))),
            unit: RefOrLiteral::Literal(DurationUnit::S),
        };

        let css_value = token
            .to_css_value(&test_ctx())
            .expect("duration should emit");

        assert_eq!(
            css_value,
            CssValue::Duration {
                value: "0.3".to_string(),
                unit: CssUnit::S,
            }
        );
    }

    #[test]
    fn border_to_css_value_emits_space_separated_shorthand() {
        let token = BorderTokenValue {
            color: BorderColor(RefAliasOrLiteral::Literal(ColorTokenValue {
                color_space: RefOrLiteral::Literal(ColorSpaceString::SRGB),
                components: RefOrLiteral::Literal(ColorComponentArray([
                    ColorComponentArrayElement::Number(JsonNumber(serde_json::Number::from(255))),
                    ColorComponentArrayElement::Number(JsonNumber(serde_json::Number::from(0))),
                    ColorComponentArrayElement::Number(JsonNumber(serde_json::Number::from(0))),
                ])),
                alpha: None,
                hex: None,
            })),
            width: BorderWidth(RefAliasOrLiteral::Literal(DimensionTokenValue {
                value: RefOrLiteral::Literal(DimensionValue(JsonNumber(serde_json::Number::from(
                    1,
                )))),
                unit: RefOrLiteral::Literal(DimensionUnit::Px),
            })),
            style: BorderStyle(RefAliasOrLiteral::Literal(StrokeStyleTokenValue::String(
                RefOrLiteral::Literal(StrokeStyleStringValue::Solid),
            ))),
        };

        let css_value = token.to_css_value(&test_ctx()).expect("border should emit");

        assert_eq!(
            css_value,
            CssValue::List {
                separator: CssSeparator::Space,
                items: vec![
                    CssValue::Function {
                        name: "rgb".to_string(),
                        args: vec![
                            CssValue::Raw("255".to_string()),
                            CssValue::Raw("0".to_string()),
                            CssValue::Raw("0".to_string()),
                        ],
                        separator: CssSeparator::Space,
                    },
                    CssValue::Dimension {
                        value: "1".to_string(),
                        unit: "px".to_string(),
                    },
                    CssValue::Ident("solid".to_string()),
                ],
            }
        );
    }

    #[test]
    fn border_to_css_value_errors_when_style_is_json_reference() {
        let token = BorderTokenValue {
            color: BorderColor(RefAliasOrLiteral::Literal(ColorTokenValue {
                color_space: RefOrLiteral::Literal(ColorSpaceString::SRGB),
                components: RefOrLiteral::Literal(ColorComponentArray([
                    ColorComponentArrayElement::Number(JsonNumber(serde_json::Number::from(255))),
                    ColorComponentArrayElement::Number(JsonNumber(serde_json::Number::from(0))),
                    ColorComponentArrayElement::Number(JsonNumber(serde_json::Number::from(0))),
                ])),
                alpha: None,
                hex: None,
            })),
            width: BorderWidth(RefAliasOrLiteral::Literal(DimensionTokenValue {
                value: RefOrLiteral::Literal(DimensionValue(JsonNumber(serde_json::Number::from(
                    1,
                )))),
                unit: RefOrLiteral::Literal(DimensionUnit::Px),
            })),
            style: BorderStyle(RefAliasOrLiteral::Ref(ref_value("#/tokens/stroke/solid"))),
        };

        let err = token
            .to_css_value(&test_ctx())
            .expect_err("json references are unsupported during css emission");

        assert!(matches!(
            err,
            CssEmitError::UnexpectedJsonReference { pointer }
            if pointer == "/tokens/stroke/solid"
        ));
    }

    #[test]
    fn stroke_style_string_to_css_value_emits_ident() {
        let token =
            StrokeStyleTokenValue::String(RefOrLiteral::Literal(StrokeStyleStringValue::Dashed));

        let css_value = token
            .to_css_value(&test_ctx())
            .expect("stroke style string should emit");

        assert_eq!(css_value, CssValue::Ident("dashed".to_string()));
    }

    #[test]
    fn stroke_style_to_css_value_flattens_dash_array_and_line_cap() {
        let token = StrokeStyleTokenValue::Object(RefOrLiteral::Literal(StrokeStyleObjectValue {
            dash_array: RefOrLiteral::Literal(StrokeStyleDashArrayValue(vec![
                RefAliasOrLiteral::Literal(DimensionTokenValue {
                    value: RefOrLiteral::Literal(DimensionValue(JsonNumber(
                        serde_json::Number::from(2),
                    ))),
                    unit: RefOrLiteral::Literal(DimensionUnit::Px),
                }),
                RefAliasOrLiteral::Literal(DimensionTokenValue {
                    value: RefOrLiteral::Literal(DimensionValue(JsonNumber(
                        serde_json::Number::from(4),
                    ))),
                    unit: RefOrLiteral::Literal(DimensionUnit::Px),
                }),
            ])),
            line_cap: RefOrLiteral::Literal(StrokeStyleLineCapValue::Round),
        }));

        let css_value = token
            .to_css_value(&test_ctx())
            .expect("stroke style should emit");

        assert_eq!(
            css_value,
            CssValue::List {
                separator: CssSeparator::Space,
                items: vec![
                    CssValue::Dimension {
                        value: "2".to_string(),
                        unit: "px".to_string(),
                    },
                    CssValue::Dimension {
                        value: "4".to_string(),
                        unit: "px".to_string(),
                    },
                    CssValue::Ident("round".to_string()),
                ],
            }
        );
    }

    #[test]
    fn stroke_style_to_css_value_errors_when_object_is_json_reference() {
        let token =
            StrokeStyleTokenValue::Object(RefOrLiteral::Ref(ref_value("#/tokens/stroke/object")));

        let err = token
            .to_css_value(&test_ctx())
            .expect_err("json references are unsupported during css emission");

        assert!(matches!(
            err,
            CssEmitError::UnexpectedJsonReference { pointer }
            if pointer == "/tokens/stroke/object"
        ));
    }

    #[test]
    fn border_to_css_declarations_emits_single_shorthand_when_allowed() {
        let token = BorderTokenValue {
            color: BorderColor(RefAliasOrLiteral::Literal(ColorTokenValue {
                color_space: RefOrLiteral::Literal(ColorSpaceString::SRGB),
                components: RefOrLiteral::Literal(ColorComponentArray([
                    ColorComponentArrayElement::Number(JsonNumber(serde_json::Number::from(255))),
                    ColorComponentArrayElement::Number(JsonNumber(serde_json::Number::from(0))),
                    ColorComponentArrayElement::Number(JsonNumber(serde_json::Number::from(0))),
                ])),
                alpha: None,
                hex: None,
            })),
            width: BorderWidth(RefAliasOrLiteral::Literal(DimensionTokenValue {
                value: RefOrLiteral::Literal(DimensionValue(JsonNumber(serde_json::Number::from(
                    1,
                )))),
                unit: RefOrLiteral::Literal(DimensionUnit::Px),
            })),
            style: BorderStyle(RefAliasOrLiteral::Literal(StrokeStyleTokenValue::String(
                RefOrLiteral::Literal(StrokeStyleStringValue::Solid),
            ))),
        };

        let declarations = token
            .to_css_declarations(
                &CssProperty::Custom("--border".to_string()),
                &test_ctx_with_composites(CssCompositeOptions {
                    expand_border: false,
                    ..CssCompositeOptions::default()
                }),
            )
            .expect("border declarations should emit");

        assert_eq!(declarations.len(), 1);
        assert!(matches!(
            declarations[0].property,
            CssProperty::Custom(ref name) if name == "--border"
        ));
    }

    #[test]
    fn border_to_css_declarations_expands_when_style_object_prevents_shorthand() {
        let token = BorderTokenValue {
            color: BorderColor(RefAliasOrLiteral::Literal(ColorTokenValue {
                color_space: RefOrLiteral::Literal(ColorSpaceString::SRGB),
                components: RefOrLiteral::Literal(ColorComponentArray([
                    ColorComponentArrayElement::Number(JsonNumber(serde_json::Number::from(255))),
                    ColorComponentArrayElement::Number(JsonNumber(serde_json::Number::from(0))),
                    ColorComponentArrayElement::Number(JsonNumber(serde_json::Number::from(0))),
                ])),
                alpha: None,
                hex: None,
            })),
            width: BorderWidth(RefAliasOrLiteral::Literal(DimensionTokenValue {
                value: RefOrLiteral::Literal(DimensionValue(JsonNumber(serde_json::Number::from(
                    1,
                )))),
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
                        RefAliasOrLiteral::Literal(DimensionTokenValue {
                            value: RefOrLiteral::Literal(DimensionValue(JsonNumber(
                                serde_json::Number::from(4),
                            ))),
                            unit: RefOrLiteral::Literal(DimensionUnit::Px),
                        }),
                    ])),
                    line_cap: RefOrLiteral::Literal(StrokeStyleLineCapValue::Round),
                }),
            ))),
        };

        let declarations = token
            .to_css_declarations(
                &CssProperty::Custom("--border".to_string()),
                &test_ctx_with_composites(CssCompositeOptions {
                    expand_border: false,
                    expand_stroke_style: false,
                    ..CssCompositeOptions::default()
                }),
            )
            .expect("border declarations should expand when style is object");

        assert_eq!(declarations.len(), 4);
        assert!(matches!(
            declarations[0].property,
            CssProperty::Custom(ref name) if name == "--border-width"
        ));
        assert!(matches!(
            declarations[1].property,
            CssProperty::Custom(ref name) if name == "--border-color"
        ));
        assert!(matches!(
            declarations[2].property,
            CssProperty::Custom(ref name) if name == "--border-style-dash-array"
        ));
        assert!(matches!(
            declarations[3].property,
            CssProperty::Custom(ref name) if name == "--border-style-line-cap"
        ));
    }

    #[test]
    fn stroke_style_to_css_declarations_expands_object_into_dash_array_and_line_cap() {
        let token = StrokeStyleTokenValue::Object(RefOrLiteral::Literal(StrokeStyleObjectValue {
            dash_array: RefOrLiteral::Literal(StrokeStyleDashArrayValue(vec![
                RefAliasOrLiteral::Literal(DimensionTokenValue {
                    value: RefOrLiteral::Literal(DimensionValue(JsonNumber(
                        serde_json::Number::from(2),
                    ))),
                    unit: RefOrLiteral::Literal(DimensionUnit::Px),
                }),
                RefAliasOrLiteral::Literal(DimensionTokenValue {
                    value: RefOrLiteral::Literal(DimensionValue(JsonNumber(
                        serde_json::Number::from(4),
                    ))),
                    unit: RefOrLiteral::Literal(DimensionUnit::Px),
                }),
            ])),
            line_cap: RefOrLiteral::Literal(StrokeStyleLineCapValue::Round),
        }));

        let declarations = token
            .to_css_declarations(
                &CssProperty::Custom("--stroke-style".to_string()),
                &test_ctx_with_composites(CssCompositeOptions {
                    expand_stroke_style: false,
                    ..CssCompositeOptions::default()
                }),
            )
            .expect("stroke style object should expand");

        assert_eq!(declarations.len(), 2);
        assert!(matches!(
            declarations[0].property,
            CssProperty::Custom(ref name) if name == "--stroke-style-dash-array"
        ));
        assert!(matches!(
            declarations[1].property,
            CssProperty::Custom(ref name) if name == "--stroke-style-line-cap"
        ));
    }

    #[test]
    fn transition_to_css_declarations_expands_when_enabled() {
        let token = TransitionTokenValue {
            duration: TransitionDuration(RefAliasOrLiteral::Literal(DurationTokenValue {
                value: RefOrLiteral::Literal(DurationValue(JsonNumber(serde_json::Number::from(
                    150,
                )))),
                unit: RefOrLiteral::Literal(DurationUnit::Ms),
            })),
            delay: TransitionDelay(RefAliasOrLiteral::Literal(DurationTokenValue {
                value: RefOrLiteral::Literal(DurationValue(JsonNumber(serde_json::Number::from(
                    75,
                )))),
                unit: RefOrLiteral::Literal(DurationUnit::Ms),
            })),
            timing_function: TransitionTimingFunction(RefAliasOrLiteral::Literal(
                CubicBezierTokenValue([
                    RefOrLiteral::Literal(JsonNumber(serde_json::Number::from(0))),
                    RefOrLiteral::Literal(json_num_f64(0.42)),
                    RefOrLiteral::Literal(json_num_f64(1.0)),
                    RefOrLiteral::Literal(json_num_f64(1.0)),
                ]),
            )),
        };

        let declarations = token
            .to_css_declarations(
                &CssProperty::Custom("--motion".to_string()),
                &test_ctx_with_composites(CssCompositeOptions {
                    expand_transition: true,
                    ..CssCompositeOptions::default()
                }),
            )
            .expect("transition declarations should expand");

        assert_eq!(declarations.len(), 3);
        assert!(matches!(
            declarations[0].property,
            CssProperty::Custom(ref name) if name == "--motion-duration"
        ));
        assert!(matches!(
            declarations[1].property,
            CssProperty::Custom(ref name) if name == "--motion-timing-function"
        ));
        assert!(matches!(
            declarations[2].property,
            CssProperty::Custom(ref name) if name == "--motion-delay"
        ));
    }

    #[test]
    fn shadow_to_css_declarations_expands_layers_when_enabled() {
        let token = ShadowTokenValue(RefOrLiteral::Literal(vec![RefAliasOrLiteral::Literal(
            ShadowObjectValue {
                color: ShadowObjectColor(RefAliasOrLiteral::Literal(ColorTokenValue {
                    color_space: RefOrLiteral::Literal(ColorSpaceString::SRGB),
                    components: RefOrLiteral::Literal(ColorComponentArray([
                        ColorComponentArrayElement::Number(JsonNumber(serde_json::Number::from(0))),
                        ColorComponentArrayElement::Number(JsonNumber(serde_json::Number::from(0))),
                        ColorComponentArrayElement::Number(JsonNumber(serde_json::Number::from(0))),
                    ])),
                    alpha: None,
                    hex: None,
                })),
                offset_x: ShadowObjectOffsetX(RefAliasOrLiteral::Literal(DimensionTokenValue {
                    value: RefOrLiteral::Literal(DimensionValue(JsonNumber(
                        serde_json::Number::from(1),
                    ))),
                    unit: RefOrLiteral::Literal(DimensionUnit::Px),
                })),
                offset_y: ShadowObjectOffsetY(RefAliasOrLiteral::Literal(DimensionTokenValue {
                    value: RefOrLiteral::Literal(DimensionValue(JsonNumber(
                        serde_json::Number::from(2),
                    ))),
                    unit: RefOrLiteral::Literal(DimensionUnit::Px),
                })),
                blur: ShadowObjectBlur(RefAliasOrLiteral::Literal(DimensionTokenValue {
                    value: RefOrLiteral::Literal(DimensionValue(JsonNumber(
                        serde_json::Number::from(4),
                    ))),
                    unit: RefOrLiteral::Literal(DimensionUnit::Px),
                })),
                spread: ShadowObjectSpread(RefAliasOrLiteral::Literal(DimensionTokenValue {
                    value: RefOrLiteral::Literal(DimensionValue(JsonNumber(
                        serde_json::Number::from(0),
                    ))),
                    unit: RefOrLiteral::Literal(DimensionUnit::Px),
                })),
                inset: Some(ShadowObjectInset(RefOrLiteral::Literal(true))),
            },
        )]));

        let declarations = token
            .to_css_declarations(
                &CssProperty::Custom("--shadow".to_string()),
                &test_ctx_with_composites(CssCompositeOptions {
                    expand_shadow: true,
                    ..CssCompositeOptions::default()
                }),
            )
            .expect("shadow declarations should expand");

        assert_eq!(declarations.len(), 6);
        assert!(matches!(
            declarations[0].property,
            CssProperty::Custom(ref name) if name == "--shadow-layer-0-offset-x"
        ));
        assert!(matches!(
            declarations[5].property,
            CssProperty::Custom(ref name) if name == "--shadow-layer-0-inset"
        ));
    }

    #[test]
    fn gradient_to_css_declarations_expands_stops_when_enabled() {
        let token = GradientTokenValue(RefOrLiteral::Literal(vec![
            RefAliasOrLiteral::Literal(GradientObject {
                color: GradientObjectColor(RefAliasOrLiteral::Literal(ColorTokenValue {
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
                position: GradientObjectPosition(RefOrLiteral::Literal(json_num_f64(0.0))),
            }),
            RefAliasOrLiteral::Literal(GradientObject {
                color: GradientObjectColor(RefAliasOrLiteral::Literal(ColorTokenValue {
                    color_space: RefOrLiteral::Literal(ColorSpaceString::SRGB),
                    components: RefOrLiteral::Literal(ColorComponentArray([
                        ColorComponentArrayElement::Number(JsonNumber(serde_json::Number::from(0))),
                        ColorComponentArrayElement::Number(JsonNumber(serde_json::Number::from(0))),
                        ColorComponentArrayElement::Number(JsonNumber(serde_json::Number::from(
                            255,
                        ))),
                    ])),
                    alpha: None,
                    hex: None,
                })),
                position: GradientObjectPosition(RefOrLiteral::Literal(json_num_f64(1.0))),
            }),
        ]));

        let declarations = token
            .to_css_declarations(
                &CssProperty::Custom("--gradient".to_string()),
                &test_ctx_with_composites(CssCompositeOptions {
                    expand_gradient: true,
                    ..CssCompositeOptions::default()
                }),
            )
            .expect("gradient declarations should expand");

        assert_eq!(declarations.len(), 4);
        assert!(matches!(
            declarations[0].property,
            CssProperty::Custom(ref name) if name == "--gradient-stop-0-color"
        ));
        assert!(matches!(
            declarations[3].property,
            CssProperty::Custom(ref name) if name == "--gradient-stop-1-position"
        ));
    }

    #[test]
    fn gradient_to_css_value_emits_comma_separated_stops() {
        let token = GradientTokenValue(RefOrLiteral::Literal(vec![
            RefAliasOrLiteral::Literal(GradientObject {
                color: GradientObjectColor(RefAliasOrLiteral::Literal(ColorTokenValue {
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
                position: GradientObjectPosition(RefOrLiteral::Literal(json_num_f64(0.0))),
            }),
            RefAliasOrLiteral::Literal(GradientObject {
                color: GradientObjectColor(RefAliasOrLiteral::Literal(ColorTokenValue {
                    color_space: RefOrLiteral::Literal(ColorSpaceString::SRGB),
                    components: RefOrLiteral::Literal(ColorComponentArray([
                        ColorComponentArrayElement::Number(JsonNumber(serde_json::Number::from(0))),
                        ColorComponentArrayElement::Number(JsonNumber(serde_json::Number::from(0))),
                        ColorComponentArrayElement::Number(JsonNumber(serde_json::Number::from(
                            255,
                        ))),
                    ])),
                    alpha: None,
                    hex: None,
                })),
                position: GradientObjectPosition(RefOrLiteral::Literal(json_num_f64(1.0))),
            }),
        ]));

        let css_value = token
            .to_css_value(&test_ctx())
            .expect("gradient should emit");

        assert!(matches!(
            css_value,
            CssValue::List {
                separator: CssSeparator::Comma,
                ..
            }
        ));
    }

    #[test]
    fn shadow_to_css_value_emits_comma_separated_shadow_list() {
        let token = ShadowTokenValue(RefOrLiteral::Literal(vec![RefAliasOrLiteral::Literal(
            ShadowObjectValue {
                color: ShadowObjectColor(RefAliasOrLiteral::Literal(ColorTokenValue {
                    color_space: RefOrLiteral::Literal(ColorSpaceString::SRGB),
                    components: RefOrLiteral::Literal(ColorComponentArray([
                        ColorComponentArrayElement::Number(JsonNumber(serde_json::Number::from(0))),
                        ColorComponentArrayElement::Number(JsonNumber(serde_json::Number::from(0))),
                        ColorComponentArrayElement::Number(JsonNumber(serde_json::Number::from(0))),
                    ])),
                    alpha: None,
                    hex: None,
                })),
                offset_x: ShadowObjectOffsetX(RefAliasOrLiteral::Literal(DimensionTokenValue {
                    value: RefOrLiteral::Literal(DimensionValue(JsonNumber(
                        serde_json::Number::from(1),
                    ))),
                    unit: RefOrLiteral::Literal(DimensionUnit::Px),
                })),
                offset_y: ShadowObjectOffsetY(RefAliasOrLiteral::Literal(DimensionTokenValue {
                    value: RefOrLiteral::Literal(DimensionValue(JsonNumber(
                        serde_json::Number::from(2),
                    ))),
                    unit: RefOrLiteral::Literal(DimensionUnit::Px),
                })),
                blur: ShadowObjectBlur(RefAliasOrLiteral::Literal(DimensionTokenValue {
                    value: RefOrLiteral::Literal(DimensionValue(JsonNumber(
                        serde_json::Number::from(4),
                    ))),
                    unit: RefOrLiteral::Literal(DimensionUnit::Px),
                })),
                spread: ShadowObjectSpread(RefAliasOrLiteral::Literal(DimensionTokenValue {
                    value: RefOrLiteral::Literal(DimensionValue(JsonNumber(
                        serde_json::Number::from(0),
                    ))),
                    unit: RefOrLiteral::Literal(DimensionUnit::Px),
                })),
                inset: Some(ShadowObjectInset(RefOrLiteral::Literal(true))),
            },
        )]));

        let css_value = token.to_css_value(&test_ctx()).expect("shadow should emit");

        assert!(matches!(
            css_value,
            CssValue::List {
                separator: CssSeparator::Comma,
                ..
            }
        ));
    }

    #[test]
    fn transition_to_css_value_emits_duration_timing_delay_sequence() {
        let token = TransitionTokenValue {
            duration: TransitionDuration(RefAliasOrLiteral::Literal(DurationTokenValue {
                value: RefOrLiteral::Literal(DurationValue(JsonNumber(serde_json::Number::from(
                    150,
                )))),
                unit: RefOrLiteral::Literal(DurationUnit::Ms),
            })),
            delay: TransitionDelay(RefAliasOrLiteral::Literal(DurationTokenValue {
                value: RefOrLiteral::Literal(DurationValue(JsonNumber(serde_json::Number::from(
                    75,
                )))),
                unit: RefOrLiteral::Literal(DurationUnit::Ms),
            })),
            timing_function: TransitionTimingFunction(RefAliasOrLiteral::Literal(
                CubicBezierTokenValue([
                    RefOrLiteral::Literal(JsonNumber(serde_json::Number::from(0))),
                    RefOrLiteral::Literal(json_num_f64(0.42)),
                    RefOrLiteral::Literal(json_num_f64(1.0)),
                    RefOrLiteral::Literal(json_num_f64(1.0)),
                ]),
            )),
        };

        let css_value = token
            .to_css_value(&test_ctx())
            .expect("transition should emit");

        assert!(matches!(
            css_value,
            CssValue::List {
                separator: CssSeparator::Space,
                ..
            }
        ));
    }

    #[test]
    fn typography_to_css_value_emits_space_separated_composite_value() {
        let token = TypographyTokenValue {
            font_family: TypographyFontFamily(RefAliasOrLiteral::Literal(FontFamilyTokenValue(
                FontFamilyValue::Single(RefOrLiteral::Literal("Inter".to_string())),
            ))),
            font_size: TypographyFontSize(RefAliasOrLiteral::Literal(DimensionTokenValue {
                value: RefOrLiteral::Literal(DimensionValue(JsonNumber(serde_json::Number::from(
                    16,
                )))),
                unit: RefOrLiteral::Literal(DimensionUnit::Px),
            })),
            font_weight: TypographyFontWeight(RefAliasOrLiteral::Literal(FontWeightTokenValue(
                RefOrLiteral::Literal(FontWeightValue::String(FontWeightValueString::Bold)),
            ))),
            letter_spacing: TypographyLetterSpacing(RefAliasOrLiteral::Literal(
                DimensionTokenValue {
                    value: RefOrLiteral::Literal(DimensionValue(JsonNumber(
                        serde_json::Number::from(1),
                    ))),
                    unit: RefOrLiteral::Literal(DimensionUnit::Px),
                },
            )),
            line_height: TypographyLineHeight(RefAliasOrLiteral::Literal(NumberTokenValue(
                RefOrLiteral::Literal(JsonNumber(json_num_f64(1.5).0)),
            ))),
        };

        let css_value = token
            .to_css_value(&test_ctx())
            .expect("typography should emit");

        assert!(matches!(
            css_value,
            CssValue::List {
                separator: CssSeparator::Space,
                ..
            }
        ));
    }
}
