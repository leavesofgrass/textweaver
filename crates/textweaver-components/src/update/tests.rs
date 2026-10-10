use super::*;
use crate::fake::FakeFetcher;

const LIST: &str = "https://example.invalid/releases";

fn target() -> Target {
    Target {
        os: "windows",
        arch: "x86_64",
        kind: PackageKind::WindowsZip,
    }
}

/// A fake release server: a list with two newer releases (a stable one
/// and a newer pre-release) and an older one, each with this computer's
/// package and SHA256SUMS.txt.
fn server(package: &[u8]) -> FakeFetcher {
    let mut f = FakeFetcher::new();
    let mut releases = Vec::new();
    for (v, pre) in [
        ("0.1.0-alpha.8", true),
        ("0.2.0", false),
        ("0.3.0-beta.1", true),
    ] {
        let name = target().package_name(v);
        let pkg = format!("https://example.invalid/{v}/{name}");
        let sums = format!("https://example.invalid/{v}/SHA256SUMS.txt");
        f.insert(pkg.clone(), package.to_vec());
        f.insert(
            sums.clone(),
            format!(
                "{}  {name}\n{}  other.zip\n",
                sha256_of(package),
                "0".repeat(64)
            ),
        );
        releases.push(format!(
            r#"{{"tag_name":"v{v}","draft":false,"prerelease":{pre},"assets":[
              {{"name":"{name}","size":{},"browser_download_url":"{pkg}"}},
              {{"name":"SHA256SUMS.txt","size":10,"browser_download_url":"{sums}"}}]}}"#,
            package.len()
        ));
    }
    releases.push(r#"{"tag_name":"v9.9.9","draft":true,"assets":[]}"#.to_owned());
    f.insert(LIST, format!("[{}]", releases.join(",")));
    f
}

fn sha256_of(bytes: &[u8]) -> String {
    crate::sha256_hex(bytes)
}

#[test]
fn versions_order_as_semantic_versioning_does() {
    let v = |s| Version::parse(s).unwrap();
    assert!(v("0.1.0-alpha.10") > v("0.1.0-alpha.9"));
    assert!(v("0.1.0-beta.1") > v("0.1.0-alpha.9"));
    assert!(v("0.1.0") > v("0.1.0-beta.1"));
    assert!(v("v0.2.0") > v("0.1.9"));
    assert_eq!(v("1.0.0+build.5"), v("1.0.0"));
    assert!(Version::parse("1.0").is_none());
    assert!(Version::parse("1.0.0-").is_none());
    assert!(is_newer("0.2.0", "0.1.0"));
    assert!(is_newer("0.2.0", ""), "nothing declined yet");
    assert!(!is_newer("0.2.0", "0.2.0"));
}

#[test]
fn the_newest_newer_release_is_offered_pre_release_or_not() {
    let f = server(b"package");
    let offer = check(&f, LIST, "0.1.0-alpha.9", &target())
        .unwrap()
        .unwrap();
    assert_eq!(offer.version, "0.3.0-beta.1");
    assert!(offer.prerelease);
    assert_eq!(
        offer.package.name,
        "textweaver-0.3.0-beta.1-windows-x86_64.zip"
    );
    assert_eq!(offer.size_text(), "0 KB");
    // Already the newest: nothing.
    assert_eq!(check(&f, LIST, "0.3.0-beta.1", &target()).unwrap(), None);
    // Only the list was read: no package, no other request.
    assert!(f.requests().iter().all(|(a, _)| a == LIST));
}

#[test]
fn a_release_without_this_computers_package_says_so() {
    let f = server(b"package");
    let mac = Target {
        os: "linux",
        arch: "riscv64",
        kind: PackageKind::Tarball,
    };
    assert!(matches!(
        check(&f, LIST, "0.1.0", &mac),
        Err(UpdateError::NoPackage(v)) if v == "0.3.0-beta.1"
    ));
    assert!(matches!(
        check(&FakeFetcher::new(), LIST, "0.1.0", &target()),
        Err(UpdateError::Fetch(_))
    ));
}

#[test]
fn a_matching_package_is_downloaded() {
    let f = server(b"the real package");
    let offer = check(&f, LIST, "0.2.0", &target()).unwrap().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("0.3.0-beta.1");
    let mut said = Vec::new();
    let path = download_update(
        &offer,
        &f,
        &dir,
        &mut |p| said.push(p.percent()),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(std::fs::read(path).unwrap(), b"the real package");
    assert_eq!(said.last(), Some(&100));
}

#[test]
fn a_tampered_package_is_refused_and_nothing_is_kept() {
    let mut f = server(b"the real package");
    let offer = check(&f, LIST, "0.2.0", &target()).unwrap().unwrap();
    // Same size, different bytes: someone changed it.
    if let Some(b) = f.bytes_mut(&offer.package.address) {
        *b = b"the evil package".to_vec();
    }
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("u");
    let e = download_update(&offer, &f, &dir, &mut |_| {}, &AtomicBool::new(false)).unwrap_err();
    assert!(matches!(e, UpdateError::Mismatch(_)), "{e}");
    assert!(!dir.exists(), "nothing kept");
    let left: Vec<_> = std::fs::read_dir(tmp.path()).unwrap().flatten().collect();
    assert!(left.is_empty(), "{left:?}");
}

#[test]
fn a_package_missing_from_the_checksums_is_refused() {
    let f = server(b"pkg");
    let mut offer = check(&f, LIST, "0.2.0", &target()).unwrap().unwrap();
    offer.package.name = "renamed.zip".into();
    let tmp = tempfile::tempdir().unwrap();
    let e =
        download_update(&offer, &f, tmp.path(), &mut |_| {}, &AtomicBool::new(false)).unwrap_err();
    assert!(matches!(e, UpdateError::NoChecksum(_)), "{e}");
}

#[test]
fn checksum_lines_are_read_as_sha256sum_writes_them() {
    let h = "a".repeat(64);
    let sums = format!("{h}  x.zip\n{}  *y.zip\nnot a line\n", "B".repeat(64));
    assert_eq!(sum_for(&sums, "x.zip"), Some(h));
    assert_eq!(sum_for(&sums, "y.zip"), Some("b".repeat(64)));
    assert_eq!(sum_for(&sums, "z.zip"), None);
}

#[test]
fn package_names_follow_the_platform() {
    let v = "0.2.0";
    let t = |os, arch, kind| Target { os, arch, kind }.package_name(v);
    assert_eq!(
        t("linux", "aarch64", PackageKind::AppImage),
        "textweaver-0.2.0-linux-aarch64.AppImage"
    );
    assert_eq!(
        t("linux", "x86_64", PackageKind::Tarball),
        "textweaver-0.2.0-linux-x86_64.tar.gz"
    );
    assert_eq!(
        t("macos", "aarch64", PackageKind::MacZip),
        "textweaver-0.2.0-macos-universal.zip"
    );
}
