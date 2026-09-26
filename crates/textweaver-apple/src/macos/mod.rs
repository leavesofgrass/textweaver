//! The macOS backends and the run-loop plumbing they share.
//!
//! This module talks to Objective-C through `objc2`, so it allows
//! `unsafe_code`; every `unsafe` block carries a `// SAFETY:` comment.

#![allow(unsafe_code)]

pub(crate) mod avspeech;
pub(crate) mod nsspeech;
pub(crate) mod output;
pub(crate) mod runloop;
pub(crate) mod ssm;
