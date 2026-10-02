//! Install from a file: a folder or a zip, checked against the pins.

use std::io::Write;

use textweaver_components::{ComponentError, Status, install_from};

use crate::common::{component, first_bytes, second_bytes};

fn zip_of(path: &std::path::Path, members: &[(&str, &[u8])]) {
    let f = std::fs::File::create(path).unwrap();
    let mut z = zip::ZipWriter::new(f);
    let opts = zip::write::SimpleFileOptions::default();
    for (name, bytes) in members {
        z.start_file(*name, opts).unwrap();
        z.write_all(bytes).unwrap();
    }
    z.finish().unwrap();
}

#[test]
fn a_folder_with_the_files_installs() {
    let tmp = tempfile::tempdir().unwrap();
    let c = component("off-a");
    let from = tmp.path().join("stick").join("onnx");
    std::fs::create_dir_all(&from).unwrap();
    std::fs::write(from.join("model.onnx"), first_bytes()).unwrap();
    std::fs::write(from.join("tokenizer.json"), second_bytes()).unwrap();
    std::fs::write(from.join("README.md"), b"hello").unwrap();
    let dest = c.dir_in(&tmp.path().join("data"));
    let report = install_from(&c, &tmp.path().join("stick"), &dest).unwrap();
    assert_eq!(report.outcome.fetched.len(), 2);
    assert_eq!(
        report.refused,
        [(
            "README.md".to_owned(),
            "not one of this component's files".to_owned()
        )]
    );
    assert_eq!(c.status_in(&dest), Status::Installed);
    assert!(!dest.join("README.md").exists());
}

#[test]
fn a_zip_installs_and_a_renamed_file_is_found_by_its_hash() {
    let tmp = tempfile::tempdir().unwrap();
    let c = component("off-b");
    let zip = tmp.path().join("download.zip");
    zip_of(
        &zip,
        &[
            ("off-b/inference.onnx", &first_bytes()),
            ("off-b/tokenizer.json", &second_bytes()),
        ],
    );
    let dest = c.dir_in(&tmp.path().join("data"));
    let report = install_from(&c, &zip, &dest).unwrap();
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    assert_eq!(c.status_in(&dest), Status::Installed);
}

#[test]
fn a_mismatched_file_is_refused_with_the_reason() {
    let tmp = tempfile::tempdir().unwrap();
    let c = component("off-c");
    let mut bad = second_bytes();
    bad[0] ^= 1;
    let zip = tmp.path().join("x.zip");
    zip_of(
        &zip,
        &[("model.onnx", &first_bytes()), ("tokenizer.json", &bad)],
    );
    let dest = c.dir_in(&tmp.path().join("data"));
    let e = install_from(&c, &zip, &dest).unwrap_err();
    assert_eq!(
        e.to_string(),
        "tokenizer.json does not match its published hash"
    );
    assert!(!dest.exists());
    // A missing file is named.
    let zip2 = tmp.path().join("y.zip");
    zip_of(&zip2, &[("model.onnx", &first_bytes())]);
    let e = install_from(&c, &zip2, &dest).unwrap_err();
    assert!(matches!(e, ComponentError::Missing { .. }), "{e:?}");
    assert!(e.to_string().starts_with("tokenizer.json is not in "));
}

#[test]
fn zip_members_with_odd_names_are_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let c = component("off-d");
    let zip = tmp.path().join("slip.zip");
    zip_of(
        &zip,
        &[
            ("../../evil.onnx", b"x"),
            ("/abs/path.bin", b"x"),
            ("dir/na me.bin", b"x"),
            ("model.onnx", &first_bytes()),
            ("tokenizer.json", &second_bytes()),
        ],
    );
    let dest = c.dir_in(&tmp.path().join("data"));
    let report = install_from(&c, &zip, &dest).unwrap();
    let reasons: Vec<&str> = report.refused.iter().map(|r| r.1.as_str()).collect();
    assert_eq!(
        reasons,
        [
            "has a parent step or an absolute path",
            "has a parent step or an absolute path",
            "not a plain file name",
        ]
    );
    assert!(!tmp.path().join("evil.onnx").exists());
    assert_eq!(c.status_in(&dest), Status::Installed);
}
