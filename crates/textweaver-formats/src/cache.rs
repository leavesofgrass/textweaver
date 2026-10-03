//! On-disk cache of loaded documents.
//!
//! An entry is keyed by the source's absolute path, its modification time
//! and size, and a fingerprint of the loader id, the [`LoadOptions`], and
//! [`CANONICAL_VERSION`]. Any change to the file,
//! the options, or the loaders' output format misses the cache. Entries are
//! JSON files named by a hash of the path, written atomically (temporary
//! file, then rename), under a directory the caller chooses (the store's
//! cache directory in the app).
//!
//! The cache never makes loading fail: an unreadable, corrupt, or stale entry
//! is a miss, and a failed write is ignored by [`DocumentCache::load`].

use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};
use textweaver_text::Document;

use crate::{CANONICAL_VERSION, LoadError, LoadOptions, Registry, Source};

/// 64-bit FNV-1a, stable across platforms and releases (unlike `std`'s
/// hasher), so cache file names and fingerprints stay valid.
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// What a cached document must match to be reused.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CacheKey {
    /// Absolute path of the source file.
    pub path: PathBuf,
    /// Modification time in nanoseconds since the Unix epoch.
    pub mtime_ns: u64,
    /// File size in bytes.
    pub size: u64,
    /// Fingerprint of the loader id, load options, and canonical version.
    pub fingerprint: u64,
}

impl CacheKey {
    /// The key for loading `path` with `loader_id` and `options`, from the
    /// file's current metadata.
    pub fn for_path(
        path: &Path,
        loader_id: &str,
        options: &LoadOptions,
    ) -> Result<Self, LoadError> {
        let io = |e| LoadError::Io(path.to_owned(), e);
        let abs = std::path::absolute(path).map_err(io)?;
        let md = fs::metadata(&abs).map_err(io)?;
        let mtime_ns = md
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |d| u64::try_from(d.as_nanos()).unwrap_or(u64::MAX));
        Ok(CacheKey {
            path: abs,
            mtime_ns,
            size: md.len(),
            fingerprint: fingerprint(loader_id, options),
        })
    }

    fn file_name(&self) -> String {
        let p = self.path.to_string_lossy();
        format!("{:016x}.json", fnv1a64(p.as_bytes()))
    }
}

/// Fingerprint of everything besides the file that shapes a loaded document.
pub fn fingerprint(loader_id: &str, options: &LoadOptions) -> u64 {
    let opts = serde_json::to_string(options).unwrap_or_default();
    fnv1a64(format!("{loader_id}|{CANONICAL_VERSION}|{opts}").as_bytes())
}

#[derive(Serialize, Deserialize)]
struct Entry {
    key: CacheKey,
    document: Document,
}

/// A directory of cached documents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentCache {
    dir: PathBuf,
}

impl DocumentCache {
    /// A cache stored under `dir` (created on first write).
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        DocumentCache { dir: dir.into() }
    }

    /// The cache directory.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn entry_path(&self, key: &CacheKey) -> PathBuf {
        self.dir.join(key.file_name())
    }

    /// The cached document for `key`, if one exists and matches it exactly.
    pub fn get(&self, key: &CacheKey) -> Option<Document> {
        let bytes = fs::read(self.entry_path(key)).ok()?;
        let entry: Entry = serde_json::from_slice(&bytes).ok()?;
        (entry.key == *key).then_some(entry.document)
    }

    /// Stores `doc` under `key`, atomically replacing any previous entry.
    pub fn put(&self, key: &CacheKey, doc: &Document) -> std::io::Result<()> {
        fs::create_dir_all(&self.dir)?;
        let entry = Entry {
            key: key.clone(),
            document: doc.clone(),
        };
        let json = serde_json::to_vec(&entry).map_err(std::io::Error::other)?;
        textweaver_core::fs::write_atomic(&self.entry_path(key), &json)
    }

    /// Loads `source` through the cache: file sources are served from a
    /// matching entry or loaded with `registry` and stored; other sources are
    /// loaded directly.
    pub fn load(
        &self,
        registry: &Registry,
        source: &Source,
        options: &LoadOptions,
    ) -> Result<Document, LoadError> {
        let Source::Path(path) = source else {
            return registry.load(source, options);
        };
        let loader = registry.resolve(source);
        let key = CacheKey::for_path(path, loader.id(), options)?;
        if let Some(mut doc) = self.get(&key) {
            doc.meta.path = Some(path.clone());
            return Ok(doc);
        }
        let doc = loader.load(source, options)?;
        // A cache that cannot be written only costs speed.
        let _ = self.put(&key, &doc);
        Ok(doc)
    }

    /// Removes every cached entry.
    pub fn clear(&self) -> std::io::Result<()> {
        match fs::read_dir(&self.dir) {
            Ok(entries) => {
                for e in entries.flatten() {
                    if e.path().extension().is_some_and(|x| x == "json") {
                        fs::remove_file(e.path())?;
                    }
                }
                Ok(())
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv_is_stable() {
        assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
    }

    #[test]
    fn hit_miss_and_invalidation() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("doc.md");
        fs::write(&file, "# One\n\nBody.").unwrap();
        let cache = DocumentCache::new(dir.path().join("cache"));
        let registry = Registry::with_builtins();
        let source = Source::Path(file.clone());
        let opts = LoadOptions::default();

        let first = cache.load(&registry, &source, &opts).unwrap();
        let key = CacheKey::for_path(&file, "markdown", &opts).unwrap();
        let cached = cache.get(&key).expect("stored after the first load");
        assert_eq!(cached, first);

        // Different options: a different fingerprint, so a miss.
        let skip = LoadOptions {
            skip_code: true,
            ..LoadOptions::default()
        };
        let other = CacheKey::for_path(&file, "markdown", &skip).unwrap();
        assert!(cache.get(&other).is_none());

        // A changed file (size differs): a miss, then the new text.
        fs::write(&file, "# Two\n\nLonger body text.").unwrap();
        let second = cache.load(&registry, &source, &opts).unwrap();
        assert_eq!(second.meta.title.as_deref(), Some("Two"));

        // A corrupt entry is a miss, not an error.
        let key = CacheKey::for_path(&file, "markdown", &opts).unwrap();
        fs::write(cache.dir().join(key.file_name()), "not json").unwrap();
        assert!(cache.get(&key).is_none());
        let third = cache.load(&registry, &source, &opts).unwrap();
        assert_eq!(third, second);

        cache.clear().unwrap();
        assert!(cache.get(&key).is_none());
    }
}
