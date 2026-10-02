# Law, standards, and conformance: what textweaver and its users must meet

This note maps the laws, regulations, and technical standards that surround textweaver and the people who use it: students with print disabilities, and the accommodations staff who convert course materials for them. It says what each rule requires, who it binds, when it applies, and what textweaver should do about it. It is for the maintainer, for contributors who touch the writers, the GUI, or the converter, and for an accommodations office deciding whether textweaver fits its compliance file. It was written on Friday, October 2, 2026, from the documents named in each section. Nothing here is legal advice. Where a claim could not be checked against its primary source, it says "unverified".

How this was checked: the research container could not reach eCFR, the Federal Register, govinfo, ed.gov, ada.gov, w3.org, iso.org, daisy.org, brailleauthority.org, or etsi.org (the egress proxy blocked them). Every primary link below is the standard address for that document, and its content was confirmed through search results and secondary summaries of it. Each legal date should be read once more against the primary link before it goes into a compliance file.

## 1. United States law as of October 2026

### ADA Title II and the 2024 web and mobile rule

Title II of the Americans with Disabilities Act binds state and local governments, including public colleges and universities. On Wednesday, April 24, 2024, the Department of Justice published a final rule adding subpart H to 28 CFR part 35 [1]. It names WCAG 2.1 Level AA as the technical standard for web content and mobile apps that a public entity provides or makes available, including through a contractor [2]. "Conventional electronic documents" are defined in 28 CFR 35.104 as PDF, word processor, presentation, and spreadsheet files, so a course PDF on an LMS is web content under the rule [3].

The exceptions in 28 CFR 35.201 are: archived web content (kept only for reference, not altered, in a clearly marked archive, and created before the compliance date); preexisting conventional electronic documents, unless they are used to apply for, gain access to, or take part in a service; content posted by a third party that is not acting for the entity; individualized, password-protected documents about one person, property, or account; and preexisting social media posts [4]. Course materials behind an LMS login are not excepted: the exception for password-protected documents covers documents about a specific individual, not a class [4]. A conforming alternate version is allowed only where technical or legal limits stop direct conformance (28 CFR 35.202), and an entity may show that full conformance would be a fundamental alteration or an undue burden (28 CFR 35.204) [1].

The compliance dates were April 24, 2026 for entities with a total population of 50,000 or more and April 26, 2027 for smaller entities and special districts [1]. On Monday, April 20, 2026, the Department published an interim final rule that moved them to April 26, 2027 and April 26, 2028 [5]. The stated reasons were resource constraints, staffing, slower remediation technology than expected, and litigation risk [5]. On Thursday, May 21, 2026, the National Federation of the Blind sued the Department of Justice and the Department of Health and Human Services in the District of Maryland (case 1:26-cv-02007-RDB) under the Administrative Procedure Act, asking the court to set aside both extensions [6]. As of mid September 2026 there was no ruling (unverified beyond the secondary reports in [6]). Nothing in the interim rule changes the standard or the exceptions; only the dates moved.

Where OHSU falls: ORS 353.020 makes OHSU a public corporation and a governmental entity [7]. A public corporation that performs governmental functions is a public entity under Title II, which is a legal conclusion the owner should confirm with OHSU counsel, not a fact this note can settle. The rule sets a state instrumentality's population by its state's population (unverified for OHSU specifically), which would put OHSU on the large-entity date, now April 26, 2027 [1][5].

### ADA Title III

Private colleges are public accommodations under Title III (28 CFR part 36). There is no Title III web rule: the Department withdrew its 2010 advance notice on Tuesday, December 26, 2017 [8]. The duty that binds private schools is effective communication through auxiliary aids and services in 28 CFR 36.303 [9]. Courts and the Department have applied it to websites and course content case by case; the DOJ and Department of Education joint Dear Colleague letter of Friday, May 19, 2023 reminds every postsecondary institution of this [10].

### Section 504

Section 504 of the Rehabilitation Act binds every recipient of federal funds. Two sets of regulations matter here.

The Department of Education's regulations at 34 CFR part 104 bind schools and colleges that take ED funds. 34 CFR 104.44(a) requires academic adjustments, and 104.44(d) requires auxiliary aids, including "taped texts" and other ways of making material available to students with sensory impairments [11]. These are the rules behind an accommodations office's work and behind OCR's resolution agreements (below). ED has not adopted a technical standard by regulation; OCR agreements name WCAG 2.1 AA as a standard it accepts [12].

The Department of Health and Human Services published its first comprehensive 504 rule on Thursday, May 9, 2024, at 45 CFR part 84, effective Monday, July 8, 2024 [13]. It binds recipients of HHS funds, which includes academic health centers and hospitals. Subpart I (84.82 to 84.89) requires web content and mobile apps to conform to WCAG 2.1 Level AA, with exceptions parallel to the Title II rule, and 84.83 bars discrimination through kiosks [14]. Subpart J (84.90 to 84.98) covers medical diagnostic equipment: equipment bought after July 8, 2024 must meet the Access Board's standards at 36 CFR part 1195, and within two years, by July 8, 2026, a recipient must have at least one accessible exam table and weight scale, or 10 percent of each type, whichever is greater [15]. The web and mobile compliance dates were May 11, 2026 for recipients with 15 or more employees and May 10, 2027 for smaller ones [13]. HHS published an interim final rule on Monday, May 11, 2026 moving them to May 11, 2027 and May 10, 2028 [16]. The kiosk and equipment dates did not move [16]. The NFB suit above challenges this extension too [6].

Separately, in Texas v. Kennedy (formerly Texas v. Becerra, N.D. Tex. 5:24-cv-00225), the court entered a final judgment on Wednesday, September 23, 2026 vacating the rule's community integration provisions nationwide on a joint motion of the states and the Department [17]. The web, kiosk, and equipment provisions were not vacated (unverified beyond the secondary reports in [17]).

### Section 508 and the Revised 508 Standards

Section 508 binds federal agencies and, through procurement, what they buy. The Access Board's Revised 508 Standards (36 CFR part 1194, final rule of Wednesday, January 18, 2017, 82 FR 5790) incorporate WCAG 2.0 Level AA by reference and apply it to web content, non-web documents, and software [18]. A public university is not bound by 508 directly, but its procurement office often asks for a 508-edition conformance report because federal grants and contracts flow through it. In 2025 the Board asked for information on a next refresh covering WCAG 2.2 and artificial intelligence (reported in [19]; the Federal Register document could not be retrieved, so its number and dates are unverified). The FY 2025 governmentwide 508 assessment was published in March 2026 [20]. No 2025 or 2026 action rescinded the standards.

### FERPA

FERPA's regulations at 34 CFR part 99 bind schools that take ED funds. An "education record" is a record directly related to a student and maintained by the institution or a party acting for it (99.3) [21]. A record of what a student read, how long, and how far, kept by the institution, is an education record once the institution maintains it. textweaver's reading statistics (`docs/reading.md`, "Reading statistics") and sync records are kept on the student's own computer, or in a folder the student chose, with no server (`docs/sync.md`). When an accommodations office runs textweaver on a lab computer, or its staff carry a student's sync folder, those files become records the office maintains, and the office's FERPA duties, including the "school official" conditions for disclosure without consent in 99.31(a)(1), attach to them [21]. The practical rule for textweaver: never write a student's name or the document's file name into a synced record, which `crates/textweaver-sync/src/record.rs` already follows per `docs/sync.md` ("Privacy"), and let an office turn statistics off per machine (`[stats] enabled`, `docs/settings-reference.md`).

### The Chafee amendment and the Marrakesh Treaty Implementation Act

17 U.S.C. 121 lets an "authorized entity" reproduce and distribute previously published literary works, and musical works fixed as text, in accessible formats "exclusively for use by eligible persons" [22]. The Marrakesh Treaty Implementation Act (Public Law 115-261, Tuesday, October 9, 2018) rewrote the definitions: an eligible person is someone who is blind, has a visual impairment or a perceptual or reading disability that cannot be improved to give substantially equivalent reading function, or cannot hold or manipulate a book or focus or move the eyes because of a physical disability; an authorized entity is a nonprofit organization or government agency whose primary mission includes services to such persons; and 17 U.S.C. 121A allows export and import of accessible format copies between Marrakesh countries [22][23]. Dyslexia qualifies when it is a perceptual or reading disability of that kind; ADHD, hearing loss, or an intellectual disability alone do not, which is how Bookshare reads the statute [24]. A university disability services office acting under its institution's mission is commonly treated as an authorized entity; that reading should be confirmed by counsel. The copies it makes are for the eligible student, not for the class, and a textweaver conversion that an office hands to a student is one of those copies.

### NIMAS

The National Instructional Materials Accessibility Standard binds K-12 only. 34 CFR 300.172 requires each state to adopt NIMAS and to work with the National Instructional Materials Access Center, and Appendix C to part 300 holds the file set specification (DAISY 3 DTBook XML, images, and metadata) [25]. NIMAC gathered stakeholder input in 2024 and sent OSEP a recommendations report in January 2025; no formal change to Appendix C had been published as of this note [26]. NIMAS matters to textweaver because its DAISY 3 loader (`crates/textweaver-formats`, per `docs/dev/architecture.md`) can read NIMAS-derived files that a student brings from school, and because Chafee eligibility language in 300.172 is the same as in section 121 [25].

### Oregon

ORS 659A.142 prohibits discrimination on the basis of disability by employers, places of public accommodation, and state and local government [27]. ORS chapter 276A gives the State Chief Information Officer authority over state agency information technology, and the Enterprise Information Services accessibility page says state websites should meet WCAG 2.1 AA; one secondary source says ORS 276A.305 names WCAG 2.0 AA (unverified, and OHSU is not a state agency under ORS 353.020 in any case) [7][28]. The Oregon Health Authority and ODHS adopted policies 010-029 and 010-030 in 2025 aligning their content and systems with WCAG 2.2 A and AA, which shows where Oregon public bodies are heading [29].

### Enforcement, 2020 to 2026

- OCR resolution agreements name the same findings again and again: an LMS that screen readers cannot use, images without alternative text, PDFs that are pictures of pages, wrong document language, and uncaptioned video. Examples with public letters: the University of North Texas at Denton (docket 06-20-2304) [30], Framingham State University (01-21-2153) [31], Purdue University Global (05-22-6001), which adopted WCAG 2.1 AA [32], and Oakland University (15-16-2210) [33]. The signing dates could not be read from the letters in this container and are unverified; the docket numbers encode the fiscal years 2020 to 2022.
- The joint Dear Colleague letter of May 19, 2023 cites resolution agreements with more than 50 institutions in one year [10].
- The Department of Justice's consent decree with the University of California, Berkeley (Monday, November 21, 2022) covers free online lectures, podcasts, and PDFs, requires WCAG 2.1 AA, and runs three and a half years with an independent auditor [34].
- Payan v. Los Angeles Community College District (9th Cir., Tuesday, August 24, 2021) upheld liability under Title II and 504 for inaccessible textbooks, handouts, math software, and a library system, and held that disparate impact claims are privately enforceable [35]. It is the case accommodations offices cite for "late is the same as never".

What this means for a converter: every finding above is about a document or a system that a student could not read. An office's defense is a record of what it converted, to what standard, and what it could not fix. Section 4 turns that into features.

## 2. Technical standards for textweaver's output and interface

| Standard | Edition and status | Can textweaver meet it today? | A conformance check |
|---|---|---|---|
| WCAG 2.2 | W3C Recommendation, Thursday, October 5, 2023, updated Thursday, December 12, 2024; ISO/IEC 40500:2025 [36] | GUI: partly. Keyboard-only use, character key shortcuts (2.1.4) and text spacing (1.4.12) are documented in `docs/keyboard.md` and `docs/reading-aids.md`; no full audit exists. HTML export: structure, language, alt text present (`docs/converting.md`) | An expert review of the GUI against the 2.2 AA criteria, and axe-core on HTML output |
| WCAG 3.0 | Working Draft, latest Thursday, September 10, 2026; Recommendation not before 2028 [37] | Not applicable yet | None required |
| EN 301 549 | v3.2.1 (2021) is the harmonized standard cited in the Official Journal; v4.1.1 published Wednesday, September 2, 2026 adopts WCAG 2.2 and adds an EAA annex, not yet cited [38][39] | Software clauses (chapter 11) map to WCAG; the GUI is untested against them. Document clauses (chapter 10) map to the export checks below | The EU edition of the ACR |
| European Accessibility Act | Directive (EU) 2019/882, applies from Saturday, June 28, 2025; covers e-books and e-reader software; microenterprise exemption for services [40] | textweaver is distributed free by one person; the directive binds economic operators placing products on the EU market, and whether a free, non-commercial download is "placed on the market" is unverified. Universities abroad that deploy it are service providers under their own national law | Publish the EU-edition ACR and an accessibility statement; that is what a European procurement office will ask for |
| EPUB 3.3 | W3C Recommendation, Thursday, May 25, 2023 [41] | Yes, for structure: OCF, package, nav, page list (`crates/textweaver-writers/src/epub.rs`; ADR-0017) | epubcheck 5.4.0 in `.github/workflows/second-tool.yml` |
| EPUB Accessibility 1.1 | W3C Recommendation, Thursday, May 25, 2023; 1.1.1 draft Wednesday, September 3, 2025 [42] | Discovery metadata: yes (`schema:accessMode`, `accessModeSufficient`, `accessibilityFeature`, `accessibilityHazard`, `accessibilitySummary` in `epub.rs`). Conformance claim (`dcterms:conformsTo`, `a11y:certifiedBy`): deliberately not written, because WCAG conformance needs a human evaluation (ADR-0017) | Ace by DAISY, which is not yet in CI [43] |
| schema.org accessibility metadata | W3C Group Note on schema.org metadata for EPUB [44] | Yes, the five properties above | Ace reports missing or inconsistent values |
| PDF/UA-1 | ISO 14289-1:2014 | Yes: krilla's validator at write time, veraPDF 1.30.2 in CI (`docs/dev/testing.md`) | veraPDF `-f ua1`; a PAC run by hand |
| PDF/UA-2 and Well-Tagged PDF | ISO 14289-2:2024, published March 2024, built on PDF 2.0 [45] | No. The writer targets PDF/UA-1; krilla's PDF/UA-2 support is unverified | veraPDF has a ua2 profile; Matterhorn Protocol 2.0 for PDF/UA-2 is in development at the PDF Association, unverified as published [46] |
| Matterhorn Protocol | 1.1, 31 checkpoints and 136 failure conditions, 89 machine-checkable [46] | Machine checks pass through veraPDF; the 47 human checks (reading order sense, alt text quality, heading meaning) have never been run on a textweaver PDF | A one-time human pass on three fixtures, recorded in `tools/second_tool_allowlist.txt` style |
| DAISY 3 | ANSI/NISO Z39.86-2005 (R2012) [47] | Read only (`crates/textweaver-formats`). No DAISY writer; the audio export writes M4B with chapters instead (`docs/audio-export.md`) | Not applicable |
| DAISY 2.02 | DAISY Consortium Recommendation, Wednesday, February 28, 2001 [48] | Neither read nor written. Many older library books are 2.02 | A loader for `ncc.html` plus SMIL is a medium task; see section 6 |
| UEB | The Rules of Unified English Braille, third edition, 2024, ICEB [49] | Grade 1 natively, grade 2 through liblouis (`crates/textweaver-writers/src/ueb.rs`, `brf.rs`; ADR-0017). Typeform and capitals passage indicators follow sections 8 and 9 | The liblouis round-trip job in `second-tool.yml` |
| Nemeth | Nemeth Code 2022, BANA, published February 2024 [50] | Through MathCAT in a build with it (`docs/math.md`) | MathCAT's own test suite; no textweaver test against the 2022 code book |
| Braille Formats | BANA, 2016 [51] | Headings, paragraphs, lists, and three table formats (11.16, 11.18, 11.2.5d) per ADR-0017 | Fixture review by a certified transcriber, never done |
| BRF | North American Braille ASCII, 40 by 25, CR LF, form feed [52] | Yes (`brf.rs`) | Open in Duxbury or emboss |
| MathML in EPUB and DOCX | DAISY Knowledge Base; `accessibilityFeature` values `MathML` and `describedMath` [53] | MathML in EPUB with the LaTeX source as fallback; Word equations in DOCX; print form plus spoken description in PDF (`docs/converting.md`, "Math") | Does `epub.rs` write `MathML` and `describedMath` in `accessibilityFeature`? Unverified; a one-line check |
| WAI-ARIA 1.3 | Working Draft, latest Thursday, June 4, 2026; adds `aria-braillelabel` and `aria-brailleroledescription` [54] | Only the HTML export uses ARIA. AccessKit exposes equivalent properties natively | Not needed for the GUI |
| UI Automation, NSAccessibility, AT-SPI2 | AccessKit adapters on Windows, macOS, Linux, Android, iOS; no IAccessible2 adapter [55] | Yes through AccessKit (`crates/textweaver-xilem`); JAWS and NVDA read UIA. The tree dumps in CI (ADR-0039) are the standing check | `uia-report.ps1`, `atspi-dump.py` in `crates/textweaver-xilem/tools/` |
| WebVTT and SRT | WebVTT W3C Candidate Recommendation, Thursday, April 4, 2019; SRT has no formal specification [56] | Both written by `crates/textweaver-export/src/cues.rs` | FCC caption quality rules at 47 CFR 79.1(j) (accurate, synchronous, complete, placed) and the DCMP Captioning Key are the content rules; textweaver's cues are exact to the word timing, so synchrony is met by construction [57][58] |

## 3. What textweaver itself should publish

An Accessibility Conformance Report. The ITI's VPAT 2.5Rev (April 2025) has four editions: 508, WCAG (2.0, 2.1, 2.2), EU (EN 301 549), and INT, which holds all three [59]. A university review asks for the 508 or INT edition; a European one for the EU edition. The INT edition covers both at once. Rules: the completed document is called an ACR, not a VPAT; each criterion gets Supports, Partially Supports, Does Not Support, or Not Applicable with remarks; and the report names the evaluation methods.

An accessibility statement. The W3C's statement generator produces the sections a statement needs: the standard aimed at, known limitations, the testing done, and how to report a problem [60]. Thorium Reader's policy is a good model for a volunteer project: it names the screen readers tested, admits it cannot afford a full test cycle each release, and points to a labeled issue tracker [61]. NVDA's privacy page is the model for data: update checks and an opt-in usage statistics switch, named in the settings dialog [62]. LibreOffice has no published ACR, which is a gap its users have asked about [63]. textweaver should do what Thorium does for testing and what NVDA does for data.

A security and privacy statement. `SECURITY.md` already says that nothing is sent without a user action and no telemetry is collected; `docs/sync.md` says what reaches the sync folder. These belong in one page, also linked from the ACR, with: local-only data; the three network actions (citation lookup, font download, Piper voice list and voice download) and when they run; what a sync folder holds (random ids, note text, highlight text, the words around a place, a document's title, author, DOI, and ISBN, never a file name or the computer's name); and that the folder is not encrypted.

A data retention note. Reading statistics are saved every 30 seconds and at close, in `stats.json`, with `tw stats --clear` to delete and `[stats] enabled = false` to stop (`docs/reading.md`). The note should say that textweaver never deletes them on its own, that statistics sync between a student's own computers when the group is on, and how an office wipes a lab computer (`--home` with a throwaway folder, as `docs/screen-readers.md` already suggests for testing).

### Outline of textweaver's ACR (INT edition)

Product: textweaver 0.1.0-alpha.N, the `textweaver-xilem` GUI, the `textweaver` terminal reader, and the documents `tw convert` writes. Evaluation methods: listening sessions with NVDA, JAWS, VoiceOver, Orca, and a Mantis Q40 (ADR-0028, ADR-0033), the CI tree dumps and Orca session (ADR-0039), epubcheck and veraPDF (`docs/dev/testing.md`). Each row below is where the answer is not yet known and must be checked before the report says Supports.

- 1.1.1 Non-text content: GUI toolbar buttons and icons have names (tree dumps say yes); exported images get the author's alt text or are reported.
- 1.3.1 Info and relationships: heading, list, and table semantics in every export (checked); the GUI document control is exposed as a group with its text as value on macOS (ADR-0039), which may fail here.
- 1.3.2 Meaningful sequence: PDF tagging order and the column-aware PDF loader (ADR-0010).
- 1.4.1 Use of color: 23 themes with contrast rules (`docs/themes.md`); state is never color alone, which needs a sweep of the status bar and RSVP panel.
- 1.4.3 Contrast and 1.4.11 Non-text contrast: not every theme meets AA, by design (`docs/themes.md`); the report must say which do.
- 1.4.12 Text spacing: supported (`docs/reading-aids.md`).
- 2.1.1 Keyboard and 2.1.2 No keyboard trap: the whole product is keyboard first; dialogs and the file chooser need a trap check.
- 2.1.4 Character key shortcuts: supported, with a switch (`docs/keyboard.md`).
- 2.2.2 Pause, stop, hide: RSVP and continuous reading pause on Space; needs a note on the live region.
- 2.3.1 Three flashes: RSVP measured under the limit (`docs/reading-aids.md`).
- 2.4.3 Focus order and 2.4.7 Focus visible: unverified in the GUI.
- 2.4.11 Focus not obscured and 2.5.7 Dragging movements, 2.5.8 Target size (new in 2.2): no drag gestures exist; target size of toolbar buttons is unverified.
- 3.1.1 and 3.1.2 Language: document language in every export; the GUI's six interface languages.
- 3.3.1 Error identification: converter errors are sentences read aloud (`docs/converting.md`, "When something fails").
- 4.1.2 Name, role, value: the AccessKit tree; the NVDA session's open failure, where the window's name is not spoken on focus (ADR-0039), is a known Partially Supports.
- 4.1.3 Status messages: the live region on Windows and the UIA notification path (`docs/gui.md`).
- 508 chapter 5 (software) 502.3 platform accessibility services: AccessKit through UIA, NSAccessibility, AT-SPI2.
- 508 chapter 6 (documentation): docs are Markdown and HTML; an accessible PDF manual would come from `tw convert` itself.
- EN 301 549 clause 11.8 (authoring tools): an authoring tool must produce accessible output and preserve accessibility information in conversions; this is textweaver's strongest claim, and the writers' tests are the evidence.

## 4. What accommodations offices need to document compliance

An office answers an OCR complaint with records. textweaver today writes `conversion-report.txt` (summary, failures, warnings; `crates/textweaver-convert/src/lib.rs`, `docs/converting.md`) and a hot-folder log with UTC timestamps (`crates/textweaver-convert/src/watch.rs`). That is a start. The following features turn it into evidence.

1. Conversion log with provenance (medium, alpha.8). For each output, record the source path, its SHA-256, the output path and its SHA-256, the writer and textweaver version (`tw --version` string), the options used (template, font, grade, math code), the UTC time, and the account name of who ran it, written as one JSON line per file beside the text report (`conversion-report.jsonl`). Hashing a 10 MB source costs about 10 ms on one core and runs on the worker that already read the file, never on the input thread. The account name is personal data under FERPA only if a student runs it; make it `--no-operator` to leave out.
2. "What could not be made accessible" report (medium, alpha.8). A per-file list, in the same JSON and in a human sentence, of: images with no description (already warned, ADR-0017), tables with no header row, math that stayed as LaTeX source, characters with no braille cell, fonts that could not embed, scanned pages recognized by OCR (the loader already marks them; `docs/converting.md`, "Scanned pages"), and links turned into text. Each item carries the page or heading it is under, so a human remediator can go there. This is the document an office attaches to a student's file when it says "we did what could be done, and here is what the publisher must fix".
3. Batch summary for a term (small, alpha.9). `tw convert --summary` over the JSON lines: counts by format, by failure reason, by warning kind, and the list of files with unresolved items. Reads the JSON only; no new runtime cost.
4. Alternate format request workflow (large, later). An office asks the publisher through the AccessText Network, where member disability services offices request files and publishers answer [64]; or Bookshare, which checks Chafee eligibility and lets a member download [24]; or Learning Ally, which checks eligibility against the same kinds of proof [65]; or the publisher directly. A `tw request` command would write a request letter from the document's metadata (title, author, ISBN, DOI, and edition, which `textweaver-cite` already looks up) and record where the request went and when. textweaver must never carry the eligibility proof itself; it should record only "eligibility on file, verified by [office]" as a yes or no, since the proof is a medical record. The Chafee limit belongs in the generated letter: the copy is for the named eligible student and may not be shared.
5. Eligibility and distribution notice in exports (small, alpha.9). An option to write a short notice into the EPUB's `dc:rights`, the DOCX core properties, and the PDF's XMP: "Accessible copy made under 17 U.S.C. 121 for an eligible person. Not for further distribution." Costs a few bytes.
6. Signed reports (small, later). A detached signature over `conversion-report.jsonl` with the office's key, so a report cannot be edited after the fact. `minisign` style, one dependency; keep it behind a feature so the lean reader stays at 28.7 MB.

## 5. Licensing and provenance of textweaver's dependencies

- GPL-3.0-or-later and proprietary engines. Eloquence, SAPI, and DECtalk run in separate host processes that talk to textweaver over pipes with a framed protocol (`SECURITY.md`, `docs/eloquence.md`, `docs/dectalk.md`). The GPL FAQ says that pipes, sockets, and command line arguments are the mechanisms of separate programs, and that two programs communicating that way are normally not one combined work, unless the semantics are intimate enough to exchange internal data structures [66]. textweaver's protocol carries text, rate, and word timing events, not internal structures. That is the strongest available position, and it is still a legal question a court would decide. textweaver never ships, downloads, or builds Eloquence, OpenEVV, or DECtalk (`docs/eloquence.md`, `docs/dectalk.md`), so no distribution question arises for those engines.
- Piper voices. Each voice carries its own license in its MODEL_CARD in the rhasspy/piper-voices repository; `lessac`, `ryan`, and `hfc_female` are non-commercial, `joe` is CC0, and `libritts_r` is CC BY [67], and textweaver reads the license aloud before every download and ships no voice (`docs/speech.md`, "Piper voices"). A university that installs a non-commercial voice on staff computers for paid work should choose a CC0 or CC BY voice; the voice manager's "non-commercial" tag is the control. The ACR's privacy section should list the voice list fetch as a network action.
- Fonts. Atkinson Hyperlegible Next and Mono and OpenDyslexic are bundled under the SIL Open Font License 1.1, with their license files in `third_party/fonts/` (`docs/dev/third-party-data.md`). The OFL allows bundling, embedding, and redistribution with software, requires the license text to travel with the font, and reserves the font name for modified versions [68]. Lexend is downloaded on request under the same license. EPUB embedding is limited to bundled fonts because installed fonts' licenses may forbid it (ADR-0017), which is the right rule.
- Lexicon. Open English WordNet 2025 is CC BY 4.0 with Princeton's notice, and CMUdict is a two-clause BSD license; both notices are in `third_party/lexicon/` and `licenses/lexicon/` in each package (`third_party/lexicon/README.md`). SCOWL's MIT-like notices are in `third_party/scowl/Copyright`. The IBMTTS dictionaries are CC0. The ocrs models are CC BY-SA 4.0, downloaded on request (`docs/converting.md`).
- MathCAT is vendored in `third_party/mathcat/` with its license; its rules files are data the GPL combination includes.
- `cargo xtask notices` writes `THIRD-PARTY-NOTICES.md` from every crate's license (`docs/dev/third-party-data.md`). That file answers the "list all third-party components and licenses" question.

What a university software review asks. HECVAT 4 (released Monday, February 10, 2025; 4.1.6 current, unverified) is the EDUCAUSE questionnaire with sections on the organization, product, infrastructure, IT accessibility (19 questions aligned to WCAG 2.1 AA), case-specific items, artificial intelligence, and privacy [69]. Many of its questions assume a vendor with a hosted service. For a local-only, open source tool the honest answers are: no hosted service, no data leaves the computer except the user-initiated actions listed above, no account, no AI model that sends data (Whisper and ocrs run in process), releases signed with build provenance attestations and SHA-256 sums (`SECURITY.md`), one maintainer, and security fixes for the newest release only. The review will also ask for the ACR, a list of third-party components (`THIRD-PARTY-NOTICES.md`), a software bill of materials (`cargo sbom` or `cargo cyclonedx` can produce one, which is a small task), and the vulnerability reporting path (`SECURITY.md`).

## 6. Twelve compliance improvements, ranked

Sizes: small is hours, medium a day or two, large a week or more. Every GUI item must work with a screen reader, a braille display, and the keyboard alone, and must not show state by color alone.

1. Write the accessibility conformance report, INT edition (large, alpha.8). Law: 28 CFR 35.200 and 45 CFR 84.84 drive the procurement questions; standard: VPAT 2.5Rev [59]. Touches `docs/` (a new `docs/accessibility-conformance.md`) and the ADR-0039 evidence. Measure: every WCAG 2.2 AA criterion has a status and a remark. No runtime cost.
2. Conversion log with provenance (medium, alpha.8). Law: 34 CFR 104.44 and OCR agreements' documentation duties [11][30]. Touches `crates/textweaver-convert/src/lib.rs` and `crates/textweaver-cli/src/cmd/convert.rs`. Measure: `cargo xtask bench` convert throughput unchanged within noise; hashing is about 1 percent of a 10 MB load. Keep hashing on the worker thread.
3. "Could not be made accessible" report (medium, alpha.8). Standard: EPUB Accessibility 1.1 and PDF/UA-1 both need alt text and header cells [42][46]. Touches the `WriteReport` in `crates/textweaver-writers/src/lib.rs` and the four writers. Measure: a fixture with an undescribed image, a headerless table, and an unparsed formula yields three items with locations.
4. Ace by DAISY in the second-tool workflow (small, alpha.8). Standard: EPUB Accessibility 1.1 [42][43]. Touches `.github/workflows/second-tool.yml` and `tools/second_tool_allowlist.txt`. Measure: every fixture EPUB passes Ace with no serious violation. CI only.
5. Accessibility, privacy, and retention statements (small, alpha.8). Law: FERPA 34 CFR 99.3 for the retention note [21]; standard: W3C statement guidance [60]. Touches `docs/` and `SECURITY.md` links. No runtime cost.
6. Fix the NVDA focus announcement so the window and document names are spoken on focus (small, alpha.8). Standard: WCAG 4.1.2 and the UIA name property [36][55]. Touches `crates/textweaver-xilem`. Measure: the NVDA session in ADR-0039 passes three runs in a row and becomes standing.
7. Write `MathML` and `describedMath` into `accessibilityFeature` when a book has math (small, alpha.9). Standard: EPUB Accessibility 1.1 and the DAISY Knowledge Base [42][53]. Touches `crates/textweaver-writers/src/epub.rs`. Measure: Ace reports the feature.
8. Human Matterhorn pass on three PDFs, recorded (small, alpha.9). Standard: PDF/UA-1 and Matterhorn 1.1's 47 human checks [46]. Touches `docs/dev/testing.md`. No code.
9. Rights notice in exports (small, alpha.9). Law: 17 U.S.C. 121 [22]. Touches `WriteOptions` in `crates/textweaver-writers/src/lib.rs` and the three container writers. Measure: the notice reads back from each format.
10. Software bill of materials in the release (small, alpha.9). Procurement: HECVAT 4 [69]. Touches `xtask/src/dist.rs`. Measure: a CycloneDX file in `SHA256SUMS.txt`'s list.
11. PDF/UA-2 output as an option (large, later). Standard: ISO 14289-2:2024 [45]. Touches `crates/textweaver-writers/src/pdf/` and depends on krilla's support, which is unverified. Measure: veraPDF `-f ua2` passes on the fixtures. Keep PDF/UA-1 the default, since most viewers and checkers still target it.
12. DAISY 2.02 loader (medium, later). Standard: DAISY 2.02 [48]. Touches `crates/textweaver-formats`. Measure: a 2.02 sample opens with its headings and page numbers; the `tw info` time on it stays under the 10 MB Markdown figure in `docs/dev/testing.md`. Alternate format request workflow (large, later) follows it.

## Sources

1. https://www.federalregister.gov/documents/2024/04/24/2024-07758/nondiscrimination-on-the-basis-of-disability-accessibility-of-web-information-and-services-of-state-and-local-government-entities
2. https://www.ecfr.gov/current/title-28/chapter-I/part-35/subpart-H
3. https://www.ecfr.gov/current/title-28/chapter-I/part-35/subpart-A/section-35.104
4. https://www.ecfr.gov/current/title-28/chapter-I/part-35/subpart-H/section-35.201
5. https://www.federalregister.gov/documents/2026/04/20/2026-07663/extension-of-compliance-dates-for-nondiscrimination-on-the-basis-of-disability-accessibility-of-web
6. https://www.adatitleiii.com/2026/06/national-federation-of-the-blind-challenges-last-minute-deadline-extensions-for-website-and-mobile-app-accessibility/ (complaint: https://browngold.com/wp-content/uploads/2026/05/NFB-v.-DOJ_5-21-2026.pdf)
7. https://www.oregonlegislature.gov/bills_laws/ors/ors353.html
8. https://www.federalregister.gov/documents/2017/12/26/2017-27510/nondiscrimination-on-the-basis-of-disability-notice-of-withdrawal-of-four-previously-announced-rulemaking-actions
9. https://www.ecfr.gov/current/title-28/chapter-I/part-36/subpart-C/section-36.303
10. https://www.justice.gov/crt/case/dear-colleague-letter-online-accessibility-postsecondary-institutions
11. https://www.ecfr.gov/current/title-34/subtitle-B/chapter-I/part-104/subpart-E/section-104.44
12. https://www.ed.gov/laws-and-policy/civil-rights-laws/disability-discrimination/disability-discrimination-key-issues/disability-discrimination-technology-accessibility
13. https://www.federalregister.gov/documents/2024/05/09/2024-09237/nondiscrimination-on-the-basis-of-disability-in-programs-or-activities-receiving-federal-financial-assistance
14. https://www.ecfr.gov/current/title-45/subtitle-A/subchapter-A/part-84/subpart-I
15. https://www.ecfr.gov/current/title-45/subtitle-A/subchapter-A/part-84/subpart-J
16. https://www.federalregister.gov/documents/2026/05/11/2026-09266/extension-of-compliance-dates-for-nondiscrimination-on-the-basis-of-disability-accessibility-of-web
17. https://clearinghouse.net/case/45899/ and https://www.acb.org/update-texas-v-becerra-case
18. https://www.access-board.gov/ict/
19. https://www.disabilityworld.org/articles/section-508-refresh-status/
20. https://www.section508.gov/manage/section-508-assessment/2025/executive-summary/
21. https://www.ecfr.gov/current/title-34/subtitle-A/part-99
22. https://uscode.house.gov/view.xhtml?req=granuleid:USC-prelim-title17-section121&num=0&edition=prelim
23. https://www.govinfo.gov/content/pkg/PLAW-115publ261/html/PLAW-115publ261.htm
24. https://www.bookshare.org/legal/copyright-information/
25. https://www.ecfr.gov/current/title-34/subtitle-B/chapter-III/part-300/subpart-B/section-300.172 and https://www.ecfr.gov/current/title-34/subtitle-B/chapter-III/part-300/appendix-Appendix%20C%20to%20Part%20300
26. https://nimac.us/nimas-specification-report-now-available/
27. https://www.oregonlegislature.gov/bills_laws/ors/ors659A.html
28. https://www.oregon.gov/eis/Pages/accessibility.aspx
29. https://www.oregon.gov/oha/Digital-Accessibility/Pages/Federal-Rules-on-Digital-Accessibility.aspx
30. https://www.ed.gov/sites/ed/files/about/offices/list/ocr/docs/investigations/more/06202304-b.pdf
31. https://www.ed.gov/sites/ed/files/about/offices/list/ocr/docs/investigations/more/01212153-b.pdf
32. https://ocrcas.ed.gov/sites/default/files/ocr-letters-and-agreements/05226001-b.pdf
33. https://www.ed.gov/media/document/15162210-bpdf-32824.pdf
34. https://www.justice.gov/archives/opa/pr/justice-department-secures-agreement-university-california-berkeley-make-online-content
35. https://cdn.ca9.uscourts.gov/datastore/opinions/2021/08/24/19-56111.pdf
36. https://www.w3.org/TR/WCAG22/
37. https://www.w3.org/TR/wcag-3.0/
38. https://www.etsi.org/deliver/etsi_en/301500_301599/301549/
39. https://accessible-eu-centre.ec.europa.eu/content-corner/news/european-accessibility-standard-en-301-549-has-been-updated-2026-09-07_en
40. https://eur-lex.europa.eu/eli/dir/2019/882/oj
41. https://www.w3.org/TR/epub-33/
42. https://www.w3.org/TR/epub-a11y-11/
43. https://daisy.org/activities/software/ace/
44. https://www.w3.org/TR/epub-a11y-11/#sec-disc-package (schema.org metadata; the separate Group Note is https://www.w3.org/publishing/a11y/schema-org-metadata/)
45. https://www.iso.org/standard/82278.html
46. https://pdfa.org/resource/the-matterhorn-protocol/
47. https://daisy.org/activities/standards/daisy/daisy-3/
48. https://daisy.org/activities/standards/daisy/daisy-2/daisy-format-2-02-specification/
49. https://iceb.org/Rules%20of%20Unified%20English%20Braille%202024.pdf
50. https://brailleauthority.org/sites/default/files/2024-02/Nemeth_2022.pdf
51. https://www.brailleauthority.org/node/41
52. https://duxburysystems.com/documentation/dbt12.7/Content/miscellaneous/brf_files.htm
53. https://kb.daisy.org/publishing/docs/html/mathml.html and https://kb.daisy.org/publishing/docs/metadata/schema.org/accessibilityFeature/MathML.html
54. https://www.w3.org/TR/wai-aria-1.3/
55. https://github.com/AccessKit/accesskit
56. https://www.w3.org/TR/webvtt1/
57. https://www.ecfr.gov/current/title-47/chapter-I/subchapter-C/part-79/subpart-A/section-79.1
58. https://dcmp.org/learn/captioningkey
59. https://www.itic.org/policy/accessibility/vpat
60. https://www.w3.org/WAI/planning/statements/
61. https://thorium.edrlab.org/en/th3/900_about_thorium/903_thorium-accessibility-policy
62. https://www.nvaccess.org/privacy/
63. https://listarchives.libreoffice.org/global/accessibility/msg01280.html
64. https://www.accesstext.org/
65. https://learningally.org/About-Us/What-We-Do/Who-Qualifies
66. https://www.gnu.org/licenses/gpl-faq.html#MereAggregation
67. https://huggingface.co/rhasspy/piper-voices
68. https://openfontlicense.org/open-font-license-official-text/
69. https://www.educause.edu/higher-education-community-vendor-assessment-toolkit and https://er.educause.edu/articles/2025/4/accessibility-in-technology-acquisition-with-hecvat-4

## See also

- [Research index](README.md)
- [Converting documents](../../converting.md): the exports and the conversion report this note builds on.
- [Using textweaver with a screen reader](../../screen-readers.md): the modes and the braille layout the ACR will describe.
- [ADR-0017: Native writers](../../adr/0017-writers.md): what each writer does for accessibility today.
