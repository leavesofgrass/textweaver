use super::*;

fn write(dir: &Path, files: &[(&str, &str)]) {
    for (name, text) in files {
        std::fs::write(dir.join(name), text).unwrap();
    }
}

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

fn clips(p: &AudioPar) -> Vec<(String, Duration, Option<Duration>)> {
    p.clips
        .iter()
        .map(|c| {
            let name = c.file.file_name().unwrap().to_string_lossy().into_owned();
            (name, c.begin, c.end)
        })
        .collect()
}

#[test]
fn clip_times_read_every_smil_form() {
    assert_eq!(clock("npt=55.105s"), Some(ms(55_105)));
    assert_eq!(clock("0:00:03.123"), Some(ms(3_123)));
    assert_eq!(clock("1:02:03.5"), Some(ms(3_723_500)));
    assert_eq!(clock("01:02.25"), Some(ms(62_250)));
    assert_eq!(clock("250ms"), Some(ms(250)));
    assert_eq!(clock("1.5min"), Some(ms(90_000)));
    assert_eq!(clock("2h"), Some(ms(7_200_000)));
    assert_eq!(clock("12"), Some(ms(12_000)));
    assert_eq!(clock("0.0000015s"), Some(Duration::from_micros(1)));
    assert_eq!(clock("soon"), None);
    assert_eq!(clock("1:2:3:4"), None);
}

const NCC: &str = r#"<html><head><title>Talking Test</title>
<meta name="dc:title" content="Talking Test"/>
<meta name="ncc:multimediaType" content="audioFullText"/></head><body>
<h1 id="h1"><a href="a.smil#p1">Chapter One</a></h1>
<span class="page-normal" id="n1"><a href="a.smil#p3">1</a></span>
</body></html>"#;

const SMIL: &str = r#"<smil><body><seq>
<par id="p1"><text src="text.html#t1"/><audio src="one.wav" clip-begin="npt=0.000s" clip-end="npt=1.500s"/></par>
<par id="p2"><text src="text.html#t2"/><seq>
 <audio src="one.wav" clip-begin="npt=1.500s" clip-end="npt=2.000s"/>
 <audio src="one.wav" clip-begin="npt=2.000s" clip-end="npt=3.250s"/></seq></par>
<par id="p3"><text src="text.html#pg1"/><audio src="one.wav" clip-begin="npt=3.25s" clip-end="npt=3.75s"/></par>
<par id="p4"><text src="text.html#t3"/></par>
<par id="p5"><text src="text.html#t4"/><audio src="sub/two.wav" clip-begin="0:00:00.5"/></par>
</seq></body></smil>"#;

const TEXT: &str = r#"<html><body><h1 id="t1">Chapter One</h1>
<p id="t2">The first words.</p><span class="page-normal" id="pg1">1</span>
<p id="t3">No audio here.</p><p id="t4">Last words.</p></body></html>"#;

#[test]
fn phrases_follow_the_text_and_text_without_audio_has_none() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        &[("ncc.html", NCC), ("a.smil", SMIL), ("text.html", TEXT)],
    );
    let path = dir.path().join("ncc.html");
    let doc = crate::Registry::with_builtins()
        .load(&Source::Path(path.clone()), &LoadOptions::default())
        .unwrap();
    let text = doc.text().to_string();
    assert_eq!(
        text,
        "Chapter One\n\nThe first words.\n\nNo audio here.\n\nLast words."
    );
    let audio = book_audio(&path, &LoadOptions::default()).unwrap().unwrap();
    assert!(audio.has_text);
    let slices: Vec<String> = audio
        .pars
        .iter()
        .map(|p| doc.slice(p.range).to_string())
        .collect();
    // "No audio here." has no phrase: speech reads it. The page number's
    // audio follows the phrase before it.
    assert_eq!(
        slices,
        ["Chapter One\n\n", "The first words.\n\n", "Last words."]
    );
    assert_eq!(
        clips(&audio.pars[0]),
        [("one.wav".into(), ms(0), Some(ms(1_500)))]
    );
    assert_eq!(
        clips(&audio.pars[1]),
        [
            ("one.wav".into(), ms(1_500), Some(ms(2_000))),
            ("one.wav".into(), ms(2_000), Some(ms(3_250))),
            ("one.wav".into(), ms(3_250), Some(ms(3_750))),
        ]
    );
    assert_eq!(clips(&audio.pars[2]), [("two.wav".into(), ms(500), None)]);
    assert_eq!(audio.pars[2].clips[0].file, dir.path().join("sub/two.wav"));
}

#[test]
fn a_book_with_no_text_plays_by_its_headings() {
    let ncc = r#"<html><head><meta name="dc:title" content="Audio Only"/></head><body>
<h1 id="h1"><a href="b.smil#q1">Chapter One</a></h1>
<h2 id="h2"><a href="b.smil#q3">Part Two</a></h2></body></html>"#;
    let smil = r#"<smil><body><seq>
<par id="q1"><audio src="x.mp3" clip-begin="npt=0s" clip-end="npt=2s"/></par>
<par id="q2"><audio src="x.mp3" clip-begin="npt=2s" clip-end="npt=4s"/></par>
<par id="q3"><audio src="x.mp3" clip-begin="npt=4s" clip-end="npt=6s"/></par>
</seq></body></smil>"#;
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), &[("ncc.html", ncc), ("b.smil", smil)]);
    let path = dir.path().join("ncc.html");
    let audio = book_audio(&path, &LoadOptions::default()).unwrap().unwrap();
    assert!(!audio.has_text);
    let ranges: Vec<_> = audio.pars.iter().map(|p| p.range).collect();
    assert_eq!(ranges, [CharRange::new(0, 13), CharRange::new(13, 21)]);
    assert_eq!(audio.pars[0].clips.len(), 2);
    assert_eq!(
        clips(&audio.pars[1]),
        [("x.mp3".into(), ms(4_000), Some(ms(6_000)))]
    );
}

#[test]
fn a_daisy_3_book_with_only_audio_reads_its_ncx_headings() {
    let opf = r#"<package xmlns:dc="http://purl.org/dc/elements/1.1/"><metadata><dc:Title>Spoken</dc:Title></metadata><manifest>
<item id="ncx" href="nav.ncx" media-type="application/x-dtbncx+xml"/>
<item id="s1" href="s.smil" media-type="application/smil"/></manifest>
<spine><itemref idref="s1"/></spine></package>"#;
    let ncx = r#"<ncx><navMap>
<navPoint id="n1"><navLabel><text>Opening</text></navLabel><content src="s.smil#a1"/>
 <navPoint id="n2"><navLabel><text>Inside</text></navLabel><content src="s.smil#a2"/></navPoint>
</navPoint></navMap></ncx>"#;
    let smil = r#"<smil><body><seq>
<audio id="a1" src="t.mp3" clipBegin="0:00:00.000" clipEnd="0:00:05.000"/>
<par id="a2"><audio src="t.mp3" clipBegin="0:00:05.000" clipEnd="0:00:09.500"/></par>
</seq></body></smil>"#;
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        &[("book.opf", opf), ("nav.ncx", ncx), ("s.smil", smil)],
    );
    let path = dir.path().join("book.opf");
    let doc = crate::Registry::with_builtins()
        .load(&Source::Path(path.clone()), &LoadOptions::default())
        .unwrap();
    assert_eq!(doc.text().to_string(), "Opening\n\nInside");
    assert_eq!(crate::warnings(&doc.meta), [crate::daisy2::NO_TEXT_WARNING]);
    let audio = book_audio(&path, &LoadOptions::default()).unwrap().unwrap();
    assert!(!audio.has_text);
    assert_eq!(audio.pars.len(), 2);
    assert_eq!(audio.pars[1].range, CharRange::new(9, 15));
    assert_eq!(audio.pars[1].clips[0].end, Some(ms(9_500)));
}

#[test]
fn other_files_have_no_book_audio() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), &[("plain.txt", "Hello.")]);
    let none = book_audio(&dir.path().join("plain.txt"), &LoadOptions::default()).unwrap();
    assert_eq!(none, None);
}
