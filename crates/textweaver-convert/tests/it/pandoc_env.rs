//! The one Pandoc path: `tw convert` runs the program `TEXTWEAVER_PANDOC`
//! names through the formats loader, and stops it at the timeout.
//!
//! The environment is set on a child run of this test binary (the ignored
//! `child` test), so no test changes its own process's environment.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use textweaver_convert::{ConvertOptions, Converter, OutputFormat, Status};

/// A stand-in for Pandoc: answers `--version`, prints fixed HTML, or hangs
/// when `FAKE_PANDOC_HANG` is set.
fn fake_pandoc(dir: &Path) -> PathBuf {
    if cfg!(windows) {
        let p = dir.join("fake-pandoc.cmd");
        fs::write(
            &p,
            "@echo off\r\n\
             if \"%1\"==\"--version\" exit /b 0\r\n\
             if defined FAKE_PANDOC_HANG ping -n 60 127.0.0.1 >nul\r\n\
             echo ^<html^>^<head^>^<title^>Fake^</title^>^</head^>^<body^>^<h1^>From the fake^</h1^>^<p^>Some ^<em^>text^</em^>.^</p^>^</body^>^</html^>\r\n",
        )
        .expect("write fake");
        p
    } else {
        let p = dir.join("fake-pandoc");
        fs::write(
            &p,
            "#!/bin/sh\n\
             [ \"$1\" = --version ] && exit 0\n\
             [ -n \"$FAKE_PANDOC_HANG\" ] && exec sleep 60\n\
             cat >/dev/null\n\
             echo '<html><head><title>Fake</title></head><body><h1>From the fake</h1><p>Some <em>text</em>.</p></body></html>'\n",
        )
        .expect("write fake");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).expect("chmod");
        }
        p
    }
}

/// Runs the `child` test in a new process with `TEXTWEAVER_PANDOC` set.
fn run_child(dir: &Path, hang: bool) {
    let exe = std::env::current_exe().expect("test binary");
    // The test's full name in the one test program: its module path
    // without the program's name (`pandoc_env::child`).
    let module = module_path!()
        .split_once("::")
        .map_or(module_path!(), |(_, m)| m);
    let name = format!("{module}::child");
    let mut cmd = Command::new(exe);
    cmd.args([name.as_str(), "--exact", "--ignored", "--nocapture"])
        .env("TEXTWEAVER_PANDOC", fake_pandoc(dir))
        .env("TW_PANDOC_ENV_DIR", dir)
        .env_remove("TEXTWEAVER_PANDOC_TIMEOUT")
        .env_remove("FAKE_PANDOC_HANG");
    if hang {
        cmd.env("FAKE_PANDOC_HANG", "1");
    }
    // Output to a file, not a pipe: on Windows the fake's own child (the
    // `ping` standing in for a hang) inherits the handles and would hold a
    // pipe open after the child test is done.
    let log = dir.join("child.log");
    let file = fs::File::create(&log).expect("log file");
    let err = file.try_clone().expect("log file");
    let status = cmd
        .stdout(file)
        .stderr(err)
        .status()
        .expect("run the child test");
    let text = fs::read_to_string(&log).unwrap_or_default();
    assert!(
        status.success() && text.contains("1 passed"),
        "child failed:\n{text}"
    );
}

#[test]
fn convert_runs_the_program_textweaver_pandoc_names() {
    let dir = tempfile::tempdir().expect("tempdir");
    run_child(dir.path(), false);
}

#[test]
fn convert_stops_a_hung_pandoc_at_the_timeout() {
    let dir = tempfile::tempdir().expect("tempdir");
    run_child(dir.path(), true);
}

#[test]
#[ignore = "run by the tests above, with TEXTWEAVER_PANDOC set"]
fn child() {
    let Some(dir) = std::env::var_os("TW_PANDOC_ENV_DIR").map(PathBuf::from) else {
        return;
    };
    let hang = std::env::var_os("FAKE_PANDOC_HANG").is_some();
    let src = dir.join("in");
    fs::create_dir_all(&src).expect("mkdir");
    fs::write(src.join("doc.rst"), "Title\n=====\n").expect("write");
    let out = dir.join("out");
    let conv = Converter::new(ConvertOptions {
        to: OutputFormat::Markdown,
        out_dir: Some(out.clone()),
        pandoc_timeout: Some(Duration::from_secs(if hang { 2 } else { 60 })),
        force: true,
        ..ConvertOptions::default()
    })
    .expect("converter");
    assert!(conv.source_extensions().contains(&"rst"));
    let started = Instant::now();
    let s = conv.run(std::slice::from_ref(&src)).expect("run");
    if !hang {
        assert_eq!(s.converted, 1, "{:?}", s.files);
        let md = fs::read_to_string(out.join("doc.md")).expect("output");
        assert!(md.contains("# From the fake"), "{md}");
        assert!(md.contains("*text*"), "{md}");
        return;
    }
    let took = started.elapsed();
    assert!(took < Duration::from_secs(40), "{took:?}");
    assert_eq!(s.failed, 1, "{:?}", s.files);
    match &s.files[0].status {
        Status::Failed(reason) => assert!(
            reason.contains("took longer than 2 seconds and was stopped"),
            "{reason}"
        ),
        other => panic!("{other:?}"),
    }
}
