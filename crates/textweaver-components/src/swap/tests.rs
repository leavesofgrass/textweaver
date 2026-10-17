use super::*;
use std::io::Write;

fn zip_of(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut z = zip::ZipWriter::new(&mut buf);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        for (name, bytes) in files {
            z.start_file(*name, opts).unwrap();
            z.write_all(bytes).unwrap();
        }
        z.finish().unwrap();
    }
    buf.into_inner()
}

#[test]
fn member_paths_drop_the_top_folder_and_refuse_escapes() {
    assert_eq!(member_path(Path::new("pkg")).unwrap(), None);
    assert_eq!(
        member_path(Path::new("pkg/docs/a.md")).unwrap(),
        Some(PathBuf::from("docs").join("a.md"))
    );
    assert!(member_path(Path::new("pkg/../x")).is_err());
    assert!(member_path(Path::new("/etc/passwd")).is_err());
    assert!(member_path(Path::new("pkg/a b.txt")).is_err());
}

#[test]
fn a_zip_unpacks_without_its_top_folder() {
    let tmp = tempfile::tempdir().unwrap();
    let pkg = tmp.path().join("textweaver-9.0.0-windows-x86_64.zip");
    std::fs::write(
        &pkg,
        zip_of(&[("tw-9/tw.exe", b"new tw"), ("tw-9/docs/a.md", b"doc")]),
    )
    .unwrap();
    let out = unpack(&pkg, &tmp.path().join("new")).unwrap();
    assert_eq!(std::fs::read(out.join("tw.exe")).unwrap(), b"new tw");
    assert_eq!(
        std::fs::read(out.join("docs").join("a.md")).unwrap(),
        b"doc"
    );
    let evil = tmp.path().join("evil.zip");
    std::fs::write(&evil, zip_of(&[("p/../../x", b"x")])).unwrap();
    assert!(unpack(&evil, &tmp.path().join("evil")).is_err());
}

#[test]
fn a_tarball_unpacks_without_its_top_folder() {
    let tmp = tempfile::tempdir().unwrap();
    let pkg = tmp.path().join("p.tar.gz");
    {
        let gz = flate2::write::GzEncoder::new(
            std::fs::File::create(&pkg).unwrap(),
            flate2::Compression::fast(),
        );
        let mut t = tar::Builder::new(gz);
        let mut h = tar::Header::new_gnu();
        h.set_size(6);
        h.set_mode(0o755);
        h.set_cksum();
        t.append_data(&mut h, "pkg/tw", &b"new tw"[..]).unwrap();
        t.into_inner().unwrap().finish().unwrap();
    }
    let out = unpack(&pkg, &tmp.path().join("new")).unwrap();
    assert_eq!(std::fs::read(out.join("tw")).unwrap(), b"new tw");
}

fn names_with_update_suffix(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.contains(".update-"))
        .collect()
}

/// The Windows swap with a fake install folder: the package's entries are
/// replaced, new ones added, and everything else kept as it was.
#[test]
fn the_swap_replaces_only_the_package_files() {
    let tmp = tempfile::tempdir().unwrap();
    let install = tmp.path().join("textweaver");
    let home = install.join("home").join("config");
    std::fs::create_dir_all(install.join("docs")).unwrap();
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(install.join("NOTICE"), b"old notice").unwrap();
    std::fs::write(install.join("tw.exe"), b"old tw").unwrap();
    // Before beta 3: the app, and textweaver.exe a launcher for tw.
    std::fs::write(install.join("textweaver-gui.exe"), b"old gui").unwrap();
    std::fs::write(install.join("textweaver.exe"), b"old launcher").unwrap();
    std::fs::write(install.join("docs").join("old.md"), b"old doc").unwrap();
    std::fs::write(install.join("install-manifest.txt"), b"mine").unwrap();
    std::fs::write(home.join("settings.toml"), b"settings").unwrap();

    let new = tmp.path().join("new");
    std::fs::create_dir_all(new.join("docs")).unwrap();
    std::fs::write(new.join("NOTICE"), b"new notice").unwrap();
    std::fs::write(new.join("tw.exe"), b"new tw").unwrap();
    std::fs::write(new.join("textweaver.exe"), b"new textweaver").unwrap();
    std::fs::write(new.join("docs").join("new.md"), b"new doc").unwrap();

    // Canonical: macOS temp folders sit behind /private.
    assert_eq!(
        std::fs::canonicalize(install_dir(&install.join("tw.exe")).unwrap()).unwrap(),
        std::fs::canonicalize(&install).unwrap()
    );
    assert!(wait_until_closed(
        &install.join("tw.exe"),
        Duration::from_secs(5)
    ));
    assert_eq!(swap_in(&new, &install, false).unwrap(), 4);
    let read = |p: &str| std::fs::read(install.join(p)).unwrap();
    assert_eq!(read("tw.exe"), b"new tw");
    assert_eq!(read("textweaver.exe"), b"new textweaver");
    // The retired app is gone; the program that replaced it starts.
    assert!(!install.join("textweaver-gui.exe").exists());
    assert_eq!(
        current_program(&install.join("textweaver-gui.exe"), &install),
        install.join(PROGRAM)
    );
    assert_eq!(read("NOTICE"), b"new notice");
    assert_eq!(read("docs/new.md"), b"new doc");
    assert!(!install.join("docs").join("old.md").exists());
    // Not the package's: kept as they were.
    assert_eq!(read("install-manifest.txt"), b"mine");
    assert_eq!(
        std::fs::read(home.join("settings.toml")).unwrap(),
        b"settings"
    );
    assert!(names_with_update_suffix(&install).is_empty());
}

/// The shortcut script names only the install folder's programs, quoted.
#[test]
fn the_shortcut_script_points_at_the_new_programs() {
    let s = retarget_script(Path::new(r"C:\Users\Ada Example\textweaver's"));
    assert!(
        s.contains(r"$dir = 'C:\Users\Ada Example\textweaver''s';"),
        "{s}"
    );
    for want in [
        "'textweaver-gui.exe'",
        "'textweaver.exe'",
        "'tw.exe'",
        "-Filter 'textweaver*.lnk'",
        "GetFolderPath('Programs')",
    ] {
        assert!(s.contains(want), "{want} is missing: {s}");
    }
}

/// A program that is not retired starts again as it is.
#[test]
fn the_program_to_start_again() {
    let install = Path::new("I");
    for kept in ["textweaver.exe", "textweaver", "tw.exe", "tw"] {
        assert_eq!(
            current_program(&install.join(kept), install),
            install.join(kept)
        );
    }
    assert_eq!(
        current_program(&install.join("TEXTWEAVER-GUI.EXE"), install),
        install.join(PROGRAM)
    );
}

/// A link in a package is kept only when it points at a plain path in its
/// own folder.
#[test]
fn links_stay_inside_their_folder() {
    let raw = Path::new("pkg/tw");
    for ok in ["textweaver", "textweaver.app/Contents/MacOS/textweaver"] {
        assert_eq!(link_target(raw, Path::new(ok)).unwrap(), Path::new(ok));
    }
    for bad in [
        "",
        "/usr/bin/textweaver",
        "../textweaver",
        "a/../../b",
        "a b",
    ] {
        assert!(link_target(raw, Path::new(bad)).is_err(), "{bad}");
    }
}

/// The Linux tarball's `tw` link to `textweaver` unpacks as a link; one
/// pointing outside the package is refused.
#[test]
fn a_tarball_keeps_its_tw_link() {
    let tar_of = |target: &str| {
        let mut buf = Vec::new();
        {
            let gz = flate2::write::GzEncoder::new(&mut buf, flate2::Compression::fast());
            let mut t = tar::Builder::new(gz);
            let mut h = tar::Header::new_gnu();
            h.set_size(4);
            h.set_mode(0o755);
            h.set_cksum();
            t.append_data(&mut h, "pkg/textweaver", &b"prog"[..])
                .unwrap();
            let mut l = tar::Header::new_gnu();
            l.set_entry_type(tar::EntryType::Symlink);
            l.set_size(0);
            l.set_mode(0o777);
            t.append_link(&mut l, "pkg/tw", target).unwrap();
            t.into_inner().unwrap().finish().unwrap();
        }
        buf
    };
    let tmp = tempfile::tempdir().unwrap();
    let pkg = tmp.path().join("p.tar.gz");
    std::fs::write(&pkg, tar_of("textweaver")).unwrap();
    let got = unpack(&pkg, &tmp.path().join("new"));
    if cfg!(unix) {
        let out = got.unwrap();
        assert_eq!(
            std::fs::read_link(out.join("tw")).unwrap(),
            Path::new("textweaver")
        );
        // Swapped in, the link stays a link to the program beside it.
        let install = tmp.path().join("install");
        std::fs::create_dir_all(&install).unwrap();
        std::fs::write(install.join("tw"), b"old tw").unwrap();
        std::fs::write(install.join("textweaver-gui"), b"old gui").unwrap();
        swap_in(&out, &install, false).unwrap();
        assert_eq!(
            std::fs::read_link(install.join("tw")).unwrap(),
            Path::new("textweaver")
        );
        assert_eq!(std::fs::read(install.join("tw")).unwrap(), b"prog");
        assert!(!install.join("textweaver-gui").exists());
    } else {
        // Windows packages hold no links.
        assert!(got.is_err());
    }
    let evil = tmp.path().join("evil.tar.gz");
    std::fs::write(&evil, tar_of("../../etc/passwd")).unwrap();
    assert!(unpack(&evil, &tmp.path().join("evil")).is_err());
}

#[test]
fn a_failed_swap_changes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let install = tmp.path().join("textweaver");
    std::fs::create_dir_all(&install).unwrap();
    std::fs::write(install.join("tw.exe"), b"old tw").unwrap();
    // A package folder that is not there: nothing is touched.
    assert!(swap_in(&tmp.path().join("missing"), &install, false).is_err());
    assert_eq!(std::fs::read(install.join("tw.exe")).unwrap(), b"old tw");
    assert!(names_with_update_suffix(&install).is_empty());
}

#[test]
fn a_folder_without_the_notice_is_not_a_package() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("tw.exe"), b"x").unwrap();
    assert!(matches!(
        install_dir(&tmp.path().join("tw.exe")),
        Err(UpdateError::NotPackage(_))
    ));
    // macOS: the folder holding the bundle.
    let exe = tmp
        .path()
        .join("Apps")
        .join("textweaver.app")
        .join("Contents")
        .join("MacOS")
        .join("textweaver");
    std::fs::create_dir_all(exe.parent().unwrap()).unwrap();
    std::fs::write(&exe, b"x").unwrap();
    let dir = install_dir(&exe).unwrap();
    assert!(dir.ends_with("Apps"), "{}", dir.display());
}

#[test]
fn only_existing_entries_on_macos() {
    let tmp = tempfile::tempdir().unwrap();
    let apps = tmp.path().join("Apps");
    std::fs::create_dir_all(apps.join("textweaver.app")).unwrap();
    std::fs::write(apps.join("textweaver.app").join("old"), b"o").unwrap();
    std::fs::write(apps.join("Other"), b"another program").unwrap();
    let new = tmp.path().join("new");
    std::fs::create_dir_all(new.join("textweaver.app")).unwrap();
    std::fs::write(new.join("textweaver.app").join("new"), b"n").unwrap();
    std::fs::write(new.join("NOTICE"), b"n").unwrap();
    assert_eq!(swap_in(&new, &apps, true).unwrap(), 1);
    assert!(apps.join("textweaver.app").join("new").is_file());
    assert!(!apps.join("textweaver.app").join("old").exists());
    assert!(!apps.join("NOTICE").exists());
    assert_eq!(
        std::fs::read(apps.join("Other")).unwrap(),
        b"another program"
    );
}

#[test]
fn an_appimage_is_replaced_whole() {
    let tmp = tempfile::tempdir().unwrap();
    let image = tmp.path().join("textweaver.AppImage");
    let new = tmp.path().join("new.AppImage");
    std::fs::write(&image, b"old").unwrap();
    std::fs::write(&new, b"new").unwrap();
    replace_file(&new, &image).unwrap();
    assert_eq!(std::fs::read(&image).unwrap(), b"new");
    assert!(names_with_update_suffix(tmp.path()).is_empty());
}

#[test]
fn the_finisher_is_told_where_and_what() {
    let a = finish_args(
        Path::new("N"),
        Path::new("I"),
        Path::new("I/textweaver.exe"),
        Some(Path::new("I/textweaver.exe")),
    );
    assert_eq!(a[..3], ["update", "--finish", "N"]);
    assert_eq!(a[a.len() - 2], "--start");
    assert_eq!(ps_quote("a'b"), "'a''b'");
    let tmp = tempfile::tempdir().unwrap();
    assert!(can_write(tmp.path()));
    assert!(start_finisher(tmp.path(), tmp.path(), tmp.path(), None).is_err());
}

#[test]
fn windows_waits_for_the_close_and_linux_installs_now() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("textweaver");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("NOTICE"), b"old").unwrap();
    std::fs::write(dir.join("tw"), b"old tw").unwrap();
    let place = Place::Folder(dir.clone());

    let zip = tmp.path().join("p.zip");
    std::fs::write(&zip, zip_of(&[("p/tw", b"new tw"), ("p/NOTICE", b"new")])).unwrap();
    let windows = Target {
        os: "windows",
        arch: "x86_64",
        kind: PackageKind::WindowsZip,
    };
    let work = tmp.path().join("work");
    let got = install(&zip, &windows, &place, &work).unwrap();
    assert_eq!(
        got,
        Installed::OnClose {
            new_root: work.join("new"),
            install: dir.clone()
        }
    );
    // Nothing replaced yet: that happens after textweaver closes.
    assert_eq!(std::fs::read(dir.join("tw")).unwrap(), b"old tw");

    let linux = Target {
        os: "linux",
        arch: "x86_64",
        kind: PackageKind::Tarball,
    };
    // The same unpacked files, put in place at once.
    let got = install(&zip, &linux, &place, &work).unwrap();
    assert_eq!(got, Installed::Done);
    assert_eq!(std::fs::read(dir.join("tw")).unwrap(), b"new tw");
}
