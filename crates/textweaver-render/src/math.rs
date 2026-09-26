//! LaTeX math to MathML with `pulldown-latex`, so screen readers (NVDA with
//! MathCAT, JAWS, VoiceOver) can read and navigate equations.
//!
//! The LaTeX source rides along as an `<annotation encoding="application/x-tex">`
//! so it can be copied. A formula the parser cannot handle is shown as its
//! source in `<code class="math-error">`; a panic inside the math renderer
//! is caught and handled the same way, so one bad formula never stops a
//! batch.

use pulldown_latex::config::DisplayMode;
use pulldown_latex::{Parser, RenderConfig, Storage, push_mathml};

use crate::escape_html;

/// MathML for `latex`, inline or as a display (block) formula.
pub fn to_mathml(latex: &str, display: bool) -> String {
    let rendered = std::panic::catch_unwind(|| {
        let storage = Storage::new();
        let parser = Parser::new(latex, &storage);
        let config = RenderConfig {
            display_mode: if display {
                DisplayMode::Block
            } else {
                DisplayMode::Inline
            },
            annotation: Some(latex),
            xml: true,
            ..RenderConfig::default()
        };
        let mut out = String::new();
        push_mathml(&mut out, parser, config).map(|()| out)
    });
    match rendered {
        Ok(Ok(mathml)) if !mathml.contains("<merror") => mathml,
        _ => {
            log::debug!("math fallback for {latex:?}");
            let class = if display {
                "math-error math-display"
            } else {
                "math-error"
            };
            format!("<code class=\"{class}\">{}</code>", escape_html(latex))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_and_display() {
        let m = to_mathml("x^2", false);
        assert!(m.starts_with("<math"), "{m}");
        assert!(m.contains("<msup>"), "{m}");
        assert!(m.contains("application/x-tex"), "{m}");
        let d = to_mathml(r"\frac{a}{b}", true);
        assert!(d.contains("display=\"block\""), "{d}");
        assert!(d.contains("<mfrac>"), "{d}");
    }

    #[test]
    fn bad_latex_falls_back_to_code() {
        let m = to_mathml(r"\frac{", false);
        assert!(m.starts_with("<code class=\"math-error\">"), "{m}");
    }
}
