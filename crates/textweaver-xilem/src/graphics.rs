//! Which graphics API the window draws with (ADR-0028, "Memory"; Wave 5).
//!
//! wgpu opens every backend it finds by default, so on Windows both the
//! Direct3D 12 and the Vulkan drivers load. Choosing one is an opt-in:
//! `--graphics vulkan` for one run, or `graphics = "vulkan"` in the
//! `[gui]` table of `settings.toml`. With Vulkan the window used about 26
//! MB less memory on the development machine (ADR-0028's status update),
//! but that depends on the graphics driver, so the default stays "auto"
//! (wgpu's own choice) until other machines are measured.
//!
//! wgpu reads the choice from the `WGPU_BACKEND` environment variable, so
//! [`apply`] sets it at the very start of `main`, before any other thread
//! exists. A `WGPU_BACKEND` already set wins, as it always did.

use std::path::Path;

use textweaver_app::store::{Paths, SettingsStore};

/// A graphics API for the window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GraphicsBackend {
    /// wgpu's own choice (every backend it finds). The default.
    #[default]
    Auto,
    /// Vulkan.
    Vulkan,
    /// Direct3D 12 (Windows).
    Dx12,
    /// Metal (macOS).
    Metal,
    /// OpenGL (or OpenGL ES).
    Gl,
}

impl GraphicsBackend {
    /// The names `--graphics` and the setting take.
    pub const NAMES: [&'static str; 5] = ["auto", "vulkan", "dx12", "metal", "gl"];

    /// The backend named `s` (case does not matter).
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "auto" | "" => Some(Self::Auto),
            "vulkan" => Some(Self::Vulkan),
            "dx12" | "d3d12" => Some(Self::Dx12),
            "metal" => Some(Self::Metal),
            "gl" | "opengl" => Some(Self::Gl),
            _ => None,
        }
    }

    /// The value for `WGPU_BACKEND`, or `None` for wgpu's own choice.
    pub fn wgpu_name(self) -> Option<&'static str> {
        match self {
            Self::Auto => None,
            Self::Vulkan => Some("vulkan"),
            Self::Dx12 => Some("dx12"),
            Self::Metal => Some("metal"),
            Self::Gl => Some("gl"),
        }
    }
}

/// The `graphics` key of a `[gui]` table (unknown to the store for now, so
/// it is kept with the table's other keys): `None` when absent or not one
/// of [`GraphicsBackend::NAMES`].
pub fn from_gui_table(extra: &toml::Table) -> Option<GraphicsBackend> {
    extra
        .get("graphics")
        .and_then(toml::Value::as_str)
        .and_then(GraphicsBackend::parse)
}

/// The backend asked for: `--graphics` first, then the setting in the
/// settings file under `home` (or the platform's folder).
pub fn wanted(cli: Option<GraphicsBackend>, home: Option<&Path>) -> GraphicsBackend {
    if let Some(b) = cli {
        return b;
    }
    let paths = match home {
        Some(h) => Paths::under(h),
        None => match Paths::platform() {
            Ok(p) => p,
            Err(_) => return GraphicsBackend::Auto,
        },
    };
    let (settings, _warnings) = SettingsStore::new(paths).load();
    from_gui_table(&settings.gui.extra).unwrap_or_default()
}

/// Makes wgpu use `backend`, unless `WGPU_BACKEND` is already set.
/// Returns the name set, if any.
///
/// Call it only at the start of `main`, before any other thread is
/// started: it sets an environment variable.
pub fn apply(backend: GraphicsBackend) -> Option<&'static str> {
    let name = backend.wgpu_name()?;
    if std::env::var_os("WGPU_BACKEND").is_some() {
        return None;
    }
    // SAFETY: `main` calls this before it starts any thread (the log file,
    // speech, the ticker, and the window all start later), so nothing reads
    // or writes the environment at the same time.
    #[allow(unsafe_code)]
    unsafe {
        std::env::set_var("WGPU_BACKEND", name);
    }
    Some(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_parse_and_map_to_wgpu() {
        for n in GraphicsBackend::NAMES {
            let b = GraphicsBackend::parse(n).expect(n);
            assert_eq!(b.wgpu_name().unwrap_or("auto"), n);
        }
        assert_eq!(
            GraphicsBackend::parse("Vulkan"),
            Some(GraphicsBackend::Vulkan)
        );
        assert_eq!(GraphicsBackend::parse("d3d12"), Some(GraphicsBackend::Dx12));
        assert_eq!(GraphicsBackend::parse("glide"), None);
    }

    #[test]
    fn the_gui_table_names_the_backend() {
        let t = |s: &str| -> toml::Table { s.parse().expect("toml") };
        assert_eq!(from_gui_table(&t("")), None);
        assert_eq!(
            from_gui_table(&t("graphics = \"vulkan\"")),
            Some(GraphicsBackend::Vulkan)
        );
        assert_eq!(from_gui_table(&t("graphics = \"fast\"")), None);
        assert_eq!(from_gui_table(&t("graphics = 3")), None);
    }

    #[test]
    fn the_command_line_wins_over_the_setting() {
        let dir = std::env::temp_dir().join(format!("tw-gui-graphics-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let paths = Paths::under(&dir);
        if let Some(parent) = paths.settings_file().parent() {
            std::fs::create_dir_all(parent).expect("config dir");
        }
        std::fs::write(paths.settings_file(), "[gui]\ngraphics = \"dx12\"\n").expect("write");
        assert_eq!(wanted(None, Some(&dir)), GraphicsBackend::Dx12);
        assert_eq!(
            wanted(Some(GraphicsBackend::Vulkan), Some(&dir)),
            GraphicsBackend::Vulkan
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}
