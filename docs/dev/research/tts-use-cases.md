# Text-to-speech for students with disabilities: use cases and lessons

This report collects what the research says about text-to-speech for students with print disabilities, what the reading products of 2026 do, and how students and accommodation offices actually work with them. It ends with a ranked list of fifteen improvements for textweaver's next alphas, each sized and tied to a crate. It is for textweaver's maintainers and for anyone deciding what to build for alpha.8 and alpha.9. Written on Friday, October 2, 2026. Claims about outside products and papers carry a source; claims about textweaver name the file they come from. Several vendor sites could not be fetched directly from this session, so their prices come from secondary pages and are marked where that matters.

## 1. What the evidence says

### Comprehension, fluency, and fatigue

The strongest single result is a meta-analysis of 22 studies and 2,942 participants from grade 3 through postsecondary: text-to-speech and read-aloud tools raised reading comprehension for students with reading disabilities by a weighted effect size of g = 0.35, with a confidence interval of 0.14 to 0.56 [1]. That is a modest, real effect, and the authors say study design explains part of the spread.

Newer controlled studies refine it:

- Children with reading and language difficulties, ages 8 to 12, comprehended more with TTS than reading silently. Students with dyslexia alone gained more than students with combined reading and language impairment [2]. The same group's 2020 study found the same TTS gain in a repeated-measures design across six conditions [3].
- Swedish schoolchildren with reading disability read faster with TTS at every grade. Comprehension improved only in grades 3 to 5, not in grades 6 to 9, and symptoms of inattention did not change the rate gain [4].
- Dutch secondary students with dyslexia, tracked by eye movement, did not comprehend better with audio support. Audio support made reading take longer and pushed readers into reading the whole text instead of selectively, because a linear narration fights skimming [5].

The honest summary for adults: TTS mostly buys speed and endurance, not comprehension. That matches the adult brain-injury and aphasia work. Nine adults with severe TBI read significantly faster with TTS and no worse in comprehension, and over half preferred it [6]. A follow-up with ten adults found "reading and listening" gave a higher comprehension rate, meaning correct answers per minute, than reading alone [7]. Twenty people with aphasia, reading with their own chosen voice, rate, and highlight settings, took significantly less processing time with no loss of accuracy, and most anticipated using TTS for "lengthy and difficult materials" [8]. People with aphasia name reduced fatigue from single-modality reading as a benefit, and name learning the device as the biggest barrier [9].

For adults with dyslexia, the five-year follow-up of a Swedish intervention found audiobooks stayed useful through school, speech-to-text was mixed, and TTS was used mainly to decode text [10]. Only 47 percent of older students kept using the apps after a six-week trial, against 82 percent of younger ones [11]. Adoption, not effect size, is the bottleneck in postsecondary use.

There is no controlled trial of TTS for ADHD or autism specifically. The ADHD moderator analysis above is the best data [4]. Return-to-learn guidance for concussion lists audiobooks and reduced reading as standard temporary adjustments [12]. For cerebral palsy, SMA, and RSI the evidence base is about input, not reading: the need is hands-free page turning, keyboard-only control, and dictation, all of which textweaver already does.

### Synchronized highlighting

Highlighting the spoken word is the feature every product sells, and the evidence for it is weak. In the two Keelor studies, TTS with highlighting and TTS without highlighting produced the same comprehension, and a faster highlight rate did not change it either [2][3]. Among people with aphasia, single-word, sentence, and no highlighting produced the same comprehension, but most participants preferred some highlighting because it kept their eyes and ears together [13]. The aphasia book-club study found the same: participants liked adjustable rate, word or sentence highlighting, and font size, and all re-read some sections more than once [14]. Lesson: highlighting is a preference and attention aid, not a comprehension lever, so offering word, sentence, both, or none matters more than any one default. textweaver draws both a word highlight and an optional sentence band (`[highlight] sentence_color`, `docs/settings.md`), but cannot yet turn the word highlight off while keeping the sentence band.

### Speech rate

Comprehension of compressed speech falls slowly up to about 275 words per minute and faster after that, in the classic review by Foulke and Sticht, and students left to choose settle near 225 [15]. Adults with aphasia preferred and were most efficient at about 150 wpm, least liked 200 wpm, and spent longer reviewing after fast passages [16]. Adults with TBI read faster with TTS even though the TTS ran at ordinary oral rates [6].

At the other end, blind listeners trained on synthetic speech comprehend more than 16 syllables per second, against a normal ceiling near 6 [17]. Advanced blind users understood sentences 2.8 times faster than a screen reader's default and still got half the content, while novices managed 1.6 times [18]. textweaver's 50 to 900 wpm range and per-voice rate memory (`docs/speech.md`) cover both groups. Its four presets, skim 350, normal 265, study 200, slow, sit where the evidence says they should.

### Neural versus formant voices

Screen-reader users overwhelmingly keep Eloquence and eSpeak because formant speech stays intelligible at very high rates and has no concatenation glitches [19][20]. Neural voices are preferred by occasional listeners and by people with dyslexia: Greek students with dyslexia identified words and sentences better in natural than synthetic speech, though text comprehension did not differ [21]. For NVDA, the Sonata add-on brought Piper voices with streaming "fast variants" and a rate boost up to 5x after moving its core to Rust [22]. Piper itself runs at roughly 8 to 9 times real time on two ARM cores [23]. The lesson for textweaver is the one it already follows: keep Eloquence and DECtalk for speed, Piper for long listening, and let each voice keep its own rate.

### Fonts, spacing, bionic reading, RSVP

Be honest with users here:

- **Letter spacing** has the best evidence. Extra-large spacing made 94 Italian and French children with dyslexia read about 20 percent faster with half the errors, because dyslexia amplifies crowding [24]. WCAG 1.4.12 cites this work [25]. textweaver's spacing presets are the right aid to lead with.
- **Dyslexia fonts** do not help. OpenDyslexic gave no gain in rate or accuracy, and no student preferred it [26]. Dyslexie performed no better than Calibri; the spacing, not the letterforms, drives any effect [27]. Among ordinary fonts, Helvetica, Courier, Arial, and Verdana read best for adults with dyslexia, and italics hurt [28]. Lexend's supporting study is a 20-child trial by its creators, not independently replicated [29]. Atkinson Hyperlegible was designed with low-vision readers and is the defensible low-vision choice, but is unstudied for dyslexia [30]. textweaver should keep shipping these fonts and stop implying they treat dyslexia.
- **Bionic reading** does not work. Four independent studies since 2024, including two with eye tracking, found no gain in speed or comprehension, and stronger bolding slowed readers [31][32][33][34]. Readwise's 2,074-reader test found a 2.6 wpm loss, which is noise [35].
- **RSVP** raises measured speed but removes regressions and parafoveal preview, and inferential comprehension degrades as speed rises [36]. It suits readers with nystagmus or tracking problems, not readers who need to understand a pharmacology chapter.

### Health-science students

Student nurses with dyslexia report difficulty with assessment formats, with reading on placement, and with disclosure; adjustments named are extended time and assistive technology [37]. Medical schools accommodate dyslexia with extra time and TTS such as Kurzweil 3000 [38]. Drug names are hard to say even for clinicians, and mispronunciation in voice systems is a recognized safety risk [39]. General TTS engines guess pronunciation from spelling, and a community dictionary of about 960 drug and clinical terms exists for exactly that gap [40]. Chemical formulas and genetics alleles need subscripts, superscripts, and parentheses read in order, and MathML is the structure that lets a reader navigate inside them [41]. A cheminformatics tool that reads compound names aloud was built because no screen reader did it [42].

## 2. The product landscape in 2026

| Product | Who it is for | Does that textweaver does not | Does worse than textweaver | Price and platforms | Offline | What offices ask for |
|---|---|---|---|---|---|---|
| Kurzweil 3000 [43][44] | College disability offices; LD and dyslexia | Column notes, bubble notes, vocabulary study guides with definitions, KES image-plus-text files, a lockdown test mode, Automater batch OCR to KES, web license | Closed format, slow, cloud-tied, no screen-reader-grade keyboard model | About $500 a year per person; Windows, Mac, web, iPad | Partly | Test mode with tools disabled per IEP; batch OCR |
| Read&Write (Texthelp, now Everway) [45][46] | K-12 and college, broad literacy support | Screen masking, highlights collected into a summary document, vocabulary list with images, Audio Maker to MP3, works inside Google Docs, Word, and browsers | No document structure navigation, weak PDF reading order | $184 a year individual; browser extension, Windows, Mac | Partly | Collect highlights; simplify text |
| Voice Dream Reader [47][48] | Dyslexia, low vision, blind; iOS and Mac | Bookshare login, pronunciation dictionary with skip rules, skips PDF headers and footers and in-text citations, navigation by time interval, highlight export | Apple only, subscription since 2023, no braille output | About $80 a year iOS, $50 Mac, secondary sources, unverified | Yes | Bookshare and pronunciation fixes |
| Speechify [49] | Consumers, ADHD | OCR of phone photos, 4.5x speed with cloud voices, cross-device sync | Cloud dependent, no structure, no screen-reader design | $139 a year | Partly | Rarely asked; students bring it |
| NaturalReader [50] | Consumers, students | Pronunciation editor, dyslexia font, web OCR | Cloud voices for quality, no structure | $119 to $159 a year | Partly | Pronunciation editor |
| ClaroRead [51] | UK Access to Work, dyslexia | Toolbar that reads any application, ScreenRuler, ClaroCapture, word prediction, scanning | No document model; reads whatever is on screen | About $225 to $329 one-time, prices unverified; Windows, Mac | Yes | Speak-from-toolbar in any app |
| Microsoft Immersive Reader and Edge Read Aloud [52][53] | Everyone with Edge or Office; schools | Line focus, picture dictionary, parts of speech coloring, translation, Copilot summaries, dozens of neural voices | Few offline voices, no notes, no export, no braille | Free; Windows, Mac, web | Few voices | Free fallback on every machine |
| Apple Spoken Content and Books [54] | Mac and iOS users | System-wide speak screen, word and sentence highlight colors and underline, Books resumes PDFs | No navigation units, no study tools | Free; Apple only | Yes | Zero-install option |
| Dolphin EasyReader [55][56] | Blind, low vision, dyslexia | Direct download from Bookshare, RNIB, NLS BARD, NFB-NEWSLINE; DAISY audio; word spacing controls; switch and eye-gaze friendly layout in 2026.04 | No PDF layout reconstruction, no writing | Free app; premium sync; Windows, iOS, Android | Yes | Library logins |
| Bookshare Reader [57][58] | US students with print disabilities | Free; audio with highlighting, MathML, Alexa | Bookshare books only | Free; iOS, Android, web | Yes | It is what Bookshare students start with |
| Learning Ally Link [59] | Dyslexia; human narration | VOICEtext human narration synced to highlighted text, 80,000 titles | Only its own library | $135 a year; iOS, Android, Chrome, web | Yes | Human voice for literature |
| Capti Voice [60] | Schools; playlists | Playlists, Google Drive and Bookshare import, assessment | iOS app pulled; service degraded in 2025 | $300 a year institutional, unverified | No | Declining |
| Thorium Reader [61][62] | Libraries, EPUB readers, blind users | EPUB 3 media overlays playback, LCP DRM, passes every epubtest read-aloud test | TTS for EPUB and DAISY only, limited PDF | Free, open source; Windows, Mac, Linux | Yes | Library ebooks with DRM |
| calibre viewer [63] | Ebook collectors | Piper and system TTS with highlight inside a huge format converter | Not an accessibility product; no structure announcement | Free, open source | Yes | Rare |
| Kindle Assistive Reader [64][65] | Kindle readers with print disabilities | Word highlight and 0.5x to 3.5x speed inside purchased books | iOS, Android, Mac, Fire only, not Windows; Kindle library only | Free with Kindle | Yes | Course readings bought on Kindle |
| Adobe Acrobat Read Out Loud [66][67] | PDF users with no other tool | Reads tagged PDFs in tag order | Wrong order on untagged and multi-column PDFs, no word highlight, no navigation | Free | Yes | Last resort |
| Balabolka [68] | Windows power users | Any SAPI voice, batch to MP3, pronunciation correction, portable | No accessibility design, dated UI | Free; Windows | Yes | Batch audio |
| @Voice Aloud Reader [69] | Android | Reads shared text from any app, pronunciation rules synced | Android only | Free with ads; Android | Yes | Phone reading |
| Legere Reader [70] | Android students | Bookshare, Pocket, Instapaper; Google and Acapela voices | Android only | About $10 one-time | Yes | Android Bookshare client |
| NVDA and JAWS [71][72] | Blind users | Say All with skim reading, speech dictionaries, rate boost; JAWS Skim Reading reads first line or sentence of each paragraph | Not document readers; no highlight for sighted co-readers | Free; about $95 a year home | Yes | Already installed |
| Orca [73] | Linux blind users | Say All, structural navigation, pronunciation dictionary | Same | Free | Yes | Linux labs |
| Emacspeak [74][75] | Programmers who are blind | Audio formatting, voice changes for structure, rate per voice | Emacs only | Free | Yes | Niche |

### Feature gaps for textweaver

| Gap | Size | Release | Run-time cost |
|---|---|---|---|
| Skim mode: first sentence of each paragraph, as JAWS does | small | alpha.8 | Same plan, fewer utterances |
| Highlight unit choice: word, sentence, both, none | small | alpha.8 | None |
| Per-document and per-course pronunciation lists with import and export, plus a health-sciences starter list | medium | alpha.8 | One hash lookup per word, already paid |
| Conversion manifest with provenance and an accessibility report per file | medium | alpha.8 | Already on the batch worker |
| Vocabulary study sheet from difficult words and definitions | small | alpha.8 | Dictionary already open |
| Collect highlights by color into the study sheet | small | alpha.8 | None |
| Time left at current rate, and per-document rate | small | alpha.8 | One division |
| Graphic without description: say so, and attach a figure note | medium | alpha.9 | None until the note is typed |
| Chemistry and unit reading rules | medium | alpha.9 | Regex pass per utterance, microseconds |
| Reading-order override for PDF pages | medium | alpha.9 | Re-run layout for one page |
| Exam profile: tools locked, session logged | medium | alpha.9 | None |
| EPUB 3 with media overlays export | large | alpha.9 | Audio export cost, already off-thread |
| Playback of DAISY audio and EPUB media overlays | large | later | Audio decode thread |
| Bookshare download inside textweaver | large | later | Network, needs a key |
| Translation of a word or passage | large | later | Model download |

## 3. How students actually read

### A 900-page pharmacology textbook

The student gets the book one of three ways: a publisher PDF through AccessText Network, where over 60 percent of requests are filled within a day [76]; a DAISY or EPUB from Bookshare, which is free for qualified US students [77]; or a scan from the disability office, which can take 10 to 30 business days [78][79]. Then they read it for fourteen weeks, a chapter a week, at 250 to 350 wpm with word highlighting, and re-read drug tables at 150 to 200. What they need is chapter-level resume, a sense of how long the chapter will take, bookmarks at every drug class, highlights by color (mechanism, adverse effects, interactions), and a study sheet they can export before an exam. textweaver already has resume, bookmarks, highlights, and a heading-grouped study sheet (`docs/notes.md`). It lacks "time left", highlight collection by color, and a vocabulary sheet.

What breaks: drug names. "Metoprolol", "ondansetron", and "tiotropium" come out of every engine differently, and the student stops trusting the voice [39][40]. textweaver's community IBMTTS dictionaries help Eloquence, and `[normalization.pronunciations]` helps every engine (`docs/speech.md`, `crates/textweaver-speech/src/normalize/community.rs`). But the list is global, not per course, and there is no way to export it to a classmate or import one from the office.

### A PDF lab manual with tables and figures

Lab manuals are made in Word, exported to PDF, and rarely tagged. Tables lose their header rows, figures have no descriptions, and page numbers and running heads interrupt every page. textweaver's loader reconstructs columns, strips running heads and footers, finds captions in six languages, and reads tables with headers (`crates/textweaver-formats/src/pdf/layout.rs`, `structure.rs`). Untagged images are found but have no text to say; a tagged `/Alt` is read as a graphic (`crates/textweaver-formats/src/pdf/mod.rs`). The missing piece is telling the student that a figure is there and has no description, and letting them attach one, because the office or a lab partner usually can describe it in a sentence.

### Journal articles in two columns

Screen-reader users still name PDFs as a leading barrier, and employed blind readers are advised to keep two screen readers for the documents one cannot handle [85][86]. The failure modes are consistent: multi-column PDFs read across columns, equations are images, and footnotes land mid-sentence [80][81]. textweaver's band-and-column algorithm handles the common case, reads footnotes inline, deferred, or skipped (`footnote_mode`), and skips bracketed citations by default (`docs/reading.md`). Reference lists still read in full. When the layout guess is wrong on one page, the student has no way to say "read this page in page order instead".

### Lecture slides

Slides are read with titles as headings and speaker notes after each slide (`crates/textweaver-formats/src/pptx.rs`). What students want next is to skip images, which textweaver does with `g`, and to read only the notes. A "read notes only" policy is a one-flag change in the narration policy.

### Exam materials

Offices need a reader that can be locked: no dictionary, no summaries, no notes, no network, and a log that says what was opened and when. Kurzweil sells exactly this [44]. textweaver's settings profiles (`docs/settings.md`) are the right place for an "exam" profile, with a lock flag the proctor sets and the student cannot clear from inside the program.

### Reading controls that matter

Every workflow above depends on five controls, all of which textweaver already has in some form: read from here (Enter or Ctrl+Space), skip to the next heading (`h`), sentence repeat (`;`), speed presets (F8), and pronunciation fixes (`docs/reading.md`, `docs/speech.md`). The gaps are skim reading, repeat-slower, and per-document rate.

### What accommodation offices need

Alternate-format production staff use OCR, then add headings, page numbers, lists, tables, and footnotes by hand [82]. They deliver EPUB, DOCX, PDF, and DAISY, because those are what Bookshare, Learning Ally, and publishers exchange [77][83]. They are asked, often months later, where a file came from, which version was converted, and whether the figures had descriptions. textweaver's batch conversion writes `conversion-report.txt` with failures and warnings and a `--json` result with per-file status and timing (`docs/converting.md`). A manifest with the source file's hash, page count, OCR engine and model version, the count of images without descriptions, and the heading tree would answer the office's questions without a second tool.

## 4. Fifteen improvements for the next alphas

Ranked by value to students, weighted toward health sciences. Measure every reading-path change with `cargo xtask bench --quick --baseline before.json --max-ratio 2` (`docs/dev/testing.md`); the relevant columns are named per item.

1. **Skim mode: read the first sentence of each paragraph.** Dyslexia, ADHD, TBI; precedent JAWS Skim Reading [72] and the finding that linear narration fights selective reading [5]. In `crates/textweaver-text/src/narrate.rs`, add a `NarrationPolicy` flag that keeps only the first utterance of each paragraph, with headings always kept; a key and palette entry in `crates/textweaver-app/src/playback.rs`. Small. Fewer utterances than a full plan; the "narration plan" bench must not regress.
2. **Highlight unit setting: word, sentence, both, none.** Aphasia, TBI, low vision; evidence that preference varies and comprehension does not [13][14]. A `[highlight] unit` setting read in `crates/textweaver-app/src/view.rs` and the GUI's document view, announced on change. Small. No run-time cost; the word map is still computed.
3. **Pronunciation lists per document and per folder, with import and export.** Nursing, medicine, pharmacy; precedent Voice Dream and NVDA dictionaries [47][71]. Load `pronunciations.toml` beside the document or in its folder on top of the global list in `crates/textweaver-speech/src/normalize/community.rs`, and add `tw settings pronunciations export`. Ship a health-sciences starter pack built from a permissive drug-name dictionary [40], checked by a pharmacist. Small to medium. A few hundred extra map entries; lookup is already per word. Load on the open worker, never on the input thread.
4. **Conversion manifest and per-file accessibility report.** Disability services staff; AccessText and Bookshare workflows [76][77]. Extend `crates/textweaver-app/src/batch.rs` and `crates/textweaver-convert/src/plan.rs` to write `conversion-manifest.json`: source path, SHA-256, size, pages, OCR engine and model hash, heading count, tables, images with and without descriptions, warnings, time. Medium. Hashing a 50 MB PDF costs about 100 ms on the batch worker; nothing on the input thread.
5. **Vocabulary study sheet.** Dyslexia, every health-science course; precedent Kurzweil vocabulary study guide and Read&Write vocabulary list [43][45]. Collect the marked difficult words of a chapter with their first definition from the lexicon (`crates/textweaver-lexicon/src/glossary.rs`, `crates/textweaver-app/src/study.rs`) into the study sheet. Small. Dictionary lookups already run off-thread.
6. **Collect highlights by color.** Precedent Read&Write "collect highlights" and Kurzweil extraction by highlighter [45][43]. Group the study sheet by highlight color or label as well as by heading in `crates/textweaver-app/src/marks.rs`. Small. None.
7. **Time left at the current rate, and per-document rate memory.** Long textbooks; aphasia readers who want 150 wpm for one book and 300 for another [16]. Words remaining divided by rate, said by Where am I (Shift+W); store the rate in the document state in `crates/textweaver-store/src/doc_state.rs`. Small. One division.
8. **Graphic without description: say it, let the reader describe it.** Low vision, blind, lab manuals [80]. In `crates/textweaver-formats/src/pdf/mod.rs`, mark images that have no `/Alt` as "graphic, no description"; in `crates/textweaver-app/src/notes.rs`, a "describe this figure" note that narration then reads as the description. The office can ship the notes file with the book through sync. Medium. None until a note exists.
9. **Chemistry and units in speech.** Chemistry, pharmacology, nursing drug calculations [41][37]. A rule set in `crates/textweaver-speech/src/normalize/` for formulas such as `H2O`, `Ca2+`, `Pb(NO3)2`, and units such as `mg/kg/day` and `mEq/L`, with Unicode subscripts and superscripts read in order and MathML chemistry passed to `textweaver-math`. Medium. One regex pass per utterance, microseconds, on the speech thread.
10. **Reading-order override per PDF page.** Journal articles, forms, posters [81]. A per-page setting, page order or single column or auto, in `crates/textweaver-formats/src/pdf/layout.rs`, chosen from the palette and kept in document state. Medium. Re-lays out one page, a few milliseconds, on the open worker.
11. **Repeat slower.** Aphasia, TBI, non-native readers [16]. A key that re-reads the current sentence at the "study" preset and returns to the previous rate, in `crates/textweaver-app/src/voice.rs` next to the preset cycle. Small. One extra utterance.
12. **Exam profile with a lock.** Accommodation offices; precedent Kurzweil test mode [44]. A profile that disables dictionary, summaries, notes export, dictation, and network, locked by a file the proctor owns, with the session written to the existing log (`crates/textweaver-app/src/logfile.rs`, `crates/textweaver-store/src/profiles.rs`). Medium. None.
13. **EPUB 3 export with media overlays.** Offices producing files for Thorium, EasyReader, and Bookshare Reader [61][55][84]. textweaver already writes EPUB 3 and audio with subtitles and chapters; join them into SMIL overlays in `crates/textweaver-export`. Large. Audio export is already off the input thread; file size grows by the audio.
14. **Deferred tables, as footnotes are deferred.** Pharmacology tables interrupt prose; `table_mode` already has structured, flat, and skip (`docs/speech.md`). Add `deferred` in `crates/textweaver-text/src/narrate.rs`. Small. None.
15. **Playback of DAISY audio and EPUB media overlays.** Learning Ally and human-narrated Bookshare titles [59][77]. `crates/textweaver-formats/src/daisy.rs` already parses SMIL; playing the referenced audio with the text highlight needs a decode thread and a new playback source in `crates/textweaver-speech`. Large. A decoder thread and buffer; keep it behind a feature to protect binary size.

Three items are deliberately absent. Bionic reading and dyslexia fonts should stay, since some readers like them, but their documentation should say the evidence is negative [26][31]. RSVP should warn that inferential comprehension drops with speed [36]. Translation and Bookshare download are on the roadmap already and need network and model decisions first.

## Sources

1. Wood, Moxley, Tighe, and Wagner, 2018, Journal of Learning Disabilities, meta-analysis: https://doi.org/10.1177/0022219416688170
2. Keelor et al., 2023, Annals of Dyslexia, TTS features and comprehension: https://doi.org/10.1007/s11881-023-00281-9
3. Keelor et al., 2020, Assistive Technology Outcomes and Benefits: https://www.atia.org/wp-content/uploads/2020/06/ATOB-V14-A2-Keelor_etal.pdf
4. Grunér, Östberg, and Hedenius, 2018, Journal of Special Education Technology: https://eric.ed.gov/?id=EJ1178864
5. Knoop-van Campen et al., 2021, Annals of Dyslexia, audio support and strategy: https://doi.org/10.1007/s11881-021-00246-w
6. Harvey, Hux, Scott, and Snell, 2013, Brain Injury: https://doi.org/10.3109/02699052.2013.823649
7. Harvey and Hux, 2015, Brain Injury: https://doi.org/10.3109/02699052.2015.1022878
8. Knollman-Porter et al., 2022, American Journal of Speech-Language Pathology: https://doi.org/10.1044/2021_AJSLP-21-00182
9. Hux et al., 2021, Journal of Communication Disorders: https://doi.org/10.1016/j.jcomdis.2021.106098
10. Almgren Bäck et al., 2023, five-year follow-up: https://doi.org/10.1080/17483107.2022.2161647
11. Nordström et al., 2018, teachers' perceptions: https://doi.org/10.1080/17483107.2018.1499142
12. Return to Learn after a Concussion, Lurie Children's and IESA guide: https://www.iesa.org/documents/general/iesa-lurie_rtl_guide.pdf
13. Effect of digital highlighting on comprehension with TTS for people with aphasia, 2021: https://pubmed.ncbi.nlm.nih.gov/33731970/
14. Wallace et al., 2023, aphasia book club with TTS: https://doi.org/10.1044/2023_AJSLP-23-00094
15. Foulke and Sticht, 1969, as summarized in Fulford, ERIC ED347988: https://files.eric.ed.gov/fulltext/ED347988.pdf
16. Hux et al., 2019, TTS rate and comprehension in aphasia: https://doi.org/10.1044/2019_AJSLP-19-00047
17. Hertrich, Dietrich, and Ackermann, 2013, ultra-fast speech in blind listeners: https://doi.org/10.3389/fpsyg.2013.00530
18. Guerreiro and Gonçalves, 2015, ASSETS, faster concurrent speech: https://researchportal.ulisboa.pt/en/publications/faster-text-to-speeches-enhancing-blind-peoples-information-scann/
19. Léonie Watson, Notes on synthetic speech: https://tink.uk/notes-on-synthetic-speech/
20. Fable, The voice behind screen readers: https://makeitfable.com/article/the-voice-behind-screen-readers/
21. Giannouli and Banou, 2019, synthetic versus natural speech in dyslexic students: https://doi.org/10.1080/17483107.2019.1629111
22. Sonata neural voices for NVDA: https://github.com/mush42/sonata-nvda
23. Reproducible CPU benchmarks for Piper: https://github.com/obole-ia/tts-cpu-benchmark
24. Zorzi et al., 2012, PNAS, extra-large letter spacing: https://doi.org/10.1073/pnas.1205566109
25. WCAG 2.1 Understanding 1.4.12 Text Spacing: https://w3c.github.io/wcag21/understanding/text-spacing.html
26. Wery and Diliberto, 2016, OpenDyslexic: https://doi.org/10.1007/s11881-016-0127-1
27. Kuster et al., 2018, Dyslexie font: https://link.springer.com/article/10.1007/s11881-017-0154-6
28. Rello and Baeza-Yates, 2013, Good fonts for dyslexia: https://www.changedyslexia.org/publications/pdfs/2013-ASSETS-Good%20Fonts%20for%20Dyslexia.pdf?v1.5.15=
29. Lexend project repository and research notes: https://github.com/googlefonts/lexend/
30. Braille Institute, Atkinson Hyperlegible: https://www.brailleinstitute.org/freefont/
31. Snell, 2024, Acta Psychologica, No, Bionic Reading does not work: https://doi.org/10.1016/j.actpsy.2024.104304
32. Spear et al., 2025, boldface letters and eye movements: https://doi.org/10.3758/s13414-025-03067-w
33. Beelders, 2025, Journal of Eye Movement Research: https://doi.org/10.3390/jemr18050049
34. Zhang, Zhao, and Wang, 2026, Bionic Reading in L2 academic contexts: https://doi.org/10.1016/j.actpsy.2026.107723
35. Readwise, Bionic Reading results from 2,074 readers: https://blog.readwise.io/bionic-reading-results/
36. RSVP: degradation of inferential comprehension as a function of speed: https://www.researchgate.net/publication/328925418_Rapid_serial_visual_presentation_Degradation_of_inferential_reading_comprehension_as_a_function_of_speed
37. Burton and Alexis, 2025, Journal of Advanced Nursing, student nurses with dyslexia: https://doi.org/10.1111/jan.16900
38. AMA Journal of Ethics, 2016, medical students with dyslexia: https://journalofethics.ama-assn.org/article/how-should-medical-schools-respond-students-dyslexia/2016-10
39. Patterson, 2018, Australian Prescriber, Unpronounceable drug names: https://doi.org/10.18773/austprescr.2018.057
40. Telnyx medical pronunciation dictionary: https://github.com/team-telnyx/medical-pronunciation-dictionary
41. University of Illinois CITL, accessible chemical equations: https://citl.illinois.edu/chemical-equations
42. Journal of Chemical Education, reading compound names aloud: https://pubs.acs.org/doi/10.1021/acs.jchemed.5b00217
43. Tech & Learning review of Kurzweil 3000, with pricing: https://www.techlearning.com/resources/kurzweil-3000
44. Kurzweil 3000 as a testing accommodation: https://www.kurzweiledu.com/files/Testing%20Datasheet.pdf
45. Everway, Read&Write for education: https://www.everway.com/products/read-and-write-education/
46. Texthelp Read&Write pricing: https://www.texthelp.com/products/read-and-write-education/premium-features/
47. Voice Dream Reader help: https://www.voicedream.com/support/reader-help/
48. Voice Dream Reader alternatives and pricing, 2026, secondary: https://www.yaps.ai/blog/voice-dream-reader-alternative
49. Speechify pricing, 2026, secondary: https://texttolab.com/blog/speechify-pricing
50. NaturalReader pricing, secondary: https://texttolab.com/blog/naturalreader-pricing
51. Georgia Tech assistive software catalog, ClaroRead entries: https://www.assistivesoftware.gatech.edu/reading-cld
52. Microsoft Edge Read aloud: https://www.microsoft.com/en-us/edge/features/read-aloud?form=MA13FJ
53. Microsoft support, Reading mode and Immersive Reader in Edge: https://support.microsoft.com/en-us/edge/use-immersive-reader-in-microsoft-edge
54. Apple, Change Read & Speak settings on Mac: https://support.apple.com/guide/mac-help/change-read-speak-settings-for-accessibility-spch638/mac
55. Dolphin EasyReader app features: https://yourdolphin.com/EasyReader-App/Features
56. Dolphin EasyReader for Windows, latest version: https://yourdolphin.com/product/version/major?id=8
57. Bookshare Reader: https://www.bookshare.org/bookshare-reader
58. DAISY Consortium, Bookshare Reader overview: https://daisy.org/guidance/info-help/guidance-training/reading-systems/bookshare-reader-ios-and-android-app-overview/
59. Learning Ally for college and adult learners: https://learningally.org/college-adults/
60. AppleVis, Capti Voice pulled from the App Store: https://www.applevis.com/forum/ios-ipados/capti-voice-has-been-pulled-app-store
61. EDRLab, Thorium Reader: https://www.edrlab.org/software/thorium-reader/
62. epubtest.org, Thorium Reader 3 on Windows: https://epubtest.org/results/3700
63. calibre manual, the E-book viewer: https://manual.calibre-ebook.com/viewer.html
64. Amazon, Assistive Reader for Kindle: https://www.amazon.com/b?ie=UTF8&node=122146457011
65. Good e-Reader, Kindle for iOS Assistive Reader: https://goodereader.com/blog/kindle/amazon-kindle-for-ios-has-a-new-assistive-reader-system
66. Adobe, Reading PDFs with reflow and accessibility features: https://helpx.adobe.com/acrobat/using/reading-pdfs-reflow-accessibility-features.html
67. Accessing Higher Ground, Read Out Loud in Adobe Reader: https://accessinghigherground.org/wp/wp-content/uploads/2019/04/Read-Aloud-Adobe-Reader.pdf
68. Balabolka: https://www.cross-plus-a.com/balabolka.htm
69. @Voice Aloud Reader on Google Play: https://play.google.com/store/apps/details?id=com.hyperionics.avar
70. Neil Squire Society, Legere Reader: https://www.neilsquire.ca/legere-reader/
71. NVDA user guide: https://download.nvaccess.org/documentation/userGuide.html
72. Freedom Scientific, JAWS Skim Reading: https://support.freedomscientific.com/SurfsUp/14-SkimReading.htm
73. GNOME Orca help: https://help.gnome.org/users/orca/stable/
74. Emacspeak: https://emacspeak.sourceforge.net/
75. Emacspeak manual, Reading: https://tvraman.github.io/emacspeak/manual/Reading.html
76. AccessText Network, How it works: https://accesstext.org/how-it-works
77. Bookshare for higher education students: https://www.bookshare.org/higher-ed-students
78. West Virginia University, alternative format turnaround: https://osa.wvu.edu/assistive-tech/alt-format
79. Valencia College, alternate format materials: https://valenciacollege.edu/students/office-for-students-with-disabilities/alternate-format.php
80. Indiana University Libraries, check reading order in research articles: https://guides.libraries.indiana.edu/accessible/pdforder
81. PDF Association, accessible math in PDF: https://pdfa.org/accessible-math-in-pdf-finally/
82. Stanford OAE, accessible format remediation: https://oae.stanford.edu/faculty-staff/accessible-format-remediation-alternate-format/accessible-format-remediation
83. AccessText Network wiki, Authorized User Reference Manual: https://accesstext.gatech.edu/wiki/Authorized_User_Reference_Manual
84. W3C, EPUB Media Overlays 3.2: https://www.w3.org/publishing/epub32/epub-mediaoverlays.html
85. WebAIM Screen Reader User Survey 10: https://webaim.org/projects/screenreadersurvey10/
86. McDonnall et al., 2025, screen reader use among employed people who are blind: https://www.blind.msstate.edu/sites/www.blind.msstate.edu/files/2025-07/McDonnall%20et%20al.%20(2025)%20Screen%20reader%20use.pdf

## See also

- [Research index](README.md)
- [Reading aids](../../reading-aids.md): what textweaver draws today, and the keys.
- [Star features not yet planned](../../star-gaps.md): the feature comparison this report extends.
- [Roadmap](../../roadmap.md): what is being built next.
