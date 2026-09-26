//! No preview server in a build without the `publish` feature. The full
//! module is `src/preview_server.rs`.

/// Never made in this build.
#[derive(Debug)]
pub(crate) enum PreviewServer {}
