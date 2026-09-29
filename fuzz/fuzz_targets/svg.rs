//! Fuzz target: SVG drawings and MathML formulas (ADR-0044). The bytes are
//! loaded as an `.svg` file (roxmltree, with its nesting and element
//! limits), as an `.mml` formula (presentation or content MathML), and
//! inline in a web page, where the HTML parser gives the same drawing
//! reader its tree.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    textweaver_fuzz::load_checked(data, "svg");
    textweaver_fuzz::load_checked(data, "mml");
    let text = String::from_utf8_lossy(data);
    let page = format!("<p>Before <svg>{text}</svg> <math>{text}</math> after.</p>");
    textweaver_fuzz::load_checked(page.as_bytes(), "html");
});
