# Research for the next waves

This folder holds research and planning material for the alpha releases after 0.1.0-alpha.7. It is for the maintainer and for contributors planning a wave of work. Nothing here is a decision: a decision gets an ADR in [adr/](../../adr/README.md). Each report ends with a ranked list of improvements, sized and placed in alpha.8, alpha.9, or later, with a performance note. Performance is a requirement ([ADR-0001](../../adr/0001-workspace-and-dependencies.md)); every suggestion says what it costs.

The material was gathered on Friday, October 2, 2026, by several research agents working in parallel, one topic each, and then read together into the wave plan.

## The reports

- [Next waves plan](next-waves-plan.md): the synthesis. What alpha.8, alpha.9, and later should hold, in order, with the measurement that proves each item.
- Performance audit (`performance-audit.md`, being written): a code-level audit of the hot paths (loading, the narration plan, segmentation, the speech thread, the GUI document widget, startup, the build profile), with a ranked plan.
- [Speech engines and runtimes](speech-engines-and-runtimes.md): neural voices and formant engines that run offline on a laptop, the Rust inference runtimes, and the speech-path latency work.
- [Text-to-speech use cases](tts-use-cases.md): the research evidence on text-to-speech for students with disabilities, the product landscape, and the lessons for textweaver.
- [Health sciences use cases](health-sciences-use-cases.md): the materials, pronunciation, numbers and units, and study workflows of medical, nursing, dental, and pharmacy students.
- [GUI and visual design](gui-and-visual-design.md): how to make the window and the terminal reader better looking without giving up accessibility or frame time.
- [Law, standards, and conformance](legal-and-standards.md): the ADA, Section 504 and 508, WCAG 2.2, EN 301 549, EPUB Accessibility, PDF/UA, and what textweaver and accommodations offices need from it.

A report listed here that is not yet in the folder is still being written; it lands in the same pull request.

## How to use this folder

- Pick items from the wave plan, not from a single report: the plan weighs each report's list against the others and against the roadmap.
- Before starting an item, record the benchmark it names (`cargo xtask bench --quick --json before.json`), and record it again after.
- When an item changes an architecture decision, write an ADR and link it from the plan.
- Keep the reports as they were written; add a dated status update line at the top of a report when an item lands, as the ADRs do.

## See also

- [Roadmap](../../roadmap.md): the public summary of what comes next.
- [Star features not yet planned](../../star-gaps.md): the feature comparison with Star.
- [Testing](../testing.md): the benchmarks and their history.
- [Architecture](../architecture.md): the crates, threads, and the path from a file to a spoken, highlighted word.
- [Documentation index](../../README.md)
