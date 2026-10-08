use ratex_font::{get_global_metrics, MathConstants};
use ratex_types::color::Color;
use ratex_types::math_style::MathStyle;

/// Optional host shaping of text before mathematical placement.
///
/// A host can inspect the offered AST slice and return `None` for unsupported
/// runs. See `docs/HOST_TEXT_LAYOUT.md` for the coordinate and sizing contract.
pub trait TextLayout {
    /// Return a measured text box in local em units, or retain bundled fonts.
    fn layout(
        &self,
        body: &[ratex_parser::ParseNode],
        options: &LayoutOptions,
    ) -> Option<crate::LayoutBox>;
}

/// Layout options passed through the layout tree.
#[derive(Clone)]
pub struct LayoutOptions<'a> {
    /// Borrowed for this layout call; absent by default.
    pub text_layout: Option<&'a dyn TextLayout>,
    /// Explicit text-command weight, or `None` to use the host default.
    pub text_weight: Option<u16>,
    /// Explicit text-command slant, or `None` to use the host default.
    pub text_italic: Option<bool>,
    pub style: MathStyle,
    pub color: Color,
    /// When set (e.g. in align/aligned), cap relation spacing to this many mu for consistency.
    pub align_relation_spacing: Option<f64>,
    /// When inside \\left...\\right, the stretch height for \\middle delimiters (second pass only).
    pub leftright_delim_height: Option<f64>,
    /// Extra horizontal kern between glyphs (em), e.g. for custom text tracking.
    pub inter_glyph_kern_em: f64,
    /// Explicit TeX sizing multiplier currently in force (`1.0` for `\normalsize`).
    ///
    /// This is layout state propagated through `\tiny` ... `\Huge`; callers
    /// should normally leave it at its default.
    pub explicit_size_multiplier: f64,
}

impl Default for LayoutOptions<'_> {
    fn default() -> Self {
        Self {
            text_layout: None,
            text_weight: None,
            text_italic: None,
            style: MathStyle::Display,
            color: Color::BLACK,
            align_relation_spacing: None,
            leftright_delim_height: None,
            inter_glyph_kern_em: 0.0,
            explicit_size_multiplier: 1.0,
        }
    }
}

impl LayoutOptions<'_> {
    /// Derive text face state while retaining the other layout options.
    pub fn with_text_font(&self, command: Option<&str>) -> Self {
        let mut result = self.clone();
        match command.unwrap_or("").trim_start_matches('\\') {
            "textbf" => result.text_weight = Some(700),
            "textmd" => result.text_weight = Some(400),
            "textit" | "emph" => result.text_italic = Some(true),
            "textup" => result.text_italic = Some(false),
            "textnormal" => {
                result.text_weight = Some(400);
                result.text_italic = Some(false);
            }
            _ => {}
        }
        result
    }

    pub fn metrics(&self) -> &'static MathConstants {
        get_global_metrics(self.style.size_index())
    }

    pub fn size_multiplier(&self) -> f64 {
        self.style.size_multiplier()
    }

    pub fn with_style(&self, style: MathStyle) -> Self {
        Self {
            text_layout: self.text_layout,
            text_weight: self.text_weight,
            text_italic: self.text_italic,
            style,
            color: self.color,
            align_relation_spacing: self.align_relation_spacing,
            leftright_delim_height: self.leftright_delim_height,
            inter_glyph_kern_em: self.inter_glyph_kern_em,
            explicit_size_multiplier: self.explicit_size_multiplier,
        }
    }

    pub fn with_color(&self, color: Color) -> Self {
        Self {
            text_layout: self.text_layout,
            text_weight: self.text_weight,
            text_italic: self.text_italic,
            style: self.style,
            color,
            align_relation_spacing: self.align_relation_spacing,
            leftright_delim_height: self.leftright_delim_height,
            inter_glyph_kern_em: self.inter_glyph_kern_em,
            explicit_size_multiplier: self.explicit_size_multiplier,
        }
    }

    pub fn with_inter_glyph_kern(&self, em: f64) -> Self {
        Self {
            inter_glyph_kern_em: em,
            ..self.clone()
        }
    }

    pub(crate) fn with_explicit_size_multiplier(&self, multiplier: f64) -> Self {
        Self {
            explicit_size_multiplier: multiplier,
            ..self.clone()
        }
    }
}

impl std::fmt::Debug for LayoutOptions<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LayoutOptions")
            .field("style", &self.style)
            .field("color", &self.color)
            .field("text_layout", &self.text_layout.is_some())
            .field("text_weight", &self.text_weight)
            .field("text_italic", &self.text_italic)
            .field("align_relation_spacing", &self.align_relation_spacing)
            .field("leftright_delim_height", &self.leftright_delim_height)
            .field("inter_glyph_kern_em", &self.inter_glyph_kern_em)
            .field("explicit_size_multiplier", &self.explicit_size_multiplier)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_inter_glyph_kern_option_survives_option_derivation() {
        let options = LayoutOptions::default()
            .with_inter_glyph_kern(0.02)
            .with_style(MathStyle::Script)
            .with_color(Color::new(0.1, 0.2, 0.3, 1.0));

        assert_eq!(options.inter_glyph_kern_em, 0.02);
        assert_eq!(options.explicit_size_multiplier, 1.0);
    }
}
