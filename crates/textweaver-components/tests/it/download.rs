//! The one downloader, through the fake fetcher.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use textweaver_components::fake::FakeFetcher;
use textweaver_components::{
    ComponentError, FileState, Progress, Sources, Status, Tenths, download,
};

use crate::common::{component, first_bytes, second_bytes, serving};

fn run(
    c: &textweaver_components::Component,
    dest: &std::path::Path,
    sources: &Sources,
    fetcher: &FakeFetcher,
    cancel: &AtomicBool,
) -> (
    Result<textweaver_components::Outcome, ComponentError>,
    Vec<u64>,
) {
    let mut tenths = Tenths::default();
    let mut said = Vec::new();
    let r = download(
        c,
        dest,
        sources,
        fetcher,
        &mut |p: Progress| said.extend(tenths.step(p)),
        cancel,
    );
    (r, said)
}

#[test]
fn a_download_checks_installs_and_says_each_tenth_once() {
    let tmp = tempfile::tempdir().unwrap();
    let c = component("model-a");
    let dest = c.dir_in(tmp.path());
    let fake = serving(&c);
    let (r, said) = run(
        &c,
        &dest,
        &Sources::public(),
        &fake,
        &AtomicBool::new(false),
    );
    let outcome = r.unwrap();
    assert_eq!(outcome.fetched, ["model.onnx", "tokenizer.json"]);
    assert!(outcome.kept.is_empty());
    // Tenths only, each once, rising to 100 (a piece of 1 KB of 5 KB
    // jumps a tenth, which is skipped rather than said late).
    assert_eq!(said, [20, 40, 60, 80, 90, 100]);
    assert!(said.windows(2).all(|w| w[0] < w[1] && w[1] % 10 == 0));
    assert_eq!(c.status_in(&dest), Status::Installed);
    assert!(
        c.verify_in(&dest)
            .iter()
            .all(|(_, s)| *s == FileState::Good)
    );
    assert_eq!(
        std::fs::read_to_string(dest.join("LICENSE.txt")).unwrap(),
        "CC0"
    );
    // Nothing temporary is left beside it.
    let names: Vec<String> = std::fs::read_dir(dest.parent().unwrap())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["model-a"]);
    assert_eq!(fake.request_count(), 2);
    // Again: everything checks out, so nothing is fetched.
    let (r, _) = run(
        &c,
        &dest,
        &Sources::public(),
        &fake,
        &AtomicBool::new(false),
    );
    assert_eq!(r.unwrap().kept.len(), 2);
    assert_eq!(fake.request_count(), 2);
}

#[test]
fn a_folder_placed_by_hand_is_adopted() {
    let tmp = tempfile::tempdir().unwrap();
    let c = component("model-b");
    let dest = c.dir_in(tmp.path());
    std::fs::create_dir_all(&dest).unwrap();
    std::fs::write(dest.join("model.onnx"), first_bytes()).unwrap();
    std::fs::write(dest.join("tokenizer.json"), second_bytes()).unwrap();
    let fake = FakeFetcher::new();
    let (r, _) = run(
        &c,
        &dest,
        &Sources::public(),
        &fake,
        &AtomicBool::new(false),
    );
    assert_eq!(r.unwrap().kept.len(), 2);
    assert_eq!(fake.request_count(), 0, "nothing downloaded again");
    assert!(dest.join("LICENSE.txt").is_file());
}

#[test]
fn a_bad_hash_is_refused_and_nothing_is_installed() {
    let tmp = tempfile::tempdir().unwrap();
    let c = component("model-c");
    let dest = c.dir_in(tmp.path());
    let mut fake = serving(&c);
    fake.bytes_mut(&c.files[1].url).unwrap()[3] ^= 1;
    let (r, _) = run(
        &c,
        &dest,
        &Sources::public(),
        &fake,
        &AtomicBool::new(false),
    );
    let e = r.unwrap_err();
    assert_eq!(
        e.to_string(),
        "tokenizer.json does not match its published hash"
    );
    assert!(!dest.exists(), "the folder holds only checked files");
    // The bad file is gone; the good one waits in the staging folder.
    let staging = dest.with_file_name("model-c.partial");
    assert!(!staging.join("tokenizer.json.part").exists());
    assert!(staging.join("model.onnx").is_file());
    // Fixed upstream: the next download fetches only the missing file.
    let fake = serving(&c);
    let (r, _) = run(
        &c,
        &dest,
        &Sources::public(),
        &fake,
        &AtomicBool::new(false),
    );
    r.unwrap();
    assert_eq!(fake.request_count(), 1);
    assert!(!staging.exists());
    assert_eq!(c.status_in(&dest), Status::Installed);
}

#[test]
fn a_cancelled_download_goes_on_from_its_part_file() {
    let tmp = tempfile::tempdir().unwrap();
    let c = component("model-d");
    let dest = c.dir_in(tmp.path());
    let cancel = Arc::new(AtomicBool::new(false));
    let fake = serving(&c).cancel_after(2048, cancel.clone());
    let (r, _) = run(&c, &dest, &Sources::public(), &fake, &cancel);
    assert!(matches!(r, Err(ComponentError::Cancelled)), "{r:?}");
    let part = dest
        .with_file_name("model-d.partial")
        .join("model.onnx.part");
    let kept = std::fs::metadata(&part).unwrap().len();
    assert!(kept >= 2048 && kept < 5000, "{kept}");
    assert!(!dest.exists());
    // Resumed: the request asks for the rest only.
    let fake = serving(&c);
    let (r, said) = run(
        &c,
        &dest,
        &Sources::public(),
        &fake,
        &AtomicBool::new(false),
    );
    r.unwrap();
    assert_eq!(fake.requests()[0].1, kept);
    assert_eq!(said.last(), Some(&100));
    assert!(
        c.verify_in(&dest)
            .iter()
            .all(|(_, s)| *s == FileState::Good)
    );
}

#[test]
fn a_server_without_ranges_starts_the_file_again() {
    let tmp = tempfile::tempdir().unwrap();
    let c = component("model-e");
    let dest = c.dir_in(tmp.path());
    let fake = serving(&c).fail_after(3000);
    let (r, _) = run(
        &c,
        &dest,
        &Sources::public(),
        &fake,
        &AtomicBool::new(false),
    );
    let e = r.unwrap_err();
    assert_eq!(e.to_string(), "model.onnx: the connection dropped");
    let mut fake = serving(&c);
    fake.whole_files_only = true;
    let (r, _) = run(
        &c,
        &dest,
        &Sources::public(),
        &fake,
        &AtomicBool::new(false),
    );
    r.unwrap();
    assert!(
        c.verify_in(&dest)
            .iter()
            .all(|(_, s)| *s == FileState::Good)
    );
}

#[test]
fn the_mirror_comes_first_and_the_public_address_is_the_fallback() {
    let tmp = tempfile::tempdir().unwrap();
    let c = component("model-f");
    let dest = c.dir_in(tmp.path());
    // The mirror has the model but not the tokenizer.
    let fake = FakeFetcher::new()
        .with("https://mirror.invalid/m/model-f/model.onnx", first_bytes())
        .with(c.files[1].url.to_string(), second_bytes());
    let sources = Sources::with_mirror("https://mirror.invalid/m/");
    let (r, _) = run(&c, &dest, &sources, &fake, &AtomicBool::new(false));
    r.unwrap();
    let asked: Vec<String> = fake.requests().into_iter().map(|r| r.0).collect();
    assert_eq!(
        asked,
        [
            "https://mirror.invalid/m/model-f/model.onnx",
            "https://mirror.invalid/m/model-f/tokenizer.json",
            c.files[1].url.as_ref(),
        ]
    );
}

#[test]
fn a_mirror_folder_on_this_computer_is_read_directly() {
    let tmp = tempfile::tempdir().unwrap();
    let c = component("model-g");
    let mirror = tmp.path().join("mirror");
    std::fs::create_dir_all(mirror.join("model-g")).unwrap();
    std::fs::write(mirror.join("model-g").join("model.onnx"), first_bytes()).unwrap();
    std::fs::write(
        mirror.join("model-g").join("tokenizer.json"),
        second_bytes(),
    )
    .unwrap();
    let dest = c.dir_in(&tmp.path().join("data"));
    let sources = Sources::with_mirror(&mirror.to_string_lossy());
    download(
        &c,
        &dest,
        &sources,
        &textweaver_components::StandardFetcher,
        &mut |_| {},
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(c.status_in(&dest), Status::Installed);
}

#[test]
fn too_much_or_too_little_is_a_size_error() {
    let tmp = tempfile::tempdir().unwrap();
    let c = component("model-h");
    let dest = c.dir_in(tmp.path());
    let mut fake = serving(&c);
    fake.bytes_mut(&c.files[1].url).unwrap().push(b'!');
    let (r, _) = run(
        &c,
        &dest,
        &Sources::public(),
        &fake,
        &AtomicBool::new(false),
    );
    assert!(matches!(r, Err(ComponentError::Size { .. })), "{r:?}");
    let mut fake = serving(&c);
    fake.bytes_mut(&c.files[1].url).unwrap().truncate(4);
    let (r, _) = run(
        &c,
        &dest,
        &Sources::public(),
        &fake,
        &AtomicBool::new(false),
    );
    assert_eq!(
        r.unwrap_err().to_string(),
        "tokenizer.json is 4 bytes, not 29"
    );
}

#[test]
fn no_address_and_no_mirror_says_so() {
    let tmp = tempfile::tempdir().unwrap();
    let mut c = component("model-i");
    for f in c.files.to_mut() {
        f.url = "".into();
    }
    let dest = c.dir_in(tmp.path());
    let (r, _) = run(
        &c,
        &dest,
        &Sources::public(),
        &FakeFetcher::new(),
        &AtomicBool::new(false),
    );
    assert!(matches!(r, Err(ComponentError::NoSource { .. })), "{r:?}");
}

#[test]
fn another_programs_fresh_lock_means_busy() {
    let tmp = tempfile::tempdir().unwrap();
    let c = component("model-j");
    let dest = c.dir_in(tmp.path());
    let staging = dest.with_file_name("model-j.partial");
    std::fs::create_dir_all(&staging).unwrap();
    std::fs::write(staging.join("download.lock"), b"").unwrap();
    let fake = serving(&c);
    let (r, _) = run(
        &c,
        &dest,
        &Sources::public(),
        &fake,
        &AtomicBool::new(false),
    );
    assert!(matches!(r, Err(ComponentError::Busy(_))), "{r:?}");
    assert_eq!(fake.request_count(), 0);
}
