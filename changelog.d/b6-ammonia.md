### Security and privacy
- **ammonia 4.1.9.** The library that cleans HTML for the browser preview and for sanitized conversion had a flaw: embedded SVG or MathML could carry markup past it (RUSTSEC-2026-0533). textweaver now uses the fixed release.
- **The conversion test matrix covers caption files.** Every built-in loader must have a fixture there; the new SRT and WebVTT loader has one.
