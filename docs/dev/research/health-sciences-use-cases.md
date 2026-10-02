# Health sciences education: materials, pronunciation, and study workflows

This report says what students and faculty in medicine, nursing, dentistry, pharmacy, physician assistant programs, public health, and biomedical research need from a reading and writing tool, and what textweaver should build for them. It is for the textweaver maintainers planning alpha.8 and alpha.9, and for the accommodations office that will deploy it. Written Friday, October 2, 2026, against the repository at 0.1.0-alpha.7. Where a claim about textweaver's own behavior was checked, it was checked by running the normalization pipeline from `crates/textweaver-speech/src/normalize/mod.rs` on clinical sentences that day, through a scratch program kept outside the repository; those results are marked "measured".

## Who the users are

Students with disabilities are a growing share of health sciences cohorts. According to PubMed, Meeks and colleagues surveyed US allopathic medical schools in 2016 and again in 2019 and reported a rise in students disclosing a disability, with ADHD, learning disabilities, and psychological disabilities the largest groups ([DOI](https://doi.org/10.1001/jama.2019.15372), source 17). A national AAMC questionnaire study found 10.2 percent of responding second-year students self-reporting a disability, and half of them using accommodations ([DOI](https://doi.org/10.1111/medu.14995), source 18). The accommodations most relevant to a reading tool are extended time, alternate formats, and text-to-speech. Faculty and the accommodations office are the second user group: they convert course packs, check documents before posting, and keep records.

## 1. The materials

Health sciences reading is dense in symbols, numbers, and names that speech engines were not built for. The table lists each kind of material, what breaks when it is read aloud or by a screen reader, and what textweaver does today, with the file that does it.

| Material | Where it comes from | What breaks in speech and screen readers | textweaver today |
|---|---|---|---|
| Textbooks on publisher platforms | VitalSource Bookshelf, RedShelf, McGraw Hill AccessMedicine, Elsevier ClinicalKey Student, Wolters Kluwer thePoint and Lippincott, UpToDate | Reading stays inside the platform. Bookshelf and RedShelf have built-in read aloud with highlighting and screen reader support (sources 1 to 4). AccessMedicine and ClinicalKey are web chapters with VPATs (sources 5 to 7). None lets the student use their own voice, lexicon, rate presets, or notes workflow, and none reads math or formulas as structure. | textweaver reads what the student can download or copy: EPUB, PDF, HTML, DOCX (`crates/textweaver-formats/src/`). Platform-locked titles need the publisher file from AccessText or Bookshare (section 6). |
| Journal PDFs | PubMed Central, publisher sites, library proxies | Two columns read across, running headers and page numbers inside sentences, figure captions mixed into text, tables flattened, reference superscripts read as numbers in the sentence ("heart failure 12 is"), supplementary data as separate spreadsheets. | The PDF loader rebuilds columns, removes running heads and page numbers, finds tables, and reads captions as captions (`crates/textweaver-formats/src/pdf/layout.rs`, `structure.rs`; [ADR-0010](../../adr/0010-pdf-loader.md), [ADR-0048](../../adr/0048-pdf-annotations-links-and-forms.md)). Superscript reference numbers are joined back into the line (`layout.rs`, "superscripts join lines later") and are then read as plain digits. Footnote markers come only from tagged notes (`structure.rs`, `MarkerKind::Footnote`). |
| Lab protocols | Course handouts, lab manuals, PDF and DOCX | Numbered steps with volumes and concentrations ("add 2.5 mL of 0.1 M NaOH"), units, temperatures, Greek letters (µL), chemical formulas, warnings in colored boxes. | Decimals, percents, and degrees are read (`normalize/numbers.rs`, `punctuation.rs`). Measured: "25 µg" is passed through as written, "H2O" and "HCO3−" are passed through, and the engine decides. |
| Drug monographs | Lexicomp, Micromedex, package inserts, StatPearls | Dosage lines ("5 mg q6h PRN"), look-alike names, Tall Man capitals, tables of adverse effects by frequency, pediatric weight-based doses (mg/kg). | Measured: "Give 5 mg q6h PRN for pain." is spoken as written; "q6h" and "PRN" are left to the engine. "mg/kg" is "mg slash kg". |
| Clinical guidelines | Societies, CDC, WHO | Recommendation grades ("Class I, Level B"), long tables, algorithms as flowcharts in images, confidence intervals and p-values. | Tables are narrated with row and column positions (`normalize/markdown.rs`, `table_mode`). Flowchart images read only through alternative text. Measured: "95% CI 1.2–3.4" is "ninety-five percent CI one point two–three point four"; the en dash goes to the engine. |
| Exam preparation | UWorld, board question banks, NCLEX practice | UWorld's accessibility statement offers accommodations by request (source 8); a third-party page says the question bank has no read-aloud of its own (source 9, unverified against UWorld). Questions are copied out one at a time. NCLEX itself allows JAWS as a documented accommodation through the state board and NCSBN (source 45). | Students paste questions into a new document and read with their voice. textweaver's clipboard and edit mode serve this ([editing guide](../../editing.md)). No quick way to keep the question, the answer, and the explanation together as a note. |
| Case files | Course PBL cases, DOCX and PDF | Vitals and labs in tables with reference ranges, time stamps in 24-hour clock, abbreviations in the record. | Measured: "at 08:05" becomes "eight oh five AM" (the time rule adds AM to a 24-hour time); "BP 120/80 mmHg" becomes "BP 120 slash 80 mmHg". |
| EHR training material | Epic and Cerner training PDFs, screenshots | Screenshots carry the content; text layers are often missing. | OCR of pages with no text layer ([ADR-0026](../../adr/0026-ocr-and-student-formats.md)); screenshot text is read as one page. Table structure from OCR is not done. |
| Anatomy atlases | Netter, Gray's, Acland videos | Labeled figures with leader lines; the text is in the picture. Tactile and 3D models are the known workaround for blind students (sources 46). | Only alternative text is read. OCR of a labeled plate gives a bag of labels with no relation to the structures. |
| Chemical structures | Biochemistry and pharmacology texts | Skeletal formulas are images. Line notations (SMILES) and markup (CML, mhchem) exist but rarely reach the student. | Measured: `$\ce{H2O}$` is "ce H 2 O": the mhchem command is unknown to `crates/textweaver-math` and is spoken by name, as [math.md](../../math.md) says. |
| Lecture slides | PPTX and PDF exports from the LMS | Text boxes out of order, speaker notes hidden, images without alt text, tables as pictures. | PPTX reads slides in order, body placeholders as lists, speaker notes under a heading ([ADR-0026](../../adr/0026-ocr-and-student-formats.md)). |

What a reader must do for all of these: keep reading order, say tables as tables, name symbols the engine would skip, expand only the abbreviations that are safe to expand, and let the student fix a pronunciation once and keep it. The next three sections take those in turn.

## 2. Pronunciation

### How engines go wrong

Rule-based engines (espeak-ng, Eloquence, DECtalk, the older SAPI voices) pronounce an unknown word by letter-to-sound rules. espeak-ng's English word list, `en_list`, has 5,794 lines and its rule file, `en_rules`, 7,132 lines (source 27). On Friday, October 2, 2026, `en_list` had no entry for acetaminophen, metformin, warfarin, lisinopril, atorvastatin, dyspnea, or ischemia, so every one of those is spelled out by rule. Piper voices take their phonemes from espeak-ng (`crates/textweaver-piper/src/phonemes.rs`; the Piper project embeds espeak-ng for phonemization, source 28), so a Piper voice inherits every espeak-ng mistake, however natural its voice. Neural engines also stress the wrong syllable in long Greek and Latin compounds, and Eloquence reads "mcg" and "PRN" as words.

The failures that matter in health sciences are:

- stress on the wrong syllable in drug names (atorvastatin, levetiracetam) and in anatomical terms (sternocleidomastoid);
- "-emia", "-itis", "-ectomy", and "-ostomy" endings read with the wrong vowel;
- eponyms from French and German (Guillain-Barré, Sjögren, Raynaud, Wernicke) read as English spellings;
- look-alike pairs read alike (hydroxyzine and hydralazine), which is a safety problem, not only a comfort problem;
- Tall Man capitals split into fragments. Measured: with split caps on, "hydrOXYzine" becomes "hydr OX Yzine" and "DOPamine" becomes "DO Pamine" (`normalize/punctuation.rs`, `SplitCaps`).

### Term sources and their licenses

| Source | What it gives | License | Size | Fit for textweaver |
|---|---|---|---|---|
| CMUdict, already shipped in `third_party/lexicon/lexicon-en.twlex` ([ADR-0025](../../adr/0025-lexicon-and-message-catalog.md)) | ARPAbet pronunciations | BSD-style (source 22) | 126,052 words (`third_party/lexicon/README.md`) | Measured coverage of 240 common clinical terms, Friday, October 2, 2026: 107 found, 133 missing (45 percent). Missing: metoprolol, atorvastatin, lisinopril, metformin, warfarin, prednisone, ceftriaxone, furosemide, creatinine, troponin, pancreatitis, auscultation, ileum, jejunum, and most "-itis", "-emia", and "-uria" words. |
| Telnyx medical pronunciation dictionary | Respelling alias and IPA for each term | Data CC BY 4.0, code MIT (source 23) | 966 entries: 398 drugs, 267 clinical terms, 152 anatomical, 149 acronyms | Direct fit: the alias column is what `[normalization] pronunciations` takes today. |
| ipa-dict (en_US) | IPA for general English | MIT (source 24) | large general list | Backstop for Piper phoneme overrides; no medical focus. |
| wordlist-medicalterms-en | Medical words, no pronunciations | GPL v3 (source 25) | 98,119 terms | A list of which words need entries; compatible with textweaver's GPL-3.0-or-later. |
| RxNorm | Drug names and codes | Everything tagged SAB=RXNORM is public domain; other sources need the UMLS license (source 21) | thousands of ingredient and brand names | Term list for drugs; no pronunciations. |
| UMLS Metathesaurus and SNOMED CT | Clinical terms | Free UMLS license; SNOMED CT free in member countries under the affiliate license, with redistribution limits (sources 19, 20) | millions of strings | Too large and too restricted to ship. Useful offline to pick the top terms. |
| FDA and ISMP Tall Man lists | Look-alike name pairs with recommended capitals | Public (sources 13, 14) | FDA list is short; ISMP's is longer and separate | Both lists go into the lexicon as whole words so split caps never cuts them. |

The Tall Man lists are made for labels and screens. A nine-year time series in 42 children's hospitals found no change in look-alike error rates after Tall Man lettering was introduced ([DOI](https://doi.org/10.1136/bmjqs-2015-004562), source 15), and a study of 7,987 international nonproprietary names showed how shared stems make the names similar by design ([DOI](https://doi.org/10.1371/journal.pone.0145431), source 16). For speech the lesson is the opposite of the visual one: a reader must make the two names sound different, by stress and by syllable, not by capitals.

### Lexicon formats the engines accept

- CMUdict ARPAbet: `W AO1 R F ER0 IH0 N`, stored one byte per phone in textweaver's data file and respelled for reading aloud as `WAR-fuh-rin` (`crates/textweaver-lexicon/src/pronounce.rs`).
- espeak-ng: its own mnemonics in `en_list`, with an `en_extra` file for a user's own words, compiled into `en_dict` (source 26). The pure-Rust port textweaver bundles writes its built-in data into a folder on first use (`phonemes.rs`, `RustPhonemizer::new`); whether that port compiles an `en_extra` file is unverified.
- Piper: IPA phoneme strings, one id per character, from the voice's `phoneme_id_map` (`crates/textweaver-piper/src/config.rs`). textweaver already builds the id list itself in `phonemes.rs::prepare`, so an IPA override needs no change to the model.
- SAPI 5: `ISpLexicon::AddPronunciation` adds a word to the user lexicon in the SAPI phone set, shared by every SAPI application (source 29).
- Eloquence: `main`, `abbr`, and `root` dictionaries, one `word<TAB>respelling` line each; textweaver ships the community set (`crates/textweaver-speech/src/normalize/community.rs`).
- NVDA: default, voice, and temporary speech dictionaries; each entry is a pattern, a text replacement, case sensitivity, and a type of anywhere, whole word, or regular expression (source 30). Replacements are text, not phonemes.
- JAWS: Dictionary Manager writes `.jdf` files, one default and one per application (source 31).

A student who uses NVDA or JAWS already keeps a dictionary. textweaver's own lexicon should import those formats rather than ask the student to type the entries twice.

### Proposal: a medical lexicon for textweaver

Data. Ship `lexicon/medical-en.toml`, a plain `term = "respelling"` file in the format `[normalization.pronunciations]` already reads ([speech guide](../../speech.md)), with an optional second column of IPA for Piper. Seed it with the Telnyx dictionary (966 entries, CC BY 4.0, credited in `THIRD-PARTY-NOTICES.md` as the lexicon sources are), then extend it to about 2,500 entries from the RxNorm ingredient names and the top terms of the medical word list, with respellings written and checked by ear. Record every source and sum in `third_party/`, as `third_party/lexicon/README.md` does today. Size on disk: about 120 KB; in the binary, nothing, because it is a data file beside the program.

Pipeline. The `Pronunciations` transform (`crates/textweaver-speech/src/normalize/abbreviations.rs`) builds one regular expression alternation of every term, longest first, case-insensitive. That is right for a few dozen user entries and wrong for 2,500: the alternation grows with every term, and it is rebuilt whenever settings change. Add a `MedicalLexicon` transform modeled on `CommunityLexicon` (`community.rs`), which looks each word up in a hash map, lower-casing a capitalized word and splitting a hyphenated one. Order: user pronunciations first, then medical, then community, so a student's own entry always wins. Keep it a normalization option, `[normalization.medical_lexicon] enabled`, on by default in a health sciences settings profile and off otherwise. For Piper, add a second hook in `textweaver-piper/src/phonemes.rs::prepare`: before a clause is phonemized, words with an IPA entry are cut out, the rest is phonemized, and the IPA is spliced in with its word owner, so the highlight stays exact.

Run-time cost. A hash lookup per word on the speech thread, which already runs normalization just before the two-sentence lookahead ([architecture](../architecture.md)); nothing touches the input thread. Measure with `cargo xtask bench --only first_speech` and the narration plan numbers on the 10 MB corpus before and after: the budget is no measurable change at the 1 MB size and under 5 percent on 10 MB. Loading the file once at startup: about a millisecond for 2,500 lines, on the speech thread's first use.

Adding terms. For a student: a palette command `add pronunciation` on the word at the cursor, which asks for the respelling, says it back through the current engine, and writes it to `[normalization.pronunciations]`, which already syncs between computers ([sync guide](../../sync.md)). For an office: `[normalization] lexicon_files = ["\\\\share\\osa\\pronunciations.toml"]`, read on startup and again when changed, so one coordinator maintains the list for every student. Import: `tw lexicon import nvda.dic` and `tw lexicon import jaws.jdf` for whole-word text entries only; regular expression entries are reported and skipped. Check what it will say with `tw speak --backend null --json`, which [math.md](../../math.md) already documents for formulas.

### Twenty test terms

Each row is a test case for the new lexicon. The expected column is the respelling the lexicon should carry; whether a given engine says it right without the lexicon is to be checked by ear per engine and is not claimed here.

| Term | Why it is hard | Expected respelling | In CMUdict today |
|---|---|---|---|
| acetaminophen | stress on the third syllable | uh-SEE-tuh-MIN-uh-fen | yes |
| atorvastatin | five syllables, two stresses | uh-TOR-vuh-STAT-in | no |
| metoprolol | stress on "TOP" | meh-TOE-pruh-lol | no |
| warfarin | first syllable "WAR" | WAR-fuh-rin | no |
| lisinopril | "SIN" stressed | lye-SIN-oh-pril | no |
| hydroxyzine vs hydralazine | look-alike pair | hye-DROK-sih-zeen; hye-DRAL-uh-zeen | no, no |
| ceftriaxone | "ax" as "AKS" | sef-try-AKS-own | no |
| phenytoin | "FEN-ih-toyn", not "fen-i-TOE-in" | FEN-ih-toyn | no |
| levetiracetam | often mangled | lee-veh-tye-RASS-eh-tam | no |
| furosemide | "fyoo-ROH" | fyoo-ROH-seh-mide | no |
| dyspnea | silent p | DISP-nee-uh | yes |
| ischemia | "is-KEE" | is-KEE-mee-uh | yes |
| cholecystitis | "koh-lee-sis-TYE-tis" | koh-lee-sis-TYE-tis | no |
| creatinine | "kree-AT-ih-neen" | kree-AT-ih-neen | no |
| sphygmomanometer | long compound | sfig-moh-muh-NOM-eh-ter | no |
| Guillain-Barré | French | ghee-YAN bah-RAY | no |
| Sjögren | Swedish, umlaut | SHOW-grin | not checked |
| Raynaud | French, silent d | ray-NOH | no |
| ileum vs ilium | sound-alike pair, different organs | ILL-ee-um (both; context decides) | no, yes |
| Wernicke | German | VER-nih-kuh | yes |

## 3. Numbers, units, and safety

These are the measured outputs of the default pipeline (`Pipeline::for_settings`, punctuation "some", split caps off, engine without native normalization) on Friday, October 2, 2026, and what a health sciences reader needs instead.

| Input | textweaver today (measured) | Needed |
|---|---|---|
| `Give 5 mg q6h PRN for pain.` | spoken as written | "5 milligrams every 6 hours as needed". With a lexicon entry for q6h and PRN, measured: "Give 5 mg every 6 hours as needed". |
| `0.5 mg, not 5.0 mg` | "zero point five mg, not five point zero mg" | Correct. Keep the leading zero; never drop a trailing zero silently. |
| `taper over 6–8 weeks` | "6–8" passed to the engine | "6 to 8 weeks". The en dash between numbers is a range. |
| `95% CI 1.2–3.4; p < 0.05; p = .03` | "ninety-five percent CI one point two–three point four; p less than zero point zero five; p equals point zero three" | Correct except the range: "one point two to three point four". |
| `K+ 3.5 mEq/L; Ca2+; Na+` | "K plus three point five mEq slash L; Ca2 plus; Na plus" | "K plus", "calcium two plus", "sodium plus" when a chemistry reader is on; "per liter" for "/L". |
| `25 mcg vs 25 µg` | both passed through | "25 micrograms" for both. ISMP says µg is misread as mg and should not be written (source 11); the reader should say the unit in full. |
| `CPT 99213; PMID 31769816; ZIP 97239` | "ninety-nine thousand two hundred thirteen; thirty-one million seven hundred sixty-nine thousand eight hundred sixteen; ninety-seven thousand two hundred thirty-nine" | Digits: "nine nine two one three". Rule 11 of `normalize/numbers.rs` reads every integer of four or more digits as a number. |
| `ICD-10 E11.9; NCT04368728` | spoken as written | Correct as is; "E eleven point nine" is how clinicians say it. |
| `at 08:05` | "eight oh five AM" | "eight oh five". A 24-hour time without AM or PM must not gain one (`numbers.rs`, rule 3). |
| `TNF-α and IL-6; 10 U insulin; QD and QOD` | passed through | "TNF alpha and IL 6"; "10 units insulin"; "Q D" and "Q O D" spelled, never expanded (see below). |
| `38.5°C; SpO2 98%; 120/80 mmHg` | "thirty-eight point five degrees C; SpO2 ninety-eight percent; 120 slash 80 mmHg" | "degrees Celsius"; "S p O 2"; "120 over 80 millimeters of mercury". |
| `WBC 11.5 × 10^9/L` | "eleven point five times 10 caret 9 slash L" | "times ten to the ninth per liter". Outside a formula, `^` is only named. |
| `BRCA1, TP53, HER2, p53; 99mTc; 131I` | passed through | Gene symbols spelled letter by letter with the digits ("B R C A one"); isotopes "technetium 99 m" and "iodine 131" from a small table. |

Decimals, percents, and currency are already right, including "2.5%" as "two point five percent". A European decimal comma, "2,5%", is read as "2,five percent" and should be left alone or read as a decimal when the document language says so.

### What an abbreviation expander must never expand

The ISMP List of Error-Prone Abbreviations, Symbols, and Dose Designations (source 11, updated 2024, source 12) and The Joint Commission's Do Not Use list, which Wick summarizes ([DOI](https://doi.org/10.4140/tcp.n.2007.870), source 47), name abbreviations that have caused harm because they are misread. A reader has the same problem in sound. The rule for textweaver: an abbreviation on those lists is spelled out letter by letter, never guessed, and at high verbosity the reader adds "error-prone abbreviation". The list to protect, in `normalize/abbreviations.rs` as a deny list checked before any expansion:

- U and IU (units): spell "U"; a lexicon may say "units" only after a number and a known drug (10 U insulin).
- QD, QOD, q.d., q.o.d. (daily, every other day): spell, never "every day" or "four times".
- MS, MSO4, MgSO4: spell; morphine sulfate and magnesium sulfate must not be guessed.
- cc: spell; "mL" is the unit to expand.
- µg: say "micrograms", because the symbol itself is the hazard.
- SC, SQ, HS, TIW, AD, AS, AU, OD, OS, OU: spell.
- "@", "<", ">": name the symbol ("at", "less than", "greater than"), which `punctuation.rs` already does.
- Trailing zero ("5.0") and missing leading zero (".5"): read the digits as written; measured, both are read correctly today.
- Drug-name abbreviations such as TPA and HCTZ: spell, never expand to a drug.

Safe expansions for the clinical lexicon: PRN (as needed), PO (by mouth), BID, TID, QID (twice, three times, four times daily), q4h, q6h, q8h (every n hours), IV, IM (I V, I M, already spelled by every engine), NPO (nothing by mouth), prn, stat.

### Tests to add

Add a `clinical_vectors` test to `crates/textweaver-speech/src/normalize/tests.rs` in the style of `normalize_numbers_vectors`, with the table above as its vectors, plus the deny list (each entry must come out unchanged or spelled), the range rule, the identifier rule (CPT, PMID, NCT, ZIP, DOI, ISBN, phone numbers), and the 24-hour time rule. Each vector also checks the offset-map invariants, as the existing tests do, so a dosage expansion still highlights "5 mg".

## 4. Math and chemistry

Statistics and pharmacokinetics formulas are already spoken well by textweaver's own engine when they are written as LaTeX. Measured on Friday, October 2, 2026:

- `$\chi^2 = 3.84$` is "chi squared equals three point eight four".
- `$\hat{\beta}_1 \pm 1.96\,\mathrm{SE}$` is "beta hat sub 1 plus or minus one point nine six SE".
- `$\bar{x} \pm \frac{s}{\sqrt{n}}$` is "x bar plus or minus the fraction with numerator s and denominator square root of n".
- `$t_{1/2} = \frac{0.693}{k_e}$` is "t sub 1 divided by 2 end sub equals the fraction with numerator zero point six nine three and denominator k sub e".
- `$C(t) = C_0 e^{-k_e t}$` is "C t equals C sub 0 e raised to the negative k sub e t power".
- `$\mathrm{CL} = \frac{\text{Dose}}{\mathrm{AUC}}$` is "CL equals the fraction with numerator Dose and denominator AUC".

Two things are missing. First, "t sub 1 divided by 2" should be "t one half" for half-life; a two-entry table of named subscripts (`1/2`, `max`, `min`, `ss`) in `crates/textweaver-math` fixes it. Second, statistics in health sciences papers is mostly not in LaTeX: "OR 2.3 (95% CI 1.1–4.8)" is prose, so the range rule in section 3 matters more than the math engine.

MathCAT, textweaver's second math engine ([ADR-0029](../../adr/0029-mathcat-speech.md)), has a "Chemistry" preference whose default reads a chemical formula out, "H two O" for H₂O, and whose "Off" value says "H sub 2 O" (source 32). It applies only to MathML, so it helps when a formula is inside math delimiters and nothing else. textweaver's own math engine does not know mhchem: `$\ce{H2O}$` is "ce H 2 O" and `$\ce{Na+ + Cl- -> NaCl}$` is "ce N a plus positive C l minus negative greater than N a C l" (measured). mhchem is the standard LaTeX and MathJax package for chemical formulas and equations (source 33). Chemical Markup Language is the XML form (source 34), and SMILES is the line notation for structures (source 35); both are machine formats that students almost never see in course material.

A reasonable alpha-level scope:

1. `\ce{}` in `crates/textweaver-math`: element symbols read by name or by letter at the student's choice, digits after a symbol as counts ("H two O"), `+` and `-` after a formula as charge ("sodium plus", "chloride minus"), `->` as "yields", `<=>` as "in equilibrium with", `^` for isotope mass, and `(s)`, `(l)`, `(g)`, `(aq)` as states. Medium size. MathML output in `tw convert` can wrap the formula as `<mi>` and `<msub>` elements so MathCAT's chemistry rules apply in a browser.
2. A plain-text formula reader, off by default and on in the health sciences profile: a token that matches element symbol, digit, and charge patterns ("Ca2+", "HCO3−", "NaCl") is spoken as a formula. Small, after the first item, because the patterns are the same.
3. CML and SMILES: out of scope for alpha. Read as code, which the reader skips or spells by setting.

## 5. Study workflows

### Reading a chapter before lecture

A student opens the chapter, hears the outline (Alt+O, [reading guide](../../reading.md)), asks for a summary of the chapter at the cursor ([ADR-0037](../../adr/0037-extractive-summaries.md); `tw summarize`), checks the reading level, and then reads at speed. Rate runs from 50 to 900 words per minute, default 265 (`docs/speech.md`). The largest study of listening rates, run on LabintheWild with 453 participants, found a mean comprehension-limited rate near 300 words per minute, with visually impaired participants faster and each year of screen reader experience adding to the rate (source 36). So a health sciences profile should default to a study preset near 300 and an exam-review preset near 450, which textweaver's F8 presets already allow ([speech guide](../../speech.md)). The performance implication is for Piper: at 450 words per minute the synthesis must stay ahead of playback, so the two-sentence lookahead ([architecture](../architecture.md)) should be measured at that rate on the slowest supported machine, with `cargo xtask bench --only first_speech` as the proxy.

### Re-reading with highlights and making study sheets

Highlights are yellow only, with no key to choose a color ([notes guide](../../notes.md)). Health sciences students sort passages by kind: high-yield, unclear, drug, mechanism. The fix is categories, not colors: a highlight takes a named category chosen from a list, spoken by name, shown by the theme's styles, and exported under its name. Notes already take tags, and the study sheet export groups notes and highlights by heading (`export study sheet`, `crates/textweaver-app/src/study.rs`). Add a category filter to the study sheet and to the highlights list.

The Obsidian vault export writes one note per textweaver note with front matter, tags, and a link back to the document ([vault guide](../../vault.md)). That is the right path for students who keep a second brain. For students who want flashcards without spaced repetition (out of scope, [roadmap](../../roadmap.md)), a `tw marks --export csv` with question and answer columns, taken from notes written as "Q: ... A: ...", lets any flashcard tool import them.

### Spaced re-reading

Spaced re-reading means coming back to the same chapter on a schedule the student sets, not an algorithm choosing cards. textweaver records reading time, furthest point, and sessions per document ([reading statistics](../../reading.md); `tw stats`), and Continue reading lists documents by last place ([library guide](../../library.md)). A "Review" list that orders documents by days since last read and by notes tagged `#review`, with the count of unresolved notes, is a small addition on the same list model and the same `stats.json`.

### Citing in APA and AMA, PubMed and DOI lookups

`tw cite` formats APA and AMA among about 80 styles and adds references by DOI through doi.org ([citations guide](../../citations.md)). Health sciences students cite by PMID as often as by DOI. NCBI's E-utilities serve PubMed records without a key at three requests a second and with a free key at ten (source 43). Add `tw cite add PMID:31769816`: one `efetch` call for the record, read the DOI from it when it has one and fall back to building the CSL-JSON from the PubMed XML when it does not, cached like DOI answers. The planned PubMed quick-open ([roadmap](../../roadmap.md)) can use the same call to open an abstract as a document, with the citation already in the folder library.

### Clinical rotations: reading on a phone or in a hallway

textweaver is desktop software. On rotation a student has a phone, a few minutes, and no keyboard. That implies three things. First, audio export is the mobile path: `tw export-audio` writes M4B audiobooks with a chapter per heading and the document's title and author ([audio export guide](../../audio-export.md)), which any phone audiobook player opens. Second, sync through a folder the student already shares (Syncthing, OneDrive) carries notes, highlights, places, and statistics between computers but not to a phone ([sync guide](../../sync.md)); the study sheet and the vault export are Markdown, which Obsidian Mobile reads. Third, a "pocket export" that writes only the highlighted passages and their notes as one short audio file plus a Markdown sheet would give a rotation student the ten-minute review they need. Pre-rounding at a nurses' station is a listening task.

## 6. Faculty and office workflows

### Converting a course pack

A course pack arrives as a zip from the LMS, a folder of PDFs, or a scanned reader. `tw convert` converts a folder, converts only what changed, and watches a folder for new files ([converting guide](../../converting.md)); archives open directly ([ADR-0026](../../adr/0026-ocr-and-student-formats.md)); scanned pages are recognized in process for English and through Tesseract for other languages, with a cache so the second opening is instant. Output is tagged PDF checked against PDF/UA-1 while writing (`crates/textweaver-writers/src/lib.rs`), EPUB 3 with accessibility metadata, DOCX, HTML, and BRF. What is missing is a report: which files were OCR'd and with which engine, which had no headings, which images had no description, how many words, and the estimated listening time at 265 words per minute. That report is what the office files with the request.

### Checking a document before posting

The Department of Justice rule under ADA Title II requires WCAG 2.1 AA for state and local government web content, including public universities, with the first compliance date in April 2026 for entities serving 50,000 or more people (source 41). Faculty now ask "is this PDF all right to post?" Anthology Ally answers that inside the LMS with a score and instructor feedback (source 40). textweaver checks its own output but has no checker for an input document. A `tw check FILE` that runs the loaders and reports scanned pages without a text layer, missing title and language, no headings or skipped levels, tables without a header row, images without alternative text, columns detected (so reading order was reconstructed rather than tagged), and the count of error-prone abbreviations, gives faculty an offline answer in seconds and the office a record. The Markdown lint (`tw lint`, [editing guide](../../editing.md)) is the pattern to follow: one line per problem, status 1 when there are problems, `--json` for scripts.

### A report for an accommodation file

An office needs to show what was provided and when. `tw marks --json` and `tw stats --json` already print a document's notes, highlights, places, and reading time ([notes guide](../../notes.md)). A `tw report` that combines the conversion report above with the file's checksums, the engine and models used, and the dates, with no reading data unless the student exports it, is small and fits how the office documents a request. Reading data belongs to the student; sync never sends recent files or document names for that reason ([sync guide](../../sync.md)).

### Bookshare and AccessText Network

Bookshare serves people with a qualifying print disability certified by a professional, free to US students through an OSEP award, in DAISY and BRF among other formats (source 37). textweaver opens DAISY 3 books and Bookshare zip files directly ([ADR-0026](../../adr/0026-ocr-and-student-formats.md)). The AccessText Network is the publishers' clearinghouse through which disability services offices request files on behalf of qualifying students; member offices report that most requests are filled within a day and nearly all within three days (sources 38). Those files are usually PDF, sometimes with a text layer and tags and sometimes not, which is exactly where the PDF loader and OCR earn their keep.

### Time per request today, and where a tool saves it

No formal survey of hours per request was found. Institutions publish their own turnaround times: scanning and processing from a few hours to several days per book, two days to two weeks when the office must cut and scan, five to ten business days, and up to four weeks when editing is involved (source 39). Reading those pages, the time goes to four steps: finding a publisher file (minutes to days, outside the tool), scanning (hours, outside the tool), recognition and structure repair (hours per book by hand), and delivery with instructions. textweaver removes most of the third step: OCR with layout recovery, running heads removed, columns in order, headings by size, and an audio or EPUB output with chapters, in seconds per chapter rather than hours. The pronunciation lexicon also moves work from the office to the student, who fixes a word once and keeps it. The honest estimate is that a tool like this cuts a scan-and-convert request from days to the same day, and leaves the publisher-file wait untouched.

## 7. Ranked improvements

Sizes: small is hours, medium a day or two, large a week or more. Every item works with a screen reader, a braille display, and the keyboard alone, and shows no state by color alone.

| # | Improvement | Users | Evidence or precedent | Crate or file | Size | Target | Performance note |
|---|---|---|---|---|---|---|---|
| 1 | Identifier guard: integers after CPT, PMID, NCT, ZIP, DOI, ISBN, phone labels read as digits | all | measured misreads in section 3 | `textweaver-speech/src/normalize/numbers.rs`, rule 11; `tests.rs` | small | alpha.8 | one lookbehind per number; no change to `first_speech` |
| 2 | Deny list for ISMP and Joint Commission abbreviations, spelled never expanded | nursing, pharmacy, medicine | ISMP list (sources 11, 12) | `normalize/abbreviations.rs` | small | alpha.8 | a hash set checked before the regex; nil |
| 3 | Clinical units and dosages: mg, mcg and µg, mL, mmol/L, "per", ranges with en dash as "to", 24-hour time without AM | all | section 3 table | `numbers.rs`, `punctuation.rs` | medium | alpha.8 | two more rules in the numbers chain; measure on the 10 MB corpus |
| 4 | Names for symbols outside math: Greek letters, µ, −, ⇌, →, ×10^n | biomedical research, pharmacology | section 3; `math.md` already names ×, ≤, ∞ outside formulas | `punctuation.rs` NAMES | small | alpha.8 | table lookup; nil |
| 5 | Medical lexicon tier with hash lookup, seeded from the Telnyx data, Tall Man words kept whole | all | CMUdict covers 45 percent of 240 terms; `community.rs` pattern | new `normalize/medical.rs`; `third_party/lexicon-medical/` | medium | alpha.9 | one hash lookup per word on the speech thread; 120 KB data file; no binary growth |
| 6 | Piper IPA override: splice lexicon phonemes into `prepare` with word owners kept | Piper users | `phonemes.rs` already builds the ids | `textweaver-piper/src/phonemes.rs` | medium | alpha.9 | per-clause string splice; measure Piper first speech at 450 wpm |
| 7 | `add pronunciation` palette command with spoken preview; `lexicon_files` for an office; import NVDA and JAWS dictionaries | students, offices | NVDA and JAWS dictionaries (sources 30, 31); pronunciations already sync | `textweaver-app`, `textweaver-cli` | medium | alpha.9 | preview runs through the speech thread; file reads on the writer thread |
| 8 | `\ce{}` chemistry speech and MathML output | biochemistry, pharmacology | mhchem (source 33); MathCAT reads chemistry (source 32) | `textweaver-math`, `textweaver-render` | medium | alpha.9 | parsed once per formula like any command; nil |
| 9 | `tw check FILE`: accessibility report for input documents | faculty, office | DOJ rule (source 41); Ally scores (source 40) | `textweaver-cli`, `textweaver-formats` | large | alpha.9 first cut, later full | reuses the loaders; runs in `tw`, never in the reader's input thread |
| 10 | PMID lookup in `tw cite add` and PubMed quick-open on the same call | all | E-utilities limits (source 43); roadmap | `textweaver-cite`, `textweaver-app/src/citations.rs` | medium | alpha.9 | one HTTPS call on a background thread, cached like DOI |
| 11 | Highlight categories by name, with filters in the list and study sheet | students | notes.md: yellow only | `textweaver-app/src/marks.rs`, `study.rs`, `textweaver-store` | medium | alpha.9 | one byte per highlight; nil |
| 12 | Reference superscripts in untagged PDFs read as "reference 12" and deferred with footnotes | research, medicine | `layout.rs` joins superscripts into the line | `textweaver-formats/src/pdf/layout.rs`, `structure.rs` | medium | alpha.9 | one pass over spans already measured; check `open` on the 300-page fixture |
| 13 | Conversion and request report (`tw convert --report`, `tw report`) | office | section 6 | `textweaver-convert`, `textweaver-cli` | small | alpha.9 | writes JSON after the batch; nil |
| 14 | Review list: documents by days since last read and `#review` notes | students | stats and Continue reading exist | `textweaver-app/src/study.rs`, `list_model.rs` | small | later | built from `stats.json` already in memory |
| 15 | Pocket export: highlights and notes as one short M4B plus a Markdown sheet | rotation students | audio export and study sheet exist | `textweaver-app/src/audio_export.rs`, `study.rs` | medium | later | same export path, a fraction of the document; runs on the export thread |

Item 16, a plain-text chemical formula reader behind a profile switch, follows item 8 and is small once the `\ce{}` tokens exist.

## Sources

1. https://support.vitalsource.com/hc/en-us/p/accessibility
2. https://success.vitalsource.com/hc/en-us/articles/360026396654
3. https://about.redshelf.com/accessibility
4. https://solve.redshelf.com/hc/en-us/articles/360015237493-Text-to-Speech-TTS
5. https://leap.ocls.ca/node/1605 (Northern Ontario School of Medicine University assessment of AccessMedicine; page not reachable from this machine, unverified)
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
36. https://par.nsf.gov/biblio/10063856 (Bragg and others, A Large Inclusive Study of Human Listening Rates, CHI 2018) and https://doi.org/10.1145/3461700 (the 2021 expansion)
37. https://www.bookshare.org/cms/node/60
38. https://ati.gmu.edu/accessible-text/accessible-textbooks/ and https://news.uga.edu/access-text-network-created-in-collaboration-with-uga-will-improve-student/
39. https://accessibility.ucdavis.edu/textbooks , https://www.uh.edu/accessibility/accommodations/alternate-format-textbooks/ , https://www.towson.edu/accessibility-disability-services/accommodations-services/print-materials.html , https://ds.oregonstate.edu/book/das-student-handbook/chapter-7-accommodation-spelling
40. https://help.blackboard.com/Learn/Administrator/Hosting/Tools_Management/Ally
41. https://doit.illinois.gov/doit-news/accessibility-news/doj-finalizes-rule-on-web-and-mobile-accessibility.html
42. https://www.w3.org/TR/pronunciation-use-cases/
43. https://www.ncbi.nlm.nih.gov/books/NBK25497/ (E-utilities)
44. https://www.mheducation.com/unitas/highered/accessibility/mckinley-human-anatomy-6e.pdf (an example of McGraw Hill's per-title accessibility report)
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
