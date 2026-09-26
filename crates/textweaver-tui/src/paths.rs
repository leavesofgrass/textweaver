//! Tab completion of file paths in the Open, Save As, and Insert image
//! prompts. Since Wave 3 it lives in the app core
//! (`textweaver_app::path_complete`), shared by every frontend's prompts;
//! this re-export keeps the old path.

pub use textweaver_app::path_complete::complete;
