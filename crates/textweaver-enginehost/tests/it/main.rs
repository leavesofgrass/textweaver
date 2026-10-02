//! The integration tests of `textweaver-enginehost`, as one test program
//! (`host_process` stays apart: it needs its own `main`). Add a new
//! integration test as a module in this folder, never as a new file
//! directly in `tests/`.

mod feed;
mod latency;
