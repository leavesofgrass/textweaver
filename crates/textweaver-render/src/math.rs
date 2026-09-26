//! Math to MathML with `textweaver-math` (ADR-0018), so screen readers
//! (NVDA with MathCAT, JAWS, VoiceOver) can read and navigate equations.
//!
//! LaTeX comes from the engines' math events (`$…$`, `$$…$$`); ASCIIMath
//! from fenced code blocks marked `asciimath` (or `am`) and, when
//! [`RenderOptions::asciimath`](crate::RenderOptions) is on, from inline
//! code spans. The source rides along as `alttext` and as an
//! `<annotation>` (`application/x-tex` or `text/x-asciimath`) so it can be
//! copied. A formula the parser had to repair (an unknown command, a
//! missing brace) is shown as its source in `<code class="math-error">`
//! rather than as a half-right formula; a panic inside the math crate is
//! caught and handled the same way, so one bad formula never stops a batch.

use textweaver_math::{Math, MathMlOptions, parse_asciimath, parse_latex};

use crate::escape_html;

/// MathML for LaTeX `src`, inline or as a display (block) formula.
pub fn to_mathml_latex(src: &str, display: bool) -> String {
    render(src, display, parse_latex)
}

/// MathML for ASCIIMath `src`, inline or as a display (block) formula.
pub fn to_mathml_asciimath(src: &str, display: bool) -> String {
    render(src, display, parse_asciimath)
}

/// MathML for LaTeX `src` (the name kept from the `pulldown-latex` days).
pub fn to_mathml(src: &str, display: bool) -> String {
    to_mathml_latex(src, display)
}

fn render(src: &str, display: bool, parse: fn(&str) -> Math) -> String {
    let rendered = std::panic::catch_unwind(|| {
        let math = parse(src);
        math.is_clean()
            .then(|| textweaver_math::to_mathml(&math, &MathMlOptions::new(display)))
    });
    match rendered {
        Ok(Some(mathml)) => mathml,
        _ => {
            log::debug!("math fallback for {src:?}");
            let class = if display {
                "math-error math-display"
            } else {
                "math-error"
            };
            format!("<code class=\"{class}\">{}</code>", escape_html(src))
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
        assert!(m.contains("alttext=\"x^2\""), "{m}");
        assert!(!m.contains("display="), "{m}");
        let d = to_mathml(r"\frac{a}{b}", true);
        assert!(d.contains("display=\"block\""), "{d}");
        assert!(d.contains("<mfrac>"), "{d}");
    }

    #[test]
    fn asciimath() {
        let m = to_mathml_asciimath("sum_(i=1)^n i^3", true);
        assert!(m.contains("display=\"block\""), "{m}");
        assert!(m.contains("<munderover>"), "{m}");
        assert!(m.contains("text/x-asciimath"), "{m}");
    }

    #[test]
    fn bad_latex_falls_back_to_code() {
        let m = to_mathml(r"\frac{", false);
        assert!(m.starts_with("<code class=\"math-error\">"), "{m}");
        let m = to_mathml(r"\nosuchcommand{x} <b>", true);
        assert_eq!(
            m,
            "<code class=\"math-error math-display\">\\nosuchcommand{x} &lt;b&gt;</code>"
        );
    }
}
