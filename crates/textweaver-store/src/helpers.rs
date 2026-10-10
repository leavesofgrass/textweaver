//! The helper programs textweaver runs but does not ship (pandoc,
//! liblouis's `lou_translate`), found the same way everywhere: the copy in
//! textweaver's components folder first, then the program an environment
//! variable names, then the PATH. So a helper textweaver fetched for the
//! reader is used at once, and one installed some other way is found
//! without asking.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::paths::{components_dir, find_in_components};

/// The environment variable that names liblouis's tables folder.
pub const LOUIS_TABLEPATH: &str = "LOUIS_TABLEPATH";

/// The helper `program` (`pandoc`, `lou_translate`): in the components
/// folder ([`components_dir`]), then the program `env` names when it is
/// set and is a file, then `program` on the PATH. `None` when none is
/// found.
pub fn find_helper(program: &str, env: Option<&str>) -> Option<PathBuf> {
    find_helper_in(
        components_dir().as_deref(),
        program,
        env.and_then(std::env::var_os),
        std::env::var_os("PATH"),
    )
}

/// [`find_helper`] with its places given: the components folder, the
/// environment variable's value, and the PATH's.
pub fn find_helper_in(
    components: Option<&Path>,
    program: &str,
    env_value: Option<OsString>,
    path: Option<OsString>,
) -> Option<PathBuf> {
    let exe = if cfg!(windows) {
        format!("{program}.exe")
    } else {
        program.to_owned()
    };
    if let Some(p) = components.and_then(|d| find_in_components(d, &[&exe])) {
        return Some(p);
    }
    if let Some(p) = env_value.filter(|v| !v.is_empty()).map(PathBuf::from)
        && p.is_file()
    {
        return Some(p);
    }
    textweaver_core::process::find_program_in(program, &path?)
}

/// liblouis's tables beside a `lou_translate` from an unpacked release:
/// `<release>/bin/lou_translate` with `<release>/share/liblouis/tables`.
pub fn liblouis_tables_beside(program: &Path) -> Option<PathBuf> {
    let root = program.parent()?.parent()?;
    let tables = root.join("share").join("liblouis").join("tables");
    tables.is_dir().then_some(tables)
}

/// A command for liblouis's `lou_translate`, found by [`find_helper`]. A
/// copy in the components folder is told where its tables are
/// ([`LOUIS_TABLEPATH`]) unless the variable is already set.
pub fn lou_translate() -> Command {
    let found = find_helper("lou_translate", None);
    let mut c = textweaver_core::process::command(
        found
            .as_deref()
            .map_or_else(|| OsString::from("lou_translate"), |p| p.into()),
    );
    if std::env::var_os(LOUIS_TABLEPATH).is_none()
        && let Some(tables) = found.as_deref().and_then(liblouis_tables_beside)
    {
        c.env(LOUIS_TABLEPATH, tables);
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_components_folder_comes_first_then_the_variable_then_the_path() {
        let tmp = tempfile::tempdir().unwrap();
        let exe = if cfg!(windows) {
            "pandoc.exe"
        } else {
            "pandoc"
        };
        let comp = tmp.path().join("components");
        let on_path = tmp.path().join("bin");
        std::fs::create_dir_all(&on_path).unwrap();
        std::fs::write(on_path.join(exe), b"x").unwrap();
        let path = Some(OsString::from(on_path.as_os_str()));
        // Only on the PATH.
        assert_eq!(
            find_helper_in(Some(&comp), "pandoc", None, path.clone()),
            Some(on_path.join(exe))
        );
        // The variable names a file: it wins over the PATH.
        let named = tmp.path().join("named-pandoc");
        std::fs::write(&named, b"x").unwrap();
        assert_eq!(
            find_helper_in(
                Some(&comp),
                "pandoc",
                Some(named.clone().into()),
                path.clone()
            ),
            Some(named.clone())
        );
        // In the components folder, a few folders down: it wins.
        let unpacked = comp.join("pandoc").join("pandoc-3.12.1");
        std::fs::create_dir_all(&unpacked).unwrap();
        std::fs::write(unpacked.join(exe), b"x").unwrap();
        assert_eq!(
            find_helper_in(Some(&comp), "pandoc", Some(named.into()), path),
            Some(unpacked.join(exe))
        );
        assert_eq!(find_helper_in(None, "pandoc", None, None), None);
    }

    #[test]
    fn liblouis_tables_are_found_beside_its_program() {
        let tmp = tempfile::tempdir().unwrap();
        let bin = tmp.path().join("bin");
        let tables = tmp.path().join("share").join("liblouis").join("tables");
        std::fs::create_dir_all(&bin).unwrap();
        assert_eq!(liblouis_tables_beside(&bin.join("lou_translate")), None);
        std::fs::create_dir_all(&tables).unwrap();
        assert_eq!(
            liblouis_tables_beside(&bin.join("lou_translate")),
            Some(tables)
        );
    }
}
