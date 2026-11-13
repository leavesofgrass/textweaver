//! Contracted braille with the print each cell stands for, through
//! liblouis's library function `lou_translate`.
//!
//! The `lou_translate` program prints braille and nothing else; the
//! library function also fills an `inputPos` array, one value per cell.
//! liblouis gives every cell of one rule the position of the rule's first
//! print char, so a run of equal values, up to the next run's value, is
//! one rule's print ("nation" is n, a, tion; "kindness" is k, in, d, ness).
//! The Braille view's print line uses it ([`translate_positions`]).
//!
//! - **Which library.** The one beside the `lou_translate` the BRF writer
//!   runs (`textweaver_store::find_helper`), loaded at run time by full
//!   path, so its tables and the program's agree. On Windows only that:
//!   an older `liblouis.dll` in the system folder wins Windows' search by
//!   bare name and rejects today's UEB tables. On Linux and macOS the
//!   program's prefix (`<prefix>/bin/lou_translate` with
//!   `<prefix>/lib/liblouis.so.20` or `liblouis.20.dylib`) comes first,
//!   then the system's versioned library name.
//! - **Tables.** A copy with its tables beside it is given them by full
//!   path (liblouis finds a table's includes beside it); the process
//!   environment is never changed. When `LOUIS_TABLEPATH` is set, the
//!   names go as they are, so it still comes first, as for the program.
//!   (`lou_setDataPath` would do the same, but liblouis 3.39 prints a
//!   deprecation warning on every call, which would reach the terminal.)
//! - **Characters.** liblouis's `widechar` is two bytes in some builds and
//!   four in others; `lou_charSize` says which, and text goes in as UTF-16
//!   or UTF-32 to match.
//! - **One call at a time.** liblouis keeps global state (compiled tables,
//!   the data path), so every call into it holds one lock.
//! - **Failure is quiet.** A library that cannot load, a missing symbol,
//!   or a failed call is an error in words; the caller falls back to its
//!   own pairing.

use std::path::{Path, PathBuf};

/// Contracted braille with each cell's print.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Positions {
    /// The cells, as Unicode braille patterns (U+2800 to U+28FF).
    pub braille: String,
    /// For each cell, the index of the print char (in `char`s of the text
    /// given) that its rule starts at.
    pub input_pos: Vec<usize>,
}

/// A buffer of liblouis `widechar`s, two or four bytes each.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Wide {
    /// Two-byte `widechar`: UTF-16 code units.
    Two(Vec<u16>),
    /// Four-byte `widechar`: one Unicode scalar each.
    Four(Vec<u32>),
}

impl Wide {
    /// A zeroed buffer of `len` units of `size` bytes, or `None` for a size
    /// liblouis does not have.
    fn zeroed(size: usize, len: usize) -> Option<Self> {
        match size {
            2 => Some(Wide::Two(vec![0; len])),
            4 => Some(Wide::Four(vec![0; len])),
            _ => None,
        }
    }

    /// How many units it holds.
    fn len(&self) -> usize {
        match self {
            Wide::Two(v) => v.len(),
            Wide::Four(v) => v.len(),
        }
    }

    #[cfg(feature = "liblouis")]
    fn as_ptr(&self) -> *const std::ffi::c_void {
        match self {
            Wide::Two(v) => v.as_ptr().cast(),
            Wide::Four(v) => v.as_ptr().cast(),
        }
    }

    #[cfg(feature = "liblouis")]
    fn as_mut_ptr(&mut self) -> *mut std::ffi::c_void {
        match self {
            Wide::Two(v) => v.as_mut_ptr().cast(),
            Wide::Four(v) => v.as_mut_ptr().cast(),
        }
    }
}

/// `text` as `widechar`s of `size` bytes, with the char index each unit
/// belongs to (a char outside the Basic Multilingual Plane is two UTF-16
/// units). `None` for a size liblouis does not have.
fn encode(text: &str, size: usize) -> Option<(Wide, Vec<usize>)> {
    let mut owner = Vec::new();
    let wide = match size {
        2 => {
            let mut units = Vec::new();
            let mut buf = [0u16; 2];
            for (i, c) in text.chars().enumerate() {
                for u in c.encode_utf16(&mut buf) {
                    units.push(*u);
                    owner.push(i);
                }
            }
            Wide::Two(units)
        }
        4 => {
            let units: Vec<u32> = text.chars().map(u32::from).collect();
            owner.extend(0..units.len());
            Wide::Four(units)
        }
        _ => return None,
    };
    Some((wide, owner))
}

/// The first `len` units of liblouis's output and their `inputPos`
/// values, as cells and the print char each starts at. `owner` maps input
/// units to chars ([`encode`]). `None` when the output is not text or a
/// position points outside the input.
fn decode(out: &Wide, len: usize, input_pos: &[i32], owner: &[usize]) -> Option<Positions> {
    let len = len.min(out.len()).min(input_pos.len());
    let char_of = |unit: usize| -> Option<usize> {
        let p = usize::try_from(*input_pos.get(unit)?).ok()?;
        owner.get(p).copied()
    };
    let mut braille = String::new();
    let mut positions = Vec::new();
    match out {
        Wide::Two(v) => {
            let mut unit = 0;
            for c in char::decode_utf16(v[..len].iter().copied()) {
                let c = c.ok()?;
                braille.push(c);
                positions.push(char_of(unit)?);
                unit += c.len_utf16();
            }
        }
        Wide::Four(v) => {
            for (unit, &u) in v[..len].iter().enumerate() {
                braille.push(char::from_u32(u)?);
                positions.push(char_of(unit)?);
            }
        }
    }
    Some(Positions {
        braille,
        input_pos: positions,
    })
}

/// The liblouis library files to try for a `lou_translate` at `program`,
/// in order: full paths beside it that exist, then (not on Windows) the
/// system's versioned name.
pub fn library_candidates(program: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    if cfg!(windows) {
        // Full paths only: never a bare name (see the module docs).
        if let Some(dir) = program.parent().filter(|d| d.is_absolute()) {
            for name in ["liblouis.dll", "liblouis-20.dll"] {
                out.push(dir.join(name));
            }
        }
        out.retain(|p| p.is_file());
        return out;
    }
    let name = if cfg!(target_os = "macos") {
        "liblouis.20.dylib"
    } else {
        "liblouis.so.20"
    };
    if let Some(prefix) = program.parent().and_then(Path::parent) {
        for lib in ["lib", "lib64"] {
            out.push(prefix.join(lib).join(name));
        }
    }
    out.retain(|p| p.is_absolute() && p.is_file());
    out.push(PathBuf::from(name));
    out
}

/// `tables`, a comma-separated list of table names, with each name under
/// the folder `dir`, when given and `LOUIS_TABLEPATH` (`env`) is unset.
/// `None` when the folder cannot be written into a table list (not UTF-8,
/// or holding a comma).
fn table_list(tables: &str, dir: Option<&Path>, env: bool) -> Option<String> {
    let Some(dir) = dir.filter(|_| !env) else {
        return Some(tables.to_owned());
    };
    let dir = dir.to_str().filter(|d| !d.contains(','))?;
    let joined: Vec<String> = tables
        .split(',')
        .map(|t| Path::new(dir).join(t).to_string_lossy().into_owned())
        .collect();
    Some(joined.join(","))
}

/// Translates `text` with liblouis's `unicode.dis` and `table` (such as
/// `en-ueb-g2.ctb`), through the library beside the `lou_translate` the
/// BRF writer runs, and says which print char each cell starts at.
///
/// # Errors
///
/// When no `lou_translate` is found, its library cannot be loaded or lacks
/// a function, or the translation fails, in words.
#[cfg(feature = "liblouis")]
pub fn translate_positions(text: &str, table: &str) -> Result<Positions, String> {
    let lib = sys::default_library()?;
    sys::translate(lib, text, &format!("unicode.dis,{table}"))
}

/// [`translate_positions`] with the library at `library` and, when given,
/// the tables in the folder `tables`.
/// The library is loaded for this call only.
///
/// # Errors
///
/// As [`translate_positions`].
#[cfg(feature = "liblouis")]
pub fn translate_positions_with(
    library: &Path,
    tables: Option<&Path>,
    text: &str,
    table: &str,
) -> Result<Positions, String> {
    let lib = sys::Louis::open(library, tables.map(Path::to_path_buf))?;
    sys::translate(&lib, text, &format!("unicode.dis,{table}"))
}

/// The calls into liblouis: the only unsafe code in this crate.
#[cfg(feature = "liblouis")]
#[allow(unsafe_code)]
mod sys {
    use std::ffi::{CString, c_char, c_int, c_void};
    use std::path::{Path, PathBuf};
    use std::sync::{Mutex, OnceLock};

    use libloading::Library;

    use super::{Positions, Wide, decode, encode};

    /// `lou_translate` (liblouis.h; `EXPORT_CALL` is `__stdcall` on
    /// Windows, which `"system"` matches, and the C convention elsewhere).
    type TranslateFn = unsafe extern "system" fn(
        table_list: *const c_char,
        inbuf: *const c_void,
        inlen: *mut c_int,
        outbuf: *mut c_void,
        outlen: *mut c_int,
        typeform: *mut u16,
        spacing: *mut c_char,
        output_pos: *mut c_int,
        input_pos: *mut c_int,
        cursor_pos: *mut c_int,
        mode: c_int,
    ) -> c_int;
    /// `lou_charSize`: the bytes in a `widechar`.
    type CharSizeFn = unsafe extern "system" fn() -> c_int;

    /// One loaded liblouis.
    pub(super) struct Louis {
        translate: TranslateFn,
        char_size: CharSizeFn,
        /// The tables folder that came with the library, if any.
        tables: Option<PathBuf>,
        /// Keeps the library mapped while the pointers above are used.
        _library: Library,
    }

    /// Every call into liblouis holds this: it keeps global state.
    static LOCK: Mutex<()> = Mutex::new(());

    /// The library beside the found `lou_translate`, loaded once.
    static DEFAULT: OnceLock<Result<Louis, String>> = OnceLock::new();

    pub(super) fn default_library() -> Result<&'static Louis, String> {
        DEFAULT
            .get_or_init(|| {
                let program = textweaver_store::find_helper("lou_translate", None)
                    .ok_or_else(|| "lou_translate is not installed".to_owned())?;
                let data = textweaver_store::helpers::liblouis_tables_beside(&program);
                let mut tried = Vec::new();
                for candidate in super::library_candidates(&program) {
                    match Louis::open(&candidate, data.clone()) {
                        Ok(l) => return Ok(l),
                        Err(e) => tried.push(e),
                    }
                }
                Err(format!(
                    "liblouis's library was not found beside {} ({})",
                    program.display(),
                    tried.join("; ")
                ))
            })
            .as_ref()
            .map_err(Clone::clone)
    }

    fn load(path: &Path) -> Result<Library, String> {
        #[cfg(windows)]
        {
            if !path.is_absolute() {
                return Err(format!("{} is not a full path", path.display()));
            }
            // SAFETY: loading runs liblouis's initializers, which set up
            // only its own globals (a plain C library). The path is the
            // full path beside the found `lou_translate`, and the flag
            // resolves its own dependencies from its folder.
            unsafe {
                libloading::os::windows::Library::load_with_flags(
                    path,
                    libloading::os::windows::LOAD_WITH_ALTERED_SEARCH_PATH,
                )
            }
            .map(Library::from)
            .map_err(|e| format!("cannot load {}: {e}", path.display()))
        }
        #[cfg(not(windows))]
        {
            // SAFETY: as above; on Linux and macOS a bare name is the
            // system's versioned liblouis.
            unsafe { Library::new(path) }
                .map_err(|e| format!("cannot load {}: {e}", path.display()))
        }
    }

    /// Copies one function pointer out of `lib`.
    ///
    /// # Safety
    ///
    /// `T` must be the function's C signature as liblouis.h declares it,
    /// and the pointer must not be called after `lib` is dropped.
    unsafe fn symbol<T: Copy>(lib: &Library, name: &str) -> Result<T, String> {
        // SAFETY: the caller guarantees `T` matches the symbol's type.
        unsafe { lib.get::<T>(name.as_bytes()) }
            .map(|s| *s)
            .map_err(|e| format!("liblouis lacks {name}: {e}"))
    }

    impl Louis {
        pub(super) fn open(path: &Path, tables: Option<PathBuf>) -> Result<Self, String> {
            let lib = load(path)?;
            // SAFETY: each type is the signature liblouis.h declares for
            // that function, and the pointers are kept with `lib`.
            unsafe {
                Ok(Louis {
                    translate: symbol(&lib, "lou_translate")?,
                    char_size: symbol(&lib, "lou_charSize")?,
                    tables,
                    _library: lib,
                })
            }
        }
    }

    /// Translates `text` with `tables` through `lib`.
    pub(super) fn translate(lib: &Louis, text: &str, tables: &str) -> Result<Positions, String> {
        if text.is_empty() {
            return Ok(Positions::default());
        }
        let env = std::env::var_os(textweaver_store::helpers::LOUIS_TABLEPATH)
            .is_some_and(|v| !v.is_empty());
        let list = super::table_list(tables, lib.tables.as_deref(), env)
            .ok_or_else(|| "the tables folder cannot be given to liblouis".to_owned())?;
        let table_list = CString::new(list).map_err(|e| format!("bad table name: {e}"))?;
        let _guard = LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        // SAFETY: `lou_charSize` takes nothing and returns an int.
        let size = unsafe { (lib.char_size)() };
        let size = usize::try_from(size).map_err(|_| "liblouis gave a bad char size".to_owned())?;
        let (input, owner) =
            encode(text, size).ok_or_else(|| format!("liblouis has {size}-byte chars"))?;
        let in_units = input.len();
        let in_len = c_int::try_from(in_units).map_err(|_| "text too long".to_owned())?;
        let mut cap = in_units * 4 + 16;
        for _ in 0..4 {
            let mut out = Wide::zeroed(size, cap).ok_or("bad char size")?;
            let mut input_pos: Vec<c_int> = vec![0; cap];
            let mut inlen = in_len;
            let mut outlen = c_int::try_from(cap).map_err(|_| "text too long".to_owned())?;
            // SAFETY: the buffers are `widechar`s of the size liblouis
            // reported, `inlen` and `outlen` give their lengths in units,
            // `input_pos` has room for `outlen` ints, and the unused
            // arguments are null, which liblouis accepts. The lock is held.
            let ok = unsafe {
                (lib.translate)(
                    table_list.as_ptr(),
                    input.as_ptr(),
                    &mut inlen,
                    out.as_mut_ptr(),
                    &mut outlen,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    input_pos.as_mut_ptr(),
                    std::ptr::null_mut(),
                    0,
                )
            };
            if ok == 0 {
                return Err(format!("liblouis could not translate with {tables}"));
            }
            // A full output buffer stops liblouis early: try a bigger one.
            if usize::try_from(inlen).unwrap_or(0) < in_units {
                cap *= 2;
                continue;
            }
            let len = usize::try_from(outlen).unwrap_or(0);
            return decode(&out, len, &input_pos, &owner)
                .ok_or_else(|| "liblouis gave positions outside the text".to_owned());
        }
        Err("liblouis's braille did not fit".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_byte_chars_are_utf16_and_map_back_to_chars() {
        // "a" then a char outside the Basic Multilingual Plane, then "b".
        let text = "a\u{1D400}b";
        let (wide, owner) = encode(text, 2).unwrap();
        assert_eq!(wide.len(), 4);
        assert_eq!(owner, vec![0, 1, 1, 2]);
        // Three cells: the second's rule starts at unit 1 (the wide char),
        // the third at unit 3 ("b").
        let out = Wide::Two(vec![0x2801, 0x2803, 0x2805, 0]);
        let p = decode(&out, 3, &[0, 1, 3, 0], &owner).unwrap();
        assert_eq!(p.braille, "\u{2801}\u{2803}\u{2805}");
        assert_eq!(p.input_pos, vec![0, 1, 2]);
    }

    #[test]
    fn four_byte_chars_are_one_unit_each() {
        let text = "a\u{1D400}b";
        let (wide, owner) = encode(text, 4).unwrap();
        assert_eq!(wide, Wide::Four(vec![0x61, 0x1D400, 0x62]));
        assert_eq!(owner, vec![0, 1, 2]);
        let out = Wide::Four(vec![0x2801, 0x2803, 0x2805]);
        let p = decode(&out, 3, &[0, 1, 2], &owner).unwrap();
        assert_eq!(p.braille, "\u{2801}\u{2803}\u{2805}");
        assert_eq!(p.input_pos, vec![0, 1, 2]);
    }

    #[test]
    fn odd_sizes_and_bad_positions_are_refused() {
        assert!(encode("a", 1).is_none());
        assert!(encode("a", 3).is_none());
        assert!(Wide::zeroed(8, 1).is_none());
        let (_, owner) = encode("ab", 4).unwrap();
        // A position past the input.
        assert!(decode(&Wide::Four(vec![0x2801]), 1, &[5], &owner).is_none());
        // A negative position.
        assert!(decode(&Wide::Four(vec![0x2801]), 1, &[-1], &owner).is_none());
        // A lone surrogate in two-byte output.
        assert!(decode(&Wide::Two(vec![0xD800]), 1, &[0], &owner).is_none());
    }

    #[test]
    fn tables_go_by_full_path_unless_the_variable_is_set() {
        let dir = Path::new("tables");
        let list = table_list("unicode.dis,en-ueb-g2.ctb", Some(dir), false).unwrap();
        let want: Vec<String> = ["unicode.dis", "en-ueb-g2.ctb"]
            .iter()
            .map(|t| dir.join(t).to_string_lossy().into_owned())
            .collect();
        assert_eq!(list, want.join(","));
        assert_eq!(
            table_list("a.ctb", Some(dir), true).as_deref(),
            Some("a.ctb")
        );
        assert_eq!(table_list("a.ctb", None, false).as_deref(), Some("a.ctb"));
        assert!(table_list("a.ctb", Some(Path::new("x,y")), false).is_none());
    }

    #[test]
    fn windows_never_tries_a_bare_name() {
        let tmp = tempfile::tempdir().unwrap();
        let program = tmp.path().join("bin").join("lou_translate");
        let found = library_candidates(&program);
        if cfg!(windows) {
            // Nothing beside it: nothing at all, not "liblouis.dll".
            assert!(found.is_empty(), "{found:?}");
        } else {
            // Only the system's versioned name is left.
            assert_eq!(found.len(), 1, "{found:?}");
            assert!(found[0].is_relative());
        }
    }

    #[test]
    fn the_library_beside_the_program_comes_first() {
        let tmp = tempfile::tempdir().unwrap();
        let (dir, name) = if cfg!(windows) {
            (tmp.path().join("bin"), "liblouis.dll")
        } else if cfg!(target_os = "macos") {
            (tmp.path().join("lib"), "liblouis.20.dylib")
        } else {
            (tmp.path().join("lib"), "liblouis.so.20")
        };
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(name), b"x").unwrap();
        let program = tmp.path().join("bin").join("lou_translate");
        let found = library_candidates(&program);
        assert_eq!(found.first(), Some(&dir.join(name)), "{found:?}");
    }

    /// A library path that does not exist is an error, not a panic.
    #[cfg(feature = "liblouis")]
    #[test]
    fn a_missing_library_is_an_error() {
        let tmp = tempfile::tempdir().unwrap();
        let missing = tmp.path().join("no-such-liblouis.dll");
        let e = translate_positions_with(&missing, None, "nation", "en-ueb-g2.ctb").unwrap_err();
        assert!(e.contains("cannot load"), "{e}");
    }

    /// The spike's ten words through the real library, when it is found:
    /// each cell points at the first print char of its rule.
    #[cfg(feature = "liblouis")]
    #[test]
    fn the_real_library_gives_rule_positions() {
        let Ok(p) = translate_positions("nation kindness", "en-ueb-g2.ctb") else {
            let required = std::env::var("TEXTWEAVER_REQUIRE_TOOLS")
                .is_ok_and(|v| v.split(',').any(|t| t.trim() == "lou_translate"));
            assert!(
                !required,
                "Fail: liblouis's library did not load, but TEXTWEAVER_REQUIRE_TOOLS requires \
                 lou_translate here"
            );
            return;
        };
        // nation: n, a, then "tion" (two cells); kindness: k, "in", d,
        // then "ness" (two cells).
        assert_eq!(p.braille.chars().count(), 4 + 1 + 5, "{p:?}");
        assert_eq!(p.input_pos, vec![0, 1, 2, 2, 6, 7, 8, 10, 11, 11], "{p:?}");
        let words = [
            "about",
            "knowledge",
            "children",
            "without",
            "through",
            "afternoon",
            "Sometimes",
            "Grandmother",
        ];
        for w in words {
            let p = translate_positions(w, "en-ueb-g2.ctb").unwrap();
            assert_eq!(p.braille.chars().count(), p.input_pos.len());
            assert_eq!(p.input_pos.first(), Some(&0), "{w}: {p:?}");
            assert!(p.input_pos.windows(2).all(|x| x[0] <= x[1]), "{w}: {p:?}");
        }
    }
}
