//! Voice tests and measurements for the Apple backends (macOS only).
//!
//! These run real synthesis, so they are skipped unless `TEXTWEAVER_APPLE=1`
//! (CI sets it on its macOS runners); on other platforms they are skipped
//! always. Nothing is played aloud: live `nsspeech` speech runs at volume 0,
//! `avspeech` plays into a silent output, and files go to a temporary
//! directory.
//!
//! The file has its own `main` (`harness = false`) because the threads
//! matter: every backend is created and driven on a separate "speech"
//! thread, as the speech service does. Phase 1 runs while the main thread is
//! blocked (so `nsspeech` is shown not to need the main run loop, and
//! `avspeech` to fail cleanly without it); phase 2 runs while the main
//! thread runs its run loop, which `avspeech`'s callbacks need.
//!
//! Run a subset with `cargo test -p textweaver-apple --test voices -- <filter>`.

fn main() {
    #[cfg(target_os = "macos")]
    macos::main();
    #[cfg(not(target_os = "macos"))]
    println!("Apple voice tests: skipped (macOS only)");
}

#[cfg(target_os = "macos")]
mod macos {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    use textweaver_apple::audio::{Pcm, read_aiff};
    use textweaver_apple::{AvSpeechBackend, DEFAULT_VOICE, NsSpeechBackend, Output};
    use textweaver_core::{CharPos, Rate, Utterance, UtteranceId, Volume};
    use textweaver_speech::{EventSink, RawEvent, SpeechBackend, VoiceParams};

    type Test = (&'static str, fn() -> Result<(), String>);

    const SENTENCE: &str = "Dr. Smith opened the library at 9:30 a.m. Café crème, naïve résumé.";
    const SENTENCE_WORDS: [&str; 12] = [
        "Dr", "Smith", "opened", "the", "library", "at", "9:30", "a.m", "Café", "crème", "naïve",
        "résumé",
    ];
    const PASSAGE: &str = "The library opened early on Monday, and the students came in from \
        the rain. They found their seats, set down their bags, and began to read the chapters \
        their teacher had assigned. Some read quietly, others listened with headphones, and a \
        few took careful notes in the margins of their books before the bell rang at noon.";
    const SAMANTHA: &str = "com.apple.voice.compact.en-US.Samantha";

    const PHASE1: &[Test] = &[
        (
            "nsspeech_lists_eloquence_voices_and_defaults_to_reed",
            ns_voices,
        ),
        (
            "nsspeech_speaks_reed_words_in_order_without_main_loop",
            ns_words,
        ),
        ("nsspeech_pause_holds_words_and_resume_finishes", ns_pause),
        (
            "nsspeech_stop_cancels_and_the_next_utterance_speaks",
            ns_stop,
        ),
        ("nsspeech_synthesizes_reed_to_wav_and_aiff", ns_file),
        ("nsspeech_latency_reed_and_samantha", ns_latency),
        ("nsspeech_rate_calibration", ns_calibration),
        (
            "avspeech_without_main_loop_reports_an_error",
            av_no_main_loop,
        ),
    ];

    const PHASE2: &[Test] = &[
        (
            "avspeech_lists_eloquence_voices_and_defaults_to_reed",
            av_voices,
        ),
        (
            "avspeech_speaks_reed_words_in_order_with_rising_offsets",
            av_words,
        ),
        (
            "avspeech_pause_holds_the_clock_and_resume_finishes",
            av_pause,
        ),
        (
            "avspeech_stop_cancels_and_the_next_utterance_speaks",
            av_stop,
        ),
        ("avspeech_synthesizes_reed_to_wav", av_file),
        ("avspeech_latency_and_offset_accuracy", av_accuracy),
        ("avspeech_rate_calibration", av_calibration),
    ];

    pub fn main() {
        if std::env::var("TEXTWEAVER_APPLE").as_deref() != Ok("1") {
            println!("Apple voice tests: skipped (set TEXTWEAVER_APPLE=1 to run them)");
            return;
        }
        let filter: Option<String> = std::env::args().skip(1).find(|a| !a.starts_with('-'));
        let pick = move |tests: &'static [Test]| -> Vec<Test> {
            tests
                .iter()
                .filter(|(name, _)| filter.as_deref().is_none_or(|f| name.contains(f)))
                .copied()
                .collect()
        };
        let (p1, p2) = (pick(PHASE1), pick(PHASE2));
        let total = p1.len() + p2.len();
        let os = std::process::Command::new("sw_vers")
            .arg("-productVersion")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default();
        println!("\nrunning {total} Apple voice tests on macOS {os}");

        // Phase 1: the main thread only waits.
        let started = Instant::now();
        start_watchdog();
        let mut failed = std::thread::Builder::new()
            .name("speech".into())
            .spawn(move || run(&p1))
            .and_then(|h| h.join().map_err(|_| std::io::Error::other("panicked")))
            .unwrap_or(1);

        // Phase 2: the main thread runs its run loop.
        let done = Arc::new(AtomicBool::new(false));
        let result = Arc::new(AtomicUsize::new(0));
        let (d, r) = (done.clone(), result.clone());
        let spawned = std::thread::Builder::new()
            .name("speech".into())
            .spawn(move || {
                r.store(run(&p2), Ordering::SeqCst);
                d.store(true, Ordering::SeqCst);
            });
        match spawned {
            Ok(handle) => {
                assert!(textweaver_apple::run_main_loop_until(
                    || done.load(Ordering::SeqCst)
                ));
                let _ = handle.join();
                failed += result.load(Ordering::SeqCst);
            }
            Err(_) => failed += 1,
        }
        println!(
            "\ntest result: {}. {} passed; {failed} failed; finished in {:.2}s\n",
            if failed == 0 { "ok" } else { "FAILED" },
            total - failed,
            started.elapsed().as_secs_f64()
        );
        if failed > 0 {
            std::process::exit(1);
        }
    }

    /// The test running now, for the watchdog.
    static CURRENT: std::sync::Mutex<(&str, Option<Instant>)> = std::sync::Mutex::new(("", None));

    /// Fails the whole run when one test takes longer than two minutes (a
    /// hang would otherwise hold the CI job until its timeout).
    fn start_watchdog() {
        let _ = std::thread::Builder::new()
            .name("watchdog".into())
            .spawn(|| {
                loop {
                    std::thread::sleep(Duration::from_secs(1));
                    let (name, since) = *CURRENT.lock().unwrap_or_else(|e| e.into_inner());
                    if since.is_some_and(|t| t.elapsed() > Duration::from_secs(120)) {
                        println!("test {name} ... FAILED: still running after 120 s");
                        std::process::exit(1);
                    }
                }
            });
    }

    fn run(tests: &[Test]) -> usize {
        let mut failed = 0;
        for (name, f) in tests {
            println!("test {name} ...");
            let t0 = Instant::now();
            *CURRENT.lock().unwrap_or_else(|e| e.into_inner()) = (name, Some(t0));
            let outcome = std::panic::catch_unwind(f);
            let secs = t0.elapsed().as_secs_f64();
            match outcome {
                Ok(Ok(())) => println!("test {name} ... ok ({secs:.2}s)"),
                Ok(Err(e)) => {
                    failed += 1;
                    println!("test {name} ... FAILED ({secs:.2}s): {e}");
                }
                Err(_) => {
                    failed += 1;
                    println!("test {name} ... FAILED ({secs:.2}s): panicked");
                }
            }
        }
        failed
    }

    fn ensure(cond: bool, msg: impl FnOnce() -> String) -> Result<(), String> {
        if cond { Ok(()) } else { Err(msg()) }
    }

    /// Records events; utterances of generations below `current` are stale.
    #[derive(Default)]
    struct Rec {
        current: u64,
        events: Vec<(UtteranceId, RawEvent, Instant)>,
    }

    impl EventSink for Rec {
        fn emit(&mut self, id: UtteranceId, event: RawEvent) {
            self.events.push((id, event, Instant::now()));
        }
        fn is_current(&self, id: UtteranceId) -> bool {
            id.generation >= self.current
        }
    }

    impl Rec {
        fn ended(&self, id: UtteranceId) -> bool {
            self.events
                .iter()
                .any(|(i, e, _)| *i == id && matches!(e, RawEvent::Finished | RawEvent::Cancelled))
        }
        fn count(&self, id: UtteranceId, pred: impl Fn(&RawEvent) -> bool) -> usize {
            self.events
                .iter()
                .filter(|(i, e, _)| *i == id && pred(e))
                .count()
        }
        fn words(&self, id: UtteranceId, text: &str) -> Vec<(String, Option<u32>, Instant)> {
            self.events
                .iter()
                .filter(|(i, _, _)| *i == id)
                .filter_map(|(_, e, t)| match e {
                    RawEvent::Word {
                        byte_range,
                        audio_ms,
                    } => Some((
                        text[byte_range.start as usize..byte_range.end as usize].to_string(),
                        *audio_ms,
                        *t,
                    )),
                    _ => None,
                })
                .collect()
        }
        fn errors(&self) -> Vec<String> {
            self.events
                .iter()
                .filter_map(|(_, e, _)| match e {
                    RawEvent::Error(m) => Some(m.clone()),
                    _ => None,
                })
                .collect()
        }
    }

    fn utt(text: &str, generation: u64, chunk: u32) -> Utterance {
        let mut u = Utterance::literal(text, CharPos(0));
        u.id = UtteranceId { generation, chunk };
        u
    }

    fn drive(
        b: &mut dyn SpeechBackend,
        rec: &mut Rec,
        timeout: Duration,
        until: impl Fn(&Rec) -> bool,
    ) -> Result<(), String> {
        let end = Instant::now() + timeout;
        while !until(rec) {
            if Instant::now() > end {
                return Err(format!(
                    "timed out after {timeout:?}; events: {:?}",
                    rec.events
                        .iter()
                        .map(|(i, e, _)| (i.chunk, e))
                        .collect::<Vec<_>>()
                ));
            }
            b.poll(rec);
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(())
    }

    /// Checks the event sequence of one utterance: Started first, words in
    /// order covering `expected`, Finished last, exactly once.
    fn check_sequence(
        rec: &Rec,
        id: UtteranceId,
        text: &str,
        expected: &[&str],
    ) -> Result<Vec<(String, Option<u32>, Instant)>, String> {
        let evs: Vec<&RawEvent> = rec
            .events
            .iter()
            .filter(|(i, _, _)| *i == id)
            .map(|(_, e, _)| e)
            .collect();
        ensure(evs.first() == Some(&&RawEvent::Started), || {
            format!("first event {:?}", evs.first())
        })?;
        ensure(evs.last() == Some(&&RawEvent::Finished), || {
            format!("last event {:?}", evs.last())
        })?;
        ensure(rec.count(id, |e| *e == RawEvent::Finished) == 1, || {
            "Finished more than once".into()
        })?;
        ensure(rec.errors().is_empty(), || {
            format!("errors {:?}", rec.errors())
        })?;
        let words = rec.words(id, text);
        let got: Vec<&str> = words.iter().map(|(w, _, _)| w.as_str()).collect();
        ensure(got == expected, || {
            format!("words {got:?}, expected {expected:?}")
        })?;
        Ok(words)
    }

    fn quiet() -> VoiceParams {
        VoiceParams {
            volume: Volume::new(0),
            ..VoiceParams::default()
        }
    }

    fn with_voice(voice: &str, volume: u8) -> VoiceParams {
        VoiceParams {
            voice: Some(voice.to_string()),
            volume: Volume::new(volume),
            ..VoiceParams::default()
        }
    }

    fn ms(d: Duration) -> f64 {
        d.as_secs_f64() * 1000.0
    }

    // ---- nsspeech ------------------------------------------------------

    fn ns_voices() -> Result<(), String> {
        let b = NsSpeechBackend::new().map_err(|e| e.to_string())?;
        ensure(b.voice() == Some(DEFAULT_VOICE), || {
            format!("default {:?}", b.voice())
        })?;
        let voices = b.voices().map_err(|e| e.to_string())?;
        let reed = voices.iter().find(|v| v.id == DEFAULT_VOICE);
        ensure(reed.is_some(), || "Reed not listed".into())?;
        let reed = reed.ok_or("Reed")?;
        println!(
            "  {} voices, {} Eloquence; Reed is {:?} {:?}",
            voices.len(),
            voices
                .iter()
                .filter(|v| v.name.starts_with("Eloquence "))
                .count(),
            reed.name,
            reed.languages
        );
        ensure(
            reed.name == "Eloquence Reed (English, United States)",
            || format!("name {:?}", reed.name),
        )?;
        ensure(voices[0].id.starts_with("com.apple.eloquence."), || {
            "Eloquence voices sort first".into()
        })?;
        ensure(textweaver_apple::normalizes_natively(b.voice()), || {
            "Reed normalizes natively".into()
        })
    }

    fn ns_words() -> Result<(), String> {
        let mut b = NsSpeechBackend::new().map_err(|e| e.to_string())?;
        b.set_params(&quiet()).map_err(|e| e.to_string())?;
        let mut rec = Rec::default();
        let (u1, u2) = (utt(SENTENCE, 1, 0), utt("Second sentence.", 1, 1));
        b.speak(&u1, &mut rec).map_err(|e| e.to_string())?;
        b.speak(&u2, &mut rec).map_err(|e| e.to_string())?;
        drive(&mut b, &mut rec, Duration::from_secs(30), |r| {
            r.ended(u2.id)
        })?;
        let words = check_sequence(&rec, u1.id, SENTENCE, &SENTENCE_WORDS)?;
        check_sequence(&rec, u2.id, "Second sentence.", &["Second", "sentence"])?;
        ensure(words.iter().all(|(_, a, _)| a.is_none()), || {
            "no audio_ms".into()
        })?;
        // The second utterance starts only after the first finished.
        let fin1 = rec
            .events
            .iter()
            .position(|(i, e, _)| *i == u1.id && *e == RawEvent::Finished);
        let st2 = rec
            .events
            .iter()
            .position(|(i, e, _)| *i == u2.id && *e == RawEvent::Started);
        ensure(fin1 < st2, || "utterances overlap".into())?;
        println!(
            "  first word after {:.0} ms; words at {:?} ms",
            b.first_word_latency().map_or(-1.0, ms),
            words
                .iter()
                .map(|(_, _, t)| ms(t.duration_since(words[0].2)).round())
                .collect::<Vec<_>>()
        );
        Ok(())
    }

    fn ns_pause() -> Result<(), String> {
        let mut b = NsSpeechBackend::new().map_err(|e| e.to_string())?;
        b.set_params(&quiet()).map_err(|e| e.to_string())?;
        let mut rec = Rec::default();
        let u = utt(PASSAGE, 1, 0);
        b.speak(&u, &mut rec).map_err(|e| e.to_string())?;
        drive(&mut b, &mut rec, Duration::from_secs(10), |r| {
            r.words(u.id, PASSAGE).len() >= 3
        })?;
        b.pause().map_err(|e| e.to_string())?;
        // Let the current word end, then expect silence.
        let settle = Instant::now() + Duration::from_millis(700);
        while Instant::now() < settle {
            b.poll(&mut rec);
            std::thread::sleep(Duration::from_millis(5));
        }
        let before = rec.words(u.id, PASSAGE).len();
        let hold = Instant::now() + Duration::from_millis(1500);
        while Instant::now() < hold {
            b.poll(&mut rec);
            std::thread::sleep(Duration::from_millis(5));
        }
        let after = rec.words(u.id, PASSAGE).len();
        ensure(before == after, || {
            format!("{before} words before, {after} after pause")
        })?;
        ensure(!rec.ended(u.id), || "ended while paused".into())?;
        b.resume().map_err(|e| e.to_string())?;
        drive(&mut b, &mut rec, Duration::from_secs(60), |r| r.ended(u.id))?;
        let words = rec.words(u.id, PASSAGE);
        let expected = PASSAGE.split_whitespace().count();
        println!(
            "  paused after {before} words; {} of {expected} words reported",
            words.len()
        );
        ensure(words.len() == expected, || format!("{} words", words.len()))?;
        ensure(rec.count(u.id, |e| *e == RawEvent::Finished) == 1, || {
            "not finished".into()
        })
    }

    fn ns_stop() -> Result<(), String> {
        let mut b = NsSpeechBackend::new().map_err(|e| e.to_string())?;
        b.set_params(&quiet()).map_err(|e| e.to_string())?;
        let mut rec = Rec::default();
        let (u1, u2) = (utt(PASSAGE, 1, 0), utt(SENTENCE, 1, 1));
        b.speak(&u1, &mut rec).map_err(|e| e.to_string())?;
        b.speak(&u2, &mut rec).map_err(|e| e.to_string())?;
        drive(&mut b, &mut rec, Duration::from_secs(10), |r| {
            !r.words(u1.id, PASSAGE).is_empty()
        })?;
        rec.current = 2;
        let t = Instant::now();
        b.stop();
        let stop_ms = ms(t.elapsed());
        let u3 = utt("After the stop.", 2, 0);
        b.speak(&u3, &mut rec).map_err(|e| e.to_string())?;
        drive(&mut b, &mut rec, Duration::from_secs(20), |r| {
            r.ended(u3.id)
        })?;
        for id in [u1.id, u2.id] {
            ensure(rec.count(id, |e| *e == RawEvent::Cancelled) == 1, || {
                format!("{id:?} not cancelled")
            })?;
            ensure(rec.count(id, |e| *e == RawEvent::Finished) == 0, || {
                format!("{id:?} finished")
            })?;
        }
        ensure(rec.count(u2.id, |e| *e == RawEvent::Started) == 0, || {
            "u2 started".into()
        })?;
        check_sequence(&rec, u3.id, "After the stop.", &["After", "the", "stop"])?;
        println!(
            "  stop took {stop_ms:.1} ms; restart first word {:.0} ms",
            b.first_word_latency().map_or(-1.0, ms)
        );
        Ok(())
    }

    fn ns_file() -> Result<(), String> {
        let mut b = NsSpeechBackend::new().map_err(|e| e.to_string())?;
        let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
        let wav = dir.path().join("reed.wav");
        let t = Instant::now();
        b.synthesize_to_file(SENTENCE, &wav)
            .map_err(|e| e.to_string())?;
        let took = ms(t.elapsed());
        let bytes = std::fs::read(&wav).map_err(|e| e.to_string())?;
        ensure(&bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE", || {
            "not WAV".into()
        })?;
        let rate = u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]);
        let secs = (bytes.len() - 44) as f64 / 2.0 / f64::from(rate);
        println!(
            "  WAV {} bytes, {rate} Hz, {secs:.2} s of audio in {took:.0} ms",
            bytes.len()
        );
        ensure(secs > 1.0 && secs < 15.0, || format!("{secs} s"))?;
        let aiff = dir.path().join("reed.aiff");
        b.synthesize_to_file(SENTENCE, &aiff)
            .map_err(|e| e.to_string())?;
        let pcm = read_aiff(&std::fs::read(&aiff).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        ensure(
            (pcm.duration_ms() as f64 / 1000.0 - secs).abs() < 0.05,
            || format!("AIFF {} ms", pcm.duration_ms()),
        )?;
        ensure(!dir.path().join("reed.wav.tmp.aiff").exists(), || {
            "temporary left".into()
        })
    }

    fn ns_first_word(b: &mut NsSpeechBackend, generation: u64) -> Result<f64, String> {
        let mut rec = Rec::default();
        let u = utt(SENTENCE, generation, 0);
        b.speak(&u, &mut rec).map_err(|e| e.to_string())?;
        drive(b, &mut rec, Duration::from_secs(30), |r| r.ended(u.id))?;
        b.first_word_latency()
            .map(ms)
            .ok_or_else(|| "no first word".into())
    }

    fn ns_latency() -> Result<(), String> {
        for voice in [DEFAULT_VOICE, SAMANTHA] {
            let t = Instant::now();
            let mut b = NsSpeechBackend::new().map_err(|e| e.to_string())?;
            b.set_params(&with_voice(voice, 0))
                .map_err(|e| e.to_string())?;
            let create = ms(t.elapsed());
            let cold = ns_first_word(&mut b, 1)?;
            let warm = ns_first_word(&mut b, 2)?;
            let warm2 = ns_first_word(&mut b, 3)?;
            println!(
                "  LATENCY nsspeech {voice}: create {create:.0} ms, first word cold {cold:.0} ms, warm {warm:.0} / {warm2:.0} ms"
            );
        }
        Ok(())
    }

    fn duration_of(pcm: &Pcm) -> f64 {
        pcm.frames() as f64 / f64::from(pcm.sample_rate.max(1))
    }

    fn ns_calibration() -> Result<(), String> {
        let words = PASSAGE.split_whitespace().count() as f64;
        let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
        let path = dir.path().join("rate.aiff");
        for (voice, rates) in [
            (
                DEFAULT_VOICE,
                &[
                    80u16, 100, 120, 140, 160, 175, 190, 200, 210, 220, 230, 240, 250, 265, 280,
                    300, 330, 360, 400, 450, 500, 550, 600, 650, 700, 800, 900,
                ][..],
            ),
            (
                SAMANTHA,
                &[
                    80u16, 100, 120, 140, 160, 175, 200, 225, 250, 265, 300, 350, 400, 450, 500,
                    550, 600, 700, 800, 900,
                ][..],
            ),
        ] {
            let mut points = Vec::new();
            for &r in rates {
                // The backend maps canonical wpm through its table; measure
                // the engine directly by asking for a raw engine rate.
                let mut b = NsSpeechBackend::new().map_err(|e| e.to_string())?;
                b.set_params(&with_voice(voice, 100))
                    .map_err(|e| e.to_string())?;
                b.set_raw_rate(Some(f64::from(r)));
                b.synthesize_to_file(PASSAGE, &path)
                    .map_err(|e| e.to_string())?;
                let pcm = read_aiff(&std::fs::read(&path).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
                let secs = duration_of(&pcm);
                let wpm = words / secs * 60.0;
                points.push((r, wpm));
                println!("  RATE nsspeech {voice} engine {r} -> {secs:.3} s, {wpm:.1} wpm");
            }
            println!(
                "  TABLE nsspeech {voice}: {}",
                points
                    .iter()
                    .map(|(r, w)| format!("({r}.0, {w:.0}.0)"))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        // Live check of the table at the default rate: time from the first
        // word to the end, silent.
        for voice in [DEFAULT_VOICE, SAMANTHA] {
            let mut b = NsSpeechBackend::new().map_err(|e| e.to_string())?;
            let params = VoiceParams {
                rate: Rate::Wpm(265),
                ..with_voice(voice, 0)
            };
            b.set_params(&params).map_err(|e| e.to_string())?;
            let mut rec = Rec::default();
            let u = utt(PASSAGE, 1, 0);
            b.speak(&u, &mut rec).map_err(|e| e.to_string())?;
            drive(&mut b, &mut rec, Duration::from_secs(90), |r| r.ended(u.id))?;
            let ws = rec.words(u.id, PASSAGE);
            let end = rec
                .events
                .iter()
                .find(|(i, e, _)| *i == u.id && *e == RawEvent::Finished)
                .map(|x| x.2);
            if let (Some(first), Some(end)) = (ws.first(), end) {
                let secs = end.duration_since(first.2).as_secs_f64();
                println!(
                    "  LIVE nsspeech {voice} at 265 wpm (effective {}): {secs:.2} s from first word to end, {:.0} wpm",
                    b.effective_wpm(),
                    words / secs * 60.0
                );
            }
        }
        Ok(())
    }

    fn av_no_main_loop() -> Result<(), String> {
        let mut b =
            AvSpeechBackend::new(Output::Silent { realtime: false }).map_err(|e| e.to_string())?;
        let mut rec = Rec::default();
        let u = utt(SENTENCE, 1, 0);
        b.speak(&u, &mut rec).map_err(|e| e.to_string())?;
        drive(&mut b, &mut rec, Duration::from_secs(10), |r| r.ended(u.id))?;
        let errors = rec.errors();
        println!("  error: {errors:?}");
        ensure(errors.iter().any(|e| e.contains("main thread")), || {
            format!("{errors:?}")
        })?;
        ensure(rec.count(u.id, |e| *e == RawEvent::Finished) == 1, || {
            "not finished".into()
        })
    }

    // ---- avspeech ------------------------------------------------------

    fn av_voices() -> Result<(), String> {
        let b =
            AvSpeechBackend::new(Output::Silent { realtime: false }).map_err(|e| e.to_string())?;
        ensure(b.voice() == Some(DEFAULT_VOICE), || {
            format!("default {:?}", b.voice())
        })?;
        let voices = b.voices().map_err(|e| e.to_string())?;
        let reed = voices
            .iter()
            .find(|v| v.id == DEFAULT_VOICE)
            .ok_or("Reed not listed")?;
        println!(
            "  {} voices; Reed is {:?} {:?} {:?}",
            voices.len(),
            reed.name,
            reed.languages,
            reed.gender
        );
        ensure(
            reed.name == "Eloquence Reed (English, United States)",
            || format!("{:?}", reed.name),
        )
    }

    fn av_words() -> Result<(), String> {
        let mut b =
            AvSpeechBackend::new(Output::Silent { realtime: true }).map_err(|e| e.to_string())?;
        let mut rec = Rec::default();
        let (u1, u2) = (utt(SENTENCE, 1, 0), utt("Second sentence.", 1, 1));
        let t = Instant::now();
        b.speak(&u1, &mut rec).map_err(|e| e.to_string())?;
        b.speak(&u2, &mut rec).map_err(|e| e.to_string())?;
        drive(&mut b, &mut rec, Duration::from_secs(30), |r| {
            r.ended(u2.id)
        })?;
        let words = check_sequence(&rec, u1.id, SENTENCE, &SENTENCE_WORDS)?;
        check_sequence(&rec, u2.id, "Second sentence.", &["Second", "sentence"])?;
        println!("  synthesis end recognized by {:?}", b.last_synthesis_end());
        let offsets: Vec<u32> = words
            .iter()
            .map(|(_, a, _)| a.unwrap_or(u32::MAX))
            .collect();
        ensure(offsets.windows(2).all(|w| w[0] < w[1]), || {
            format!("offsets {offsets:?}")
        })?;
        ensure(offsets.iter().all(|&o| o < 20_000), || {
            format!("offsets {offsets:?}")
        })?;
        // Word events fire when the silent clock reaches them: arrival
        // follows the audio offsets.
        let started = rec
            .events
            .iter()
            .find(|(i, e, _)| *i == u1.id && *e == RawEvent::Started)
            .map(|x| x.2)
            .ok_or("no start")?;
        let lag: Vec<f64> = words
            .iter()
            .map(|(_, a, at)| ms(at.duration_since(started)) - f64::from(a.unwrap_or(0)))
            .collect();
        println!(
            "  offsets {offsets:?} ms; arrival minus offset {:?} ms; first audio {:.0} ms after speak, started {:.0} ms",
            lag.iter().map(|l| l.round()).collect::<Vec<_>>(),
            b.first_audio_latency().map_or(-1.0, ms),
            ms(started.duration_since(t))
        );
        ensure(lag.iter().all(|l| l.abs() <= 60.0), || {
            format!("lag {lag:?}")
        })
    }

    fn av_pause() -> Result<(), String> {
        let mut b =
            AvSpeechBackend::new(Output::Silent { realtime: true }).map_err(|e| e.to_string())?;
        let mut rec = Rec::default();
        let u = utt(PASSAGE, 1, 0);
        b.speak(&u, &mut rec).map_err(|e| e.to_string())?;
        drive(&mut b, &mut rec, Duration::from_secs(10), |r| {
            r.words(u.id, PASSAGE).len() >= 3
        })?;
        b.pause().map_err(|e| e.to_string())?;
        let before = rec.words(u.id, PASSAGE).len();
        let hold = Instant::now() + Duration::from_millis(1500);
        while Instant::now() < hold {
            b.poll(&mut rec);
            std::thread::sleep(Duration::from_millis(5));
        }
        let after = rec.words(u.id, PASSAGE).len();
        ensure(before == after, || {
            format!("{before} words before, {after} after pause")
        })?;
        b.resume().map_err(|e| e.to_string())?;
        drive(&mut b, &mut rec, Duration::from_secs(60), |r| r.ended(u.id))?;
        let n = rec.words(u.id, PASSAGE).len();
        let expected = PASSAGE.split_whitespace().count();
        println!("  paused after {before} words; {n} of {expected} words reported");
        ensure(n == expected, || format!("{n} words"))
    }

    fn av_stop() -> Result<(), String> {
        let mut b =
            AvSpeechBackend::new(Output::Silent { realtime: true }).map_err(|e| e.to_string())?;
        let mut rec = Rec::default();
        let (u1, u2) = (utt(PASSAGE, 1, 0), utt(SENTENCE, 1, 1));
        b.speak(&u1, &mut rec).map_err(|e| e.to_string())?;
        b.speak(&u2, &mut rec).map_err(|e| e.to_string())?;
        drive(&mut b, &mut rec, Duration::from_secs(10), |r| {
            !r.words(u1.id, PASSAGE).is_empty()
        })?;
        rec.current = 2;
        b.stop();
        let u3 = utt("After the stop.", 2, 0);
        b.speak(&u3, &mut rec).map_err(|e| e.to_string())?;
        drive(&mut b, &mut rec, Duration::from_secs(20), |r| {
            r.ended(u3.id)
        })?;
        for id in [u1.id, u2.id] {
            ensure(rec.count(id, |e| *e == RawEvent::Cancelled) == 1, || {
                format!("{id:?} not cancelled")
            })?;
            ensure(rec.count(id, |e| *e == RawEvent::Finished) == 0, || {
                format!("{id:?} finished")
            })?;
        }
        check_sequence(&rec, u3.id, "After the stop.", &["After", "the", "stop"]).map(|_| ())
    }

    fn av_file() -> Result<(), String> {
        let mut b =
            AvSpeechBackend::new(Output::Silent { realtime: false }).map_err(|e| e.to_string())?;
        let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
        let wav = dir.path().join("reed.wav");
        let t = Instant::now();
        b.synthesize_to_file(SENTENCE, &wav)
            .map_err(|e| e.to_string())?;
        let bytes = std::fs::read(&wav).map_err(|e| e.to_string())?;
        ensure(&bytes[0..4] == b"RIFF", || "not WAV".into())?;
        let rate = u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]);
        let secs = (bytes.len() - 44) as f64 / 2.0 / f64::from(rate);
        println!(
            "  WAV {} bytes, {rate} Hz, {secs:.2} s in {:.0} ms",
            bytes.len(),
            ms(t.elapsed())
        );
        ensure(secs > 2.0 && secs < 15.0, || format!("{secs} s"))
    }

    /// Ends of silent gaps (at least 40 ms below 2% of peak RMS), in ms.
    fn gap_ends(pcm: &Pcm) -> Vec<f64> {
        let frame = (pcm.sample_rate / 100).max(1) as usize; // 10 ms
        let rms: Vec<f32> = pcm
            .samples
            .chunks(frame)
            .map(|c| (c.iter().map(|s| s * s).sum::<f32>() / c.len() as f32).sqrt())
            .collect();
        let peak = rms.iter().copied().fold(0.0, f32::max);
        let mut out = Vec::new();
        let mut run = 0;
        for (i, r) in rms.iter().enumerate() {
            if *r < peak * 0.02 {
                run += 1;
            } else {
                if run >= 4 {
                    out.push(i as f64 * 10.0);
                }
                run = 0;
            }
        }
        out
    }

    fn av_accuracy() -> Result<(), String> {
        for voice in [DEFAULT_VOICE, SAMANTHA] {
            // Live first-word latency through the silent realtime output.
            let mut b = AvSpeechBackend::new(Output::Silent { realtime: true })
                .map_err(|e| e.to_string())?;
            b.set_params(&with_voice(voice, 100))
                .map_err(|e| e.to_string())?;
            let mut firsts = Vec::new();
            for g in 1..=3 {
                let mut rec = Rec::default();
                let u = utt(SENTENCE, g, 0);
                let t = Instant::now();
                b.speak(&u, &mut rec).map_err(|e| e.to_string())?;
                drive(&mut b, &mut rec, Duration::from_secs(30), |r| r.ended(u.id))?;
                let first = rec
                    .words(u.id, SENTENCE)
                    .first()
                    .map(|w| ms(w.2.duration_since(t)));
                firsts.push((
                    b.first_audio_latency().map_or(-1.0, ms),
                    first.unwrap_or(-1.0),
                ));
            }
            println!(
                "  LATENCY avspeech {voice}: (first buffer, first word event) ms: {:?}",
                firsts
                    .iter()
                    .map(|(a, w)| (a.round(), w.round()))
                    .collect::<Vec<_>>()
            );
            // Offset accuracy: word offsets against the ends of silent gaps.
            let s = b.synthesize_words(PASSAGE).map_err(|e| e.to_string())?;
            let offsets: Vec<f64> = s.words.iter().map(|(_, ms)| f64::from(*ms)).collect();
            let dur = duration_of(&s.pcm) * 1000.0;
            let gaps: Vec<f64> = gap_ends(&s.pcm)
                .into_iter()
                .filter(|g| *g < dur - 50.0 && *g > 50.0)
                .collect();
            // Signed: offset of the nearest word minus the gap end (negative:
            // the word offset lies inside the pause before the word).
            let signed: Vec<f64> = gaps
                .iter()
                .map(|g| {
                    offsets
                        .iter()
                        .map(|o| o - g)
                        .min_by(|a, b| a.abs().total_cmp(&b.abs()))
                        .unwrap_or(f64::NAN)
                })
                .collect();
            println!(
                "  SIGNED avspeech {voice}: nearest word offset minus gap end, ms: {:?}",
                signed.iter().map(|d| d.round()).collect::<Vec<_>>()
            );
            let mut dist: Vec<f64> = signed.iter().map(|d| d.abs()).collect();
            dist.sort_by(f64::total_cmp);
            let median = dist.get(dist.len() / 2).copied().unwrap_or(f64::NAN);
            let within = dist.iter().filter(|d| **d <= 30.0).count();
            let distinct = {
                let mut o = offsets.clone();
                o.dedup();
                o.len()
            };
            println!(
                "  ACCURACY avspeech {voice}: {} words, {distinct} distinct offsets, {} buffers ({:.1} ms each), audio {:.0} ms; {} silent gaps: median distance to nearest word offset {median:.0} ms, {within} within 30 ms, max {:.0} ms",
                s.words.len(),
                s.buffers,
                dur / s.buffers.max(1) as f64,
                dur,
                gaps.len(),
                dist.last().copied().unwrap_or(f64::NAN)
            );
            let expected = PASSAGE.split_whitespace().count();
            let enough = if voice == DEFAULT_VOICE {
                expected
            } else {
                expected - 3
            };
            ensure(s.words.len() >= enough, || {
                format!("{} words", s.words.len())
            })?;
            println!(
                "  END avspeech {voice}: synthesis end recognized by {:?}",
                s.end
            );
            ensure(offsets.windows(2).all(|w| w[0] <= w[1]), || {
                "offsets fall".into()
            })?;
            ensure(offsets.last().is_some_and(|l| *l < dur), || {
                "offset past the end".into()
            })?;
        }
        Ok(())
    }

    fn av_calibration() -> Result<(), String> {
        let words = PASSAGE.split_whitespace().count() as f64;
        for voice in [DEFAULT_VOICE, SAMANTHA] {
            let mut b = AvSpeechBackend::new(Output::Silent { realtime: false })
                .map_err(|e| e.to_string())?;
            b.set_params(&with_voice(voice, 100))
                .map_err(|e| e.to_string())?;
            let mut points = Vec::new();
            for step in 0..=20 {
                let r = step as f32 * 0.05;
                b.set_raw_rate(Some(r));
                let s = b.synthesize_words(PASSAGE).map_err(|e| e.to_string())?;
                let secs = duration_of(&s.pcm);
                let wpm = words / secs * 60.0;
                println!("  RATE avspeech {voice} engine {r:.2} -> {secs:.3} s, {wpm:.1} wpm");
                points.push((r, wpm));
            }
            println!(
                "  TABLE avspeech {voice}: {}",
                points
                    .iter()
                    .map(|(r, w)| format!("({r:.2}, {w:.0}.0)"))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        Ok(())
    }
}
