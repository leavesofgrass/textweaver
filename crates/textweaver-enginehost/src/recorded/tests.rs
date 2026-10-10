use textweaver_speech::RawEvent;

use super::*;

/// A ramp: sample `i` is `i % 20_000`, so any sample says where it came
/// from.
fn ramp(n: usize) -> Vec<i16> {
    (0..n)
        .map(|i| i16::try_from(i % 20_000).unwrap_or(0))
        .collect()
}

fn wav_file(dir: &Path, name: &str, samples: &[i16], rate: u32) -> PathBuf {
    let path = dir.join(name);
    crate::wav::write(&path, samples, rate).unwrap();
    path
}

fn reader() -> ReadFile {
    Arc::new(|p: &Path| std::fs::read(p))
}

/// Everything `source` gives for one clip from `begin` to `end` samples.
fn clip(source: &mut Source, begin: u64, end: u64) -> Vec<i16> {
    let mut from = source.prepare(begin);
    let mut out = Vec::new();
    while let Some(chunk) = source.take(from, Some(end), 333) {
        out.extend(chunk);
        from = source.position();
    }
    out
}

#[test]
fn clip_times_become_whole_samples() {
    assert_eq!(samples_at(Duration::from_millis(55_105), 22_050), 1_215_065);
    assert_eq!(samples_at(Duration::from_secs(3600), 44_100), 158_760_000);
    assert_eq!(samples_at(Duration::ZERO, 8_000), 0);
}

#[test]
fn clips_are_trimmed_to_the_sample_without_drift() {
    let dir = tempfile::tempdir().unwrap();
    let all = ramp(24_000);
    let file = wav_file(dir.path(), "a.wav", &all, 8_000);
    let mut s = Source::open(&file, &reader()).unwrap();
    assert_eq!((s.rate, s.delay), (8_000, 0));
    // Phrases one after another: no seek, every sample once.
    assert_eq!(clip(&mut s, 800, 1_600), all[800..1_600]);
    assert_eq!(clip(&mut s, 1_600, 2_800), all[1_600..2_800]);
    // A small overlap plays on from where the audio is.
    assert_eq!(s.prepare(2_400), 2_800);
    assert_eq!(clip(&mut s, 2_400, 3_000), all[2_800..3_000]);
    // A gap is decoded past; a jump back seeks.
    assert_eq!(clip(&mut s, 5_000, 5_100), all[5_000..5_100]);
    assert_eq!(clip(&mut s, 100, 150), all[100..150]);
    // The end of the file ends a clip with no end.
    let mut from = s.prepare(23_990);
    let mut tail = Vec::new();
    while let Some(c) = s.take(from, None, 4) {
        tail.extend(c);
        from = s.position();
    }
    assert_eq!(tail, all[23_990..]);
}

#[test]
fn resampling_keeps_the_length_across_chunks() {
    let input = ramp(10_000);
    let mut whole = Resample::new(8_000, 22_050);
    let one = whole.push(&input);
    let mut parts = Resample::new(8_000, 22_050);
    let mut many = Vec::new();
    for chunk in input.chunks(777) {
        many.extend(parts.push(chunk));
    }
    assert_eq!(one, many);
    // Output j sits at input j * 8000 / 22050: the last whole one is 9999.
    assert_eq!(one.len(), 9_999 * 22_050 / 8_000 + 1);
    assert_eq!(one[0], 0);
    assert_eq!(one[22_050 / 2], 4_000);
}

#[test]
fn utterances_play_their_clips_in_order_with_their_words() {
    let dir = tempfile::tempdir().unwrap();
    let a = wav_file(dir.path(), "a.wav", &ramp(16_000), 8_000);
    // Another rate: resampled to the first file's.
    let b = wav_file(dir.path(), "b.wav", &ramp(16_000), 16_000);
    let mut p = BookPlayer::new(AudioOutput::Null { speed: 200.0 }, reader());
    let clip = |file: &PathBuf, from: u64, to: u64| RecordedClip {
        file: file.clone(),
        begin: Duration::from_millis(from),
        end: Some(Duration::from_millis(to)),
    };
    let one = UtteranceId {
        generation: 1,
        chunk: 0,
    };
    let two = UtteranceId {
        generation: 1,
        chunk: 1,
    };
    p.play(
        one,
        "Hello there world",
        &[clip(&a, 0, 500), clip(&a, 500, 1_250)],
    );
    p.play(two, "Next", &[clip(&b, 100, 600)]);
    let mut events = Vec::new();
    struct Sink<'a>(&'a mut Vec<(UtteranceId, RawEvent)>);
    impl EventSink for Sink<'_> {
        fn emit(&mut self, id: UtteranceId, event: RawEvent) {
            self.0.push((id, event));
        }
        fn is_current(&self, _: UtteranceId) -> bool {
            true
        }
    }
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    while !events.contains(&(two, RawEvent::Finished)) {
        assert!(std::time::Instant::now() < deadline, "{events:?}");
        p.poll(&mut Sink(&mut events));
        std::thread::sleep(Duration::from_millis(2));
    }
    // 1.25 s at 8 kHz, then 0.5 s of 16 kHz audio at 8 kHz: every sample
    // is accounted for, so nothing drifts.
    assert_eq!(p.playback().feed().pushed(), 10_000 + 4_000);
    let said: Vec<_> = events
        .iter()
        .filter_map(|(id, e)| match e {
            RawEvent::Word { byte_range, .. } => Some((id.chunk, byte_range.clone())),
            RawEvent::Started => Some((id.chunk, 100..100)),
            RawEvent::Finished => Some((id.chunk, 200..200)),
            _ => None,
        })
        .collect();
    assert_eq!(
        said,
        [
            (0, 100..100),
            (0, 0..5),
            (0, 6..11),
            (0, 12..17),
            (0, 200..200),
            (1, 100..100),
            (1, 0..4),
            (1, 200..200),
        ]
    );
}

#[test]
fn a_missing_file_ends_its_utterance_with_an_error() {
    let mut p = BookPlayer::new(AudioOutput::Null { speed: 200.0 }, reader());
    let id = UtteranceId::default();
    let gone = RecordedClip {
        file: PathBuf::from("no-such-file.wav"),
        begin: Duration::ZERO,
        end: None,
    };
    p.play(id, "Gone", &[gone]);
    let mut events = Vec::new();
    struct Sink<'a>(&'a mut Vec<RawEvent>);
    impl EventSink for Sink<'_> {
        fn emit(&mut self, _: UtteranceId, event: RawEvent) {
            self.0.push(event);
        }
        fn is_current(&self, _: UtteranceId) -> bool {
            true
        }
    }
    for _ in 0..50 {
        p.poll(&mut Sink(&mut events));
        if events.contains(&RawEvent::Finished) {
            break;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(
        matches!(
            events.as_slice(),
            [RawEvent::Started, RawEvent::Error(_), RawEvent::Finished]
        ),
        "{events:?}"
    );
}
