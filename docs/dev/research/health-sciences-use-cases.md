# Health sciences education: materials, pronunciation, and study workflows

This report says what students and faculty in medicine, nursing, dentistry, pharmacy, physician assistant programs, public health, and biomedical research need from a reading and writing tool, and what textweaver should build for them. It is for the maintainers planning alpha.8 and alpha.9, and for the accommodations office that will deploy it. Written Friday, October 2, 2026, against the repository at 0.1.0-alpha.7. Claims about textweaver's own behavior marked "measured" were checked that day by running the normalization pipeline from `crates/textweaver-speech/src/normalize/mod.rs` on clinical sentences through a scratch program kept outside the repository.

## Who the users are

According to PubMed, Meeks and colleagues surveyed US medical schools in 2016 and 2019 and found more students disclosing a disability, with ADHD, learning disabilities, and psychological disabilities the largest groups ([DOI](https://doi.org/10.1001/jama.2019.15372), source 17). In a national AAMC questionnaire, 10.2 percent of responding second-year students reported a disability and half of them used accommodations ([DOI](https://doi.org/10.1111/medu.14995), source 18). A reading tool serves alternate formats and text-to-speech. Faculty and the office are the second user group: they convert course packs, check documents before posting, and keep records.

## 1. The materials

The table lists each kind of material, what breaks when it is read aloud, and what textweaver does today, naming the file.

| Material | What breaks in speech and screen readers | textweaver today |
|---|---|---|
| Textbooks on platforms: VitalSource Bookshelf, RedShelf, McGraw Hill AccessMedicine, Elsevier ClinicalKey Student, Wolters Kluwer thePoint, UpToDate | Reading stays inside the platform. Bookshelf and RedShelf have read aloud with highlighting and screen reader support (sources 1 to 4); AccessMedicine and ClinicalKey are web chapters with VPATs (sources 5 to 7). None lets the student use their own voice, lexicon, rate presets, or notes, and none reads formulas as structure. | Reads what the student can download: EPUB, PDF, HTML, DOCX (`crates/textweaver-formats/src/`). Platform-locked titles need the publisher file from AccessText or Bookshare (section 6). |
| Journal PDFs | Two columns read across, running headers inside sentences, captions mixed into text, tables flattened, reference superscripts read as numbers in the sentence, supplements as spreadsheets. | Columns, running heads, tables, and captions are handled (`pdf/layout.rs`, `pdf/structure.rs`; [ADR-0010](../../adr/0010-pdf-loader.md), [ADR-0048](../../adr/0048-pdf-annotations-links-and-forms.md)). Superscripts are joined back into the line (`layout.rs`) and read as plain digits. Footnote markers come only from tagged notes (`structure.rs`, `MarkerKind::Footnote`). |
| Lab protocols | Volumes and concentrations ("2.5 mL of 0.1 M NaOH"), units, µL, formulas, warnings in colored boxes. | Decimals, percents, and degrees are read (`normalize/numbers.rs`, `punctuation.rs`). Measured: "25 µg", "H2O", and "HCO3−" pass through as written and the engine decides. |
| Drug monographs: Lexicomp, package inserts, StatPearls | Dosage lines, look-alike names, Tall Man capitals, adverse-effect tables, weight-based doses. | Measured: "Give 5 mg q6h PRN for pain." is spoken as written; "mg/kg" is "mg slash kg". |
| Clinical guidelines | Recommendation grades, long tables, algorithms as images, confidence intervals and p-values. | Tables narrated with positions (`normalize/markdown.rs`, `table_mode`). Measured: "95% CI 1.2–3.4" keeps the en dash for the engine. |
| Exam preparation: UWorld, board banks, NCLEX practice | UWorld offers accommodations by request (source 8); a third-party page says the bank has no read-aloud (source 9, unverified). NCLEX allows JAWS as a documented accommodation (source 45). | Questions are pasted into a document and read ([editing guide](../../editing.md)). No way to keep question, answer, and explanation together as a note. |
| Case files | Vitals and labs with reference ranges, 24-hour times, record abbreviations. | Measured: "at 08:05" becomes "eight oh five AM"; "BP 120/80 mmHg" becomes "BP 120 slash 80 mmHg". |
| EHR training material | Screenshots carry the content; text layers are missing. | OCR of pages with no text layer ([ADR-0026](../../adr/0026-ocr-and-student-formats.md)); no table structure from OCR. |
| Anatomy atlases | Labeled figures; the text is in the picture. Tactile and 3D models are the known workaround (source 46). | Only alternative text is read; OCR of a plate gives labels with no relation to structures. |
| Chemical structures | Skeletal formulas are images; SMILES, CML, and mhchem rarely reach the student. | Measured: `$\ce{H2O}$` is "ce H 2 O", because `\ce` is unknown to `crates/textweaver-math` and is spoken by name ([math.md](../../math.md)). |
| Lecture slides | Text boxes out of order, hidden speaker notes, images without alt text. | PPTX slides in order, placeholders as lists, speaker notes under a heading ([ADR-0026](../../adr/0026-ocr-and-student-formats.md)). |

What a reader must do for all of these: keep reading order, say tables as tables, name symbols the engine would skip, expand only the abbreviations that are safe to expand, and let the student fix a pronunciation once and keep it.

## 2. Pronunciation

### How engines go wrong

Rule-based engines pronounce an unknown word by letter-to-sound rules. espeak-ng's English word list `en_list` has 5,794 lines and its rule file 7,132 (source 27). On Friday, October 2, 2026, `en_list` had no entry for acetaminophen, metformin, warfarin, lisinopril, atorvastatin, dyspnea, or ischemia, so each is read by rule. Piper voices take their phonemes from espeak-ng (`crates/textweaver-piper/src/phonemes.rs`; the Piper project embeds espeak-ng, source 28), so a Piper voice inherits every espeak-ng mistake, however natural it sounds. The failures that matter:

- stress on the wrong syllable in drug names and anatomical compounds;
- "-emia", "-itis", "-ectomy" endings with the wrong vowel;
- French and German eponyms (Guillain-Barré, Sjögren, Raynaud, Wernicke) read as English;
- look-alike pairs read alike (hydroxyzine, hydralazine), a safety problem;
- Tall Man capitals split. Measured with split caps on: "hydrOXYzine" becomes "hydr OX Yzine" and "DOPamine" becomes "DO Pamine" (`normalize/punctuation.rs`, `SplitCaps`).

### Term sources and their licenses

| Source | Gives | License | Size | Fit |
|---|---|---|---|---|
| CMUdict, shipped in `third_party/lexicon/lexicon-en.twlex` ([ADR-0025](../../adr/0025-lexicon-and-message-catalog.md)) | ARPAbet | BSD-style (source 22) | 126,052 words (`third_party/lexicon/README.md`) | Measured coverage of 240 common clinical terms: 107 found, 133 missing (45 percent). Missing: metoprolol, atorvastatin, metformin, warfarin, furosemide, creatinine, and most "-itis", "-emia", and "-uria" words. |
| Telnyx medical pronunciation dictionary | respelling and IPA per term | data CC BY 4.0, code MIT (source 23) | 966 entries: 398 drugs, 267 clinical, 152 anatomical, 149 acronyms | Direct fit: the respelling column is what `[normalization] pronunciations` takes today. |
| ipa-dict | IPA for general English | MIT (source 24) | large | Backstop for Piper overrides; no medical focus. |
| wordlist-medicalterms-en | words, no pronunciations | GPL v3 (source 25) | 98,119 terms | Says which words need entries; license-compatible. |
| RxNorm | drug names | SAB=RXNORM content is public domain; the rest needs the UMLS license (source 21) | thousands | Drug term list; no pronunciations. |
| UMLS and SNOMED CT | clinical terms | free UMLS license; SNOMED affiliate license with redistribution limits (sources 19, 20) | millions | Too large and restricted to ship; useful offline to pick top terms. |
| FDA and ISMP Tall Man lists | look-alike pairs with capitals | public (sources 13, 14) | FDA short, ISMP longer | Whole-word entries so split caps never cuts them. |

Tall Man lettering is made for labels. A nine-year series in 42 children's hospitals found no change in look-alike error rates after it was introduced ([DOI](https://doi.org/10.1136/bmjqs-2015-004562), source 15), and a study of 7,987 nonproprietary names showed how shared stems make names similar by design ([DOI](https://doi.org/10.1371/journal.pone.0145431), source 16). For speech the lesson is to make the two names sound different by stress and syllable, not by capitals.

### Lexicon formats the engines accept

- CMUdict ARPAbet: `W AO1 R F ER0 IH0 N`, respelled for reading aloud as `WAR-fuh-rin` (`crates/textweaver-lexicon/src/pronounce.rs`).
- espeak-ng: its own mnemonics in `en_list`, plus an `en_extra` file for user words, compiled into `en_dict` (source 26). Whether the bundled Rust port compiles `en_extra` is unverified.
- Piper: IPA, one id per character from the voice's `phoneme_id_map` (`crates/textweaver-piper/src/config.rs`); textweaver builds the id list itself in `phonemes.rs::prepare`.
- SAPI 5: `ISpLexicon::AddPronunciation` adds a word in the SAPI phone set to a user lexicon shared by every application (source 29).
- Eloquence: `main`, `abbr`, and `root` dictionaries, one `word<TAB>respelling` per line (`normalize/community.rs`).
- NVDA: default, voice, and temporary dictionaries; an entry is a pattern, a text replacement, case sensitivity, and a type of anywhere, whole word, or regular expression (source 30).
- JAWS: Dictionary Manager writes `.jdf` files, one default and one per application (source 31).

A student on NVDA or JAWS already keeps a dictionary; textweaver should import it.

### Proposal: a medical lexicon

Data. Ship `lexicon/medical-en.toml`, a `term = "respelling"` file in the format `[normalization.pronunciations]` already reads ([speech guide](../../speech.md)), with an optional IPA column for Piper. Seed it with the Telnyx data (966 entries, credited in `THIRD-PARTY-NOTICES.md`), then grow it to about 2,500 entries from RxNorm ingredient names and the top terms of the medical word list, with respellings checked by ear. Record sources and checksums in `third_party/`, as `third_party/lexicon/README.md` does. About 120 KB on disk; no binary growth.

Pipeline. The `Pronunciations` transform (`normalize/abbreviations.rs`) builds one regular expression alternation of every term, rebuilt whenever settings change. That is right for a few dozen user entries and wrong for 2,500. Add a `MedicalLexicon` transform modeled on `CommunityLexicon` (`community.rs`): a hash map lookup per word, lower-casing a capitalized word and splitting a hyphenated one. Order: user entries, then medical, then community, so the student always wins. Switch: `[normalization.medical_lexicon] enabled`, on in a health sciences profile. For Piper, a hook in `phonemes.rs::prepare` cuts out words with an IPA entry before the clause is phonemized and splices the IPA in with its word owner, so the highlight stays exact.

Run-time cost. One hash lookup per word on the speech thread, where normalization already runs before the two-sentence lookahead ([architecture](../architecture.md)); nothing on the input thread. Budget: no measurable change in `cargo xtask bench --only first_speech` on 1 MB and under 5 percent on the 10 MB narration plan. Loading 2,500 lines takes about a millisecond.

Adding terms. For a student: a palette command `add pronunciation` on the word at the cursor, which asks for the respelling, says it back, and writes `[normalization.pronunciations]`, which already syncs ([sync guide](../../sync.md)). For an office: `[normalization] lexicon_files = ["\\\\share\\osa\\pronunciations.toml"]`, read at startup and when changed, so one coordinator maintains it for every student. `tw lexicon import nvda.dic` and `tw lexicon import jaws.jdf` take whole-word text entries and report the rest. `tw speak --backend null --json` shows what will be said ([math.md](../../math.md)).

### Twenty test terms

Each row is a test for the lexicon. The expected column is the respelling the lexicon carries; what each engine says without it is to be checked by ear and is not claimed here.

| Term | Why it is hard | Expected respelling | In CMUdict |
|---|---|---|---|
| acetaminophen | stress on the third syllable | uh-SEE-tuh-MIN-uh-fen | yes |
| atorvastatin | five syllables, two stresses | uh-TOR-vuh-STAT-in | no |
| metoprolol | stress on "TOP" | meh-TOE-pruh-lol | no |
| warfarin | "WAR" | WAR-fuh-rin | no |
| lisinopril | "SIN" | lye-SIN-oh-pril | no |
| hydroxyzine vs hydralazine | look-alike pair | hye-DROK-sih-zeen; hye-DRAL-uh-zeen | no, no |
| ceftriaxone | "AKS" | sef-try-AKS-own | no |
| phenytoin | "FEN-ih-toyn" | FEN-ih-toyn | no |
| levetiracetam | often mangled | lee-veh-tye-RASS-eh-tam | no |
| furosemide | "fyoo-ROH" | fyoo-ROH-seh-mide | no |
| dyspnea | silent p | DISP-nee-uh | yes |
| ischemia | "is-KEE" | is-KEE-mee-uh | yes |
| cholecystitis | "koh-lee-sis-TYE" | koh-lee-sis-TYE-tis | no |
| creatinine | "kree-AT" | kree-AT-ih-neen | no |
| sphygmomanometer | long compound | sfig-moh-muh-NOM-eh-ter | no |
| Guillain-Barré | French | ghee-YAN bah-RAY | no |
| Sjögren | Swedish | SHOW-grin | not checked |
| Raynaud | silent d | ray-NOH | no |
| ileum vs ilium | sound-alike, different organs | ILL-ee-um; context decides | no, yes |
| Wernicke | German | VER-nih-kuh | yes |

## 3. Numbers, units, and safety

Measured outputs of the default pipeline (punctuation "some", split caps off, engine without native normalization), and what a health sciences reader needs.

| Input | textweaver today (measured) | Needed |
|---|---|---|
| `Give 5 mg q6h PRN for pain.` | as written | "5 milligrams every 6 hours as needed". With lexicon entries for q6h and PRN, measured: "Give 5 mg every 6 hours as needed". |
| `0.5 mg, not 5.0 mg` | "zero point five mg, not five point zero mg" | Correct. Keep the leading zero; never drop a trailing zero silently. |
| `taper over 6–8 weeks` | "6–8" passed to the engine | "6 to 8 weeks". An en dash between numbers is a range. |
| `95% CI 1.2–3.4; p < 0.05; p = .03` | "ninety-five percent CI one point two–three point four; p less than zero point zero five; p equals point zero three" | Correct except the range. |
| `K+ 3.5 mEq/L; Ca2+; Na+` | "K plus three point five mEq slash L; Ca2 plus; Na plus" | "calcium two plus", "sodium plus" with a chemistry reader on; "per liter" for "/L". |
| `25 mcg vs 25 µg` | both passed through | "25 micrograms" for both. ISMP says µg is misread as mg (source 11); say the unit in full. |
| `CPT 99213; PMID 31769816; ZIP 97239` | "ninety-nine thousand two hundred thirteen; thirty-one million seven hundred sixty-nine thousand eight hundred sixteen; ninety-seven thousand two hundred thirty-nine" | Digits. Rule 11 of `normalize/numbers.rs` reads every integer of four or more digits as a number. |
| `ICD-10 E11.9; NCT04368728` | as written | Correct; "E eleven point nine" is how clinicians say it. |
| `at 08:05` | "eight oh five AM" | "eight oh five". A 24-hour time without AM or PM must not gain one (`numbers.rs`, rule 3). |
| `TNF-α; 10 U insulin; QD and QOD` | passed through | "TNF alpha"; "10 units insulin"; "Q D" and "Q O D" spelled, never expanded. |
| `38.5°C; SpO2 98%; 120/80 mmHg` | "thirty-eight point five degrees C; SpO2 ninety-eight percent; 120 slash 80 mmHg" | "degrees Celsius"; "S p O 2"; "120 over 80 millimeters of mercury". |
| `WBC 11.5 × 10^9/L` | "eleven point five times 10 caret 9 slash L" | "times ten to the ninth per liter". |
| `BRCA1, TP53; 99mTc; 131I` | passed through | Gene symbols letter by letter with digits; isotopes "technetium 99 m", "iodine 131" from a small table. |

A European decimal comma, "2,5%", is read as "2,five percent" and should follow the document language.

### What an abbreviation expander must never expand

The ISMP List of Error-Prone Abbreviations (source 11, updated 2024, source 12) and The Joint Commission's Do Not Use list, summarized by Wick ([DOI](https://doi.org/10.4140/tcp.n.2007.870), source 47), name abbreviations that caused harm because they are misread. A reader has the same problem in sound. Rule: an abbreviation on those lists is spelled letter by letter, never guessed, and at high verbosity the reader adds "error-prone abbreviation". The deny list, checked in `normalize/abbreviations.rs` before any expansion:

- U and IU: spell; a lexicon may say "units" only after a number and a known drug.
- QD, QOD, q.d., q.o.d.: spell, never "every day" or "four times".
- MS, MSO4, MgSO4: spell; morphine and magnesium must not be guessed.
- cc: spell; mL is the unit to expand. µg: say "micrograms", because the symbol is the hazard.
- SC, SQ, HS, TIW, AD, AS, AU, OD, OS, OU: spell.
- "@", "<", ">": name the symbol, which `punctuation.rs` already does.
- Trailing zero and missing leading zero: read the digits as written; measured, both are right today.
- Drug-name abbreviations such as TPA and HCTZ: spell, never expand to a drug.

Safe expansions: PRN, PO, BID, TID, QID, q4h, q6h, q8h, NPO, stat.

### Tests to add

Add `clinical_vectors` to `crates/textweaver-speech/src/normalize/tests.rs`, in the style of `normalize_numbers_vectors`: the table above, the deny list (each entry unchanged or spelled), the range rule, the identifier rule (CPT, PMID, NCT, ZIP, DOI, ISBN, phone), and the 24-hour time rule, each checking the offset-map invariants so a dosage expansion still highlights "5 mg".

## 4. Math and chemistry

Statistics and pharmacokinetics formulas written as LaTeX are already spoken well. Measured:

- `$\chi^2 = 3.84$`: "chi squared equals three point eight four".
- `$\hat{\beta}_1 \pm 1.96\,\mathrm{SE}$`: "beta hat sub 1 plus or minus one point nine six SE".
- `$\bar{x} \pm \frac{s}{\sqrt{n}}$`: "x bar plus or minus the fraction with numerator s and denominator square root of n".
- `$t_{1/2} = \frac{0.693}{k_e}$`: "t sub 1 divided by 2 end sub equals the fraction with numerator zero point six nine three and denominator k sub e".
- `$C(t) = C_0 e^{-k_e t}$`: "C t equals C sub 0 e raised to the negative k sub e t power".
- `$\mathrm{CL} = \frac{\text{Dose}}{\mathrm{AUC}}$`: "CL equals the fraction with numerator Dose and denominator AUC".

Two gaps. "t sub 1 divided by 2" should be "t one half"; a table of named subscripts (`1/2`, `max`, `ss`) in `crates/textweaver-math` fixes it. And most statistics in papers is prose, so the range rule in section 3 matters more than the math engine.

MathCAT ([ADR-0029](../../adr/0029-mathcat-speech.md)) has a Chemistry preference whose default reads "H two O" for H₂O and whose Off value says "H sub 2 O" (source 32). It applies only to MathML inside math delimiters. textweaver's own engine does not know mhchem: `$\ce{Na+ + Cl- -> NaCl}$` is "ce N a plus positive C l minus negative greater than N a C l" (measured). mhchem is the standard LaTeX and MathJax package for formulas and equations (source 33); Chemical Markup Language is the XML form (source 34) and SMILES the line notation (source 35), and students almost never meet either.

Alpha scope:

1. `\ce{}` in `crates/textweaver-math`: elements by name or letter at the student's choice, digits as counts, trailing `+` and `-` as charge, `->` as "yields", `<=>` as "in equilibrium with", `^` for isotope mass, and `(s)`, `(l)`, `(g)`, `(aq)` as states. Medium. `tw convert` can emit the formula as `<mi>` and `<msub>` so MathCAT's rules apply in a browser.
2. A plain-text formula reader, off by default: tokens matching element, digit, and charge patterns ("Ca2+", "NaCl") spoken as formulas. Small, after item 1.
3. CML and SMILES: out of scope; read as code.

## 5. Study workflows

### Reading a chapter before lecture

The student hears the outline (Alt+O, [reading guide](../../reading.md)), summarizes the chapter at the cursor ([ADR-0037](../../adr/0037-extractive-summaries.md); `tw summarize`), checks the reading level, and reads at speed. Rate runs from 50 to 900 words per minute, default 265 (`docs/speech.md`). The largest study of listening rates, 453 participants on LabintheWild, found a mean comprehension-limited rate near 300 words per minute, with visually impaired participants faster and each year of screen reader experience adding to it (source 36). A health sciences profile should preset study near 300 and review near 450 through the F8 presets ([speech guide](../../speech.md)). For Piper, synthesis must stay ahead of playback at 450, so the two-sentence lookahead ([architecture](../architecture.md)) should be measured at that rate on the slowest supported machine, with `cargo xtask bench --only first_speech` as the proxy.

### Re-reading with highlights and making study sheets

Highlights are yellow only, with no key to choose a color ([notes guide](../../notes.md)). Students sort passages by kind: high-yield, unclear, drug, mechanism. The fix is categories, not colors: a named category from a list, spoken by name, shown by the theme's styles, exported under its name. The study sheet export groups notes and highlights by heading (`export study sheet`, `crates/textweaver-app/src/study.rs`); add a category filter there and in the highlights list. The Obsidian vault export writes one note per textweaver note with front matter and tags ([vault guide](../../vault.md)). For flashcards without spaced repetition (out of scope, [roadmap](../../roadmap.md)), `tw marks --export csv` with question and answer columns from notes written as "Q: ... A: ..." lets any tool import them.

### Spaced re-reading

Spaced re-reading means returning to a chapter on a schedule the student sets. textweaver records reading time, furthest point, and sessions per document ([reading statistics](../../reading.md); `tw stats`), and Continue reading lists documents by last place ([library guide](../../library.md)). A Review list ordered by days since last read and by notes tagged `#review` is a small addition on the same list model and `stats.json`.

### Citing in APA and AMA, PubMed and DOI lookups

`tw cite` formats APA and AMA among about 80 styles and adds references by DOI through doi.org ([citations guide](../../citations.md)). Health sciences students cite by PMID as often. NCBI's E-utilities serve PubMed records at three requests a second without a key and ten with one (source 43). Add `tw cite add PMID:31769816`: one `efetch` call, the DOI from the record when it has one, else CSL-JSON built from the PubMed XML, cached like DOI answers. The planned PubMed quick-open ([roadmap](../../roadmap.md)) can use the same call to open an abstract as a document with its citation already in the folder library.

### Clinical rotations

textweaver is desktop software; on rotation a student has a phone and a few minutes. Audio export is the mobile path: `tw export-audio` writes M4B audiobooks with a chapter per heading ([audio export guide](../../audio-export.md)). Sync carries notes, highlights, places, and statistics between computers, not to a phone ([sync guide](../../sync.md)); the study sheet and vault export are Markdown, which Obsidian Mobile reads. A pocket export of only the highlighted passages and their notes, as one short audio file plus a sheet, is the ten-minute review a rotation student needs.

## 6. Faculty and office workflows

### Converting a course pack

A course pack arrives as a zip from the LMS, a folder of PDFs, or a scanned reader. `tw convert` converts a folder, converts only what changed, and watches a folder ([converting guide](../../converting.md)); archives open directly and scanned pages are recognized with a cache ([ADR-0026](../../adr/0026-ocr-and-student-formats.md)). Output is tagged PDF checked against PDF/UA-1 while writing (`crates/textweaver-writers/src/lib.rs`), EPUB 3, DOCX, HTML, and BRF. Missing is a report: which files were OCR'd and with which engine, which had no headings, which images had no description, word counts, and listening time at 265 words per minute. That report is what the office files with the request.

### Checking a document before posting

The Department of Justice rule under ADA Title II requires WCAG 2.1 AA of state and local government web content, public universities included, from April 2026 for entities serving 50,000 or more people (source 41). Anthology Ally answers "is this PDF all right to post?" inside the LMS with a score and instructor feedback (source 40). textweaver checks its own output but has no checker for an input document. A `tw check FILE` that runs the loaders and reports scanned pages without a text layer, missing title and language, skipped heading levels, tables without a header row, images without alternative text, columns detected rather than tagged, and error-prone abbreviations gives faculty an offline answer in seconds. `tw lint` ([editing guide](../../editing.md)) is the pattern: one line per problem, status 1, `--json`.

### A report for an accommodation file

`tw marks --json` and `tw stats --json` already print notes, highlights, places, and reading time ([notes guide](../../notes.md)). A `tw report` combining the conversion report, checksums, engines used, and dates, with no reading data unless the student exports it, fits how an office documents a request. Reading data belongs to the student; sync never sends recent files for that reason ([sync guide](../../sync.md)).

### Bookshare and AccessText Network

Bookshare serves people with a certified print disability, free to US students through an OSEP award, in DAISY and BRF among other formats (source 37); textweaver opens DAISY 3 books and Bookshare zips ([ADR-0026](../../adr/0026-ocr-and-student-formats.md)). The AccessText Network is the publishers' clearinghouse through which offices request files for qualifying students; member offices report most requests filled within a day and nearly all within three (sources 38). Those files are usually PDF, with or without tags, which is where the PDF loader and OCR earn their keep.

### Time per request today

No formal survey of hours per request was found. Institutions publish turnaround times: a few hours to several days per scanned book, two days to two weeks when the office must cut and scan, five to ten business days, up to four weeks with editing (source 39). The time goes to four steps: finding a publisher file, scanning, recognition with structure repair, and delivery. textweaver removes most of the third: OCR with layout recovery, running heads removed, columns in order, headings by size, and audio or EPUB with chapters, in seconds per chapter rather than hours. The honest estimate: a scan-and-convert request drops from days to the same day, and the publisher-file wait is untouched.

## 7. Ranked improvements

Small is hours, medium a day or two, large a week or more. Every item works with a screen reader, a braille display, and the keyboard alone, and shows no state by color alone.

| # | Improvement | Users | Evidence or precedent | Crate or file | Size | Target | Performance note |
|---|---|---|---|---|---|---|---|
| 1 | Identifier guard: integers after CPT, PMID, NCT, ZIP, DOI, ISBN, phone read as digits | all | measured misreads, section 3 | `normalize/numbers.rs` rule 11; `tests.rs` | small | alpha.8 | one lookbehind per number; no change to `first_speech` |
| 2 | Deny list for ISMP and Joint Commission abbreviations, spelled never expanded | nursing, pharmacy, medicine | ISMP list (sources 11, 12) | `normalize/abbreviations.rs` | small | alpha.8 | hash set before the regex; nil |
| 3 | Clinical units and dosages: mg, mcg and µg, mL, mmol/L, "per", en dash ranges as "to", 24-hour time without AM | all | section 3 | `numbers.rs`, `punctuation.rs` | medium | alpha.8 | two more rules in the chain; measure on 10 MB |
| 4 | Names for symbols outside math: Greek letters, µ, −, ⇌, →, ×10^n | research, pharmacology | `math.md` already names ×, ≤, ∞ outside formulas | `punctuation.rs` NAMES | small | alpha.8 | table lookup; nil |
| 5 | Medical lexicon tier with hash lookup, Telnyx seed, Tall Man words kept whole | all | CMUdict covers 45 percent of 240 terms; `community.rs` | new `normalize/medical.rs`; `third_party/lexicon-medical/` | medium | alpha.9 | one lookup per word on the speech thread; 120 KB data; no binary growth |
| 6 | Piper IPA override spliced into `prepare` with word owners kept | Piper users | `phonemes.rs` builds the ids | `textweaver-piper/src/phonemes.rs` | medium | alpha.9 | per-clause splice; measure first speech at 450 wpm |
| 7 | `add pronunciation` with spoken preview; `lexicon_files` for an office; NVDA and JAWS import | students, offices | sources 30, 31; pronunciations already sync | `textweaver-app`, `textweaver-cli` | medium | alpha.9 | preview on the speech thread; file reads on the writer thread |
| 8 | `\ce{}` chemistry speech and MathML output | biochemistry, pharmacology | mhchem (source 33); MathCAT chemistry (source 32) | `textweaver-math`, `textweaver-render` | medium | alpha.9 | parsed once per formula; nil |
| 9 | `tw check FILE`: accessibility report for input documents | faculty, office | DOJ rule (source 41); Ally (source 40) | `textweaver-cli`, `textweaver-formats` | large | alpha.9 first cut, later full | reuses the loaders; runs in `tw`, never on the reader's input thread |
| 10 | PMID lookup in `tw cite add`; PubMed quick-open on the same call | all | E-utilities (source 43); roadmap | `textweaver-cite`, `app/src/citations.rs` | medium | alpha.9 | one HTTPS call on a background thread, cached |
| 11 | Highlight categories by name, filters in list and study sheet | students | notes.md: yellow only | `app/src/marks.rs`, `study.rs`, `textweaver-store` | medium | alpha.9 | one byte per highlight; nil |
| 12 | Reference superscripts in untagged PDFs read as "reference 12" and deferred like footnotes | research, medicine | `layout.rs` joins superscripts into the line | `pdf/layout.rs`, `pdf/structure.rs` | medium | alpha.9 | one pass over spans already measured; check `open` on the 300-page fixture |
| 13 | Conversion and request report (`tw convert --report`, `tw report`) | office | section 6 | `textweaver-convert`, `textweaver-cli` | small | alpha.9 | JSON after the batch; nil |
| 14 | Review list by days since last read and `#review` notes | students | stats and Continue reading exist | `app/src/study.rs`, `list_model.rs` | small | later | from `stats.json` already in memory |
| 15 | Pocket export: highlights and notes as one short M4B plus a sheet | rotation students | audio export and study sheet exist | `app/src/audio_export.rs`, `study.rs` | medium | later | same export path on the export thread |

## Sources

1. https://support.vitalsource.com/hc/en-us/p/accessibility
2. https://success.vitalsource.com/hc/en-us/articles/360026396654
3. https://about.redshelf.com/accessibility
4. https://solve.redshelf.com/hc/en-us/articles/360015237493-Text-to-Speech-TTS
5. https://leap.ocls.ca/node/1605 (NOSM University assessment of AccessMedicine; not reachable from this machine, unverified)
6. https://www.elsevier.support/ckstudent/answer/what-is-clinicalkey-students-accessibility-policy
7. https://download.lww.com/vpat/wk-thepoint-accessibility.pdf
8. https://www.uworld.com/assets/data/Ada_Request_Form.pdf
9. https://castreader.com/en/blog/uworld-text-to-speech (third party, unverified)
10. https://www.ncbi.nlm.nih.gov/books/ (NCBI Bookshelf, where StatPearls is published)
11. https://home.ecri.org/blogs/ismp-resources/list-of-error-prone-abbreviations
12. https://home.ecri.org/blogs/ismp-news/ismp-updates-list-of-error-prone-abbreviations-symbols-and-dose-designations
13. https://www.fda.gov/drugs/medication-errors-related-cder-regulated-drug-products/fda-name-differentiation-project
14. https://online.ecri.org/hubfs/ISMP/Resources/ISMP_Look-Alike_Tallman_Letters.pdf
15. https://doi.org/10.1136/bmjqs-2015-004562 (Zhong and others, 2016, via PubMed)
16. https://doi.org/10.1371/journal.pone.0145431 (Bryan and others, 2015, via PubMed)
17. https://doi.org/10.1001/jama.2019.15372 (Meeks and others, 2019, via PubMed)
18. https://doi.org/10.1111/medu.14995 (Meeks and others, 2022, via PubMed)
19. https://www.nlm.nih.gov/databases/umls.html
20. https://www.nlm.nih.gov/research/umls/Snomed/snomed_license.html
21. https://www.nlm.nih.gov/research/umls/rxnorm/docs/termsofservice.html
22. https://github.com/cmusphinx/cmudict
23. https://github.com/team-telnyx/medical-pronunciation-dictionary
24. https://github.com/open-dict-data/ipa-dict
25. https://github.com/glutanimate/wordlist-medicalterms-en
26. https://github.com/espeak-ng/espeak-ng/blob/master/docs/dictionary.md
27. https://github.com/espeak-ng/espeak-ng/blob/master/dictsource/en_list
28. https://github.com/OHF-Voice/piper1-gpl
29. https://msdn.microsoft.com/en-us/library/ms717899.aspx (ISpLexicon, SAPI 5.3)
30. https://www.nvaccess.org/files/nvda/documentation/userGuide.html (NVDA User Guide, Speech Dictionaries)
31. https://support.freedomscientific.com/teachers/lessons/9.1_DictionaryManagerIntroduction.htm
32. https://github.com/NSoiffer/MathCAT/blob/main/docs/users.md
33. https://ctan.org/pkg/mhchem
34. http://www.xml-cml.org/
35. http://opensmiles.org/opensmiles.html
36. https://par.nsf.gov/biblio/10063856 (Bragg and others, CHI 2018) and https://doi.org/10.1145/3461700 (the 2021 expansion)
37. https://www.bookshare.org/cms/node/60
38. https://ati.gmu.edu/accessible-text/accessible-textbooks/ and https://news.uga.edu/access-text-network-created-in-collaboration-with-uga-will-improve-student/
39. https://accessibility.ucdavis.edu/textbooks , https://www.uh.edu/accessibility/accommodations/alternate-format-textbooks/ , https://www.towson.edu/accessibility-disability-services/accommodations-services/print-materials.html , https://ds.oregonstate.edu/book/das-student-handbook/chapter-7-accommodation-spelling
40. https://help.blackboard.com/Learn/Administrator/Hosting/Tools_Management/Ally
41. https://doit.illinois.gov/doit-news/accessibility-news/doj-finalizes-rule-on-web-and-mobile-accessibility.html
42. https://www.w3.org/TR/pronunciation-use-cases/
43. https://www.ncbi.nlm.nih.gov/books/NBK25497/ (E-utilities)
44. https://www.mheducation.com/unitas/highered/accessibility/mckinley-human-anatomy-6e.pdf (a McGraw Hill per-title accessibility report)
45. https://www.mass.gov/doc/nclex-applicants-requesting-test-accommodations-due-to-a-disability-0/download
46. https://link.springer.com/chapter/10.1007/978-3-031-08648-9_30 and https://www.perkins.org/resource/how-to-modify-accessible-anatomy-graphics-for-low-vision/
47. https://doi.org/10.4140/tcp.n.2007.870 (Wick, 2007, via PubMed)

## See also

- [README.md](README.md): the research index in this folder.
- [Reading and writing math](../../math.md): how formulas are spoken and explored.
- [Citations and references](../../citations.md): `tw cite`, APA, AMA, DOI lookups.
- [Bookmarks, notes, and highlights](../../notes.md): study sheets and exports.
- [ADR-0026: OCR, and formats for students](../../adr/0026-ocr-and-student-formats.md): scans, DAISY, archives, slides.
- [ADR-0010: PDF loader](../../adr/0010-pdf-loader.md): columns, running heads, tables.
- [ADR-0025: Define word offline](../../adr/0025-lexicon-and-message-catalog.md): the CMUdict data file.
- [Speech](../../speech.md): pronunciations, the community lexicon, rate presets.
- [Architecture](../architecture.md): the speech thread and where normalization runs.
