//! Voice lists that never keep anyone waiting (Phase 2).
//!
//! Listing voices can be slow: SAPI starts a helper process per registry to
//! read its voice tokens, and a speech server answers over a socket. Each
//! backend lists its voices once, when it starts, and keeps the answer, a
//! failure included, in a [`VoiceCache`]: in-process engines fill it at
//! once; SAPI fills it from a background thread
//! ([`VoiceCache::spawn`]). The speech service reads the cache from any
//! thread without waiting ([`crate::SpeechService::voice_list`]), so the
//! Choose Voice list opens at once, or says the voices are still loading
//! and opens when they arrive.

use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use crate::backend::{SpeechError, Voice};

/// What is known about a backend's voices.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VoiceList {
    /// Still being listed.
    Loading,
    /// The voices.
    Ready(Vec<Voice>),
    /// Listing failed; this is why.
    Failed(SpeechError),
}

impl VoiceList {
    /// The voices, the failure, or [`SpeechError::Engine`] saying they are
    /// still loading.
    pub fn into_result(self) -> Result<Vec<Voice>, SpeechError> {
        match self {
            VoiceList::Ready(v) => Ok(v),
            VoiceList::Failed(e) => Err(e),
            VoiceList::Loading => Err(SpeechError::Engine("the voices are still loading".into())),
        }
    }

    /// True while the voices are being listed.
    pub fn is_loading(&self) -> bool {
        matches!(self, VoiceList::Loading)
    }
}

type Hook = Box<dyn FnOnce() + Send + 'static>;

struct State {
    list: VoiceList,
    hooks: Vec<Hook>,
}

struct Inner {
    state: Mutex<State>,
    ready: Condvar,
}

/// A voice list filled once and shared between threads. Clones share it.
#[derive(Clone)]
pub struct VoiceCache {
    inner: Arc<Inner>,
}

impl std::fmt::Debug for VoiceCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VoiceCache")
            .field("list", &self.get())
            .finish()
    }
}

impl VoiceCache {
    fn with(list: VoiceList) -> Self {
        VoiceCache {
            inner: Arc::new(Inner {
                state: Mutex::new(State {
                    list,
                    hooks: Vec::new(),
                }),
                ready: Condvar::new(),
            }),
        }
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.inner.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// A cache already holding `result` (an in-process engine's list).
    pub fn ready(result: Result<Vec<Voice>, SpeechError>) -> Self {
        Self::with(match result {
            Ok(v) => VoiceList::Ready(v),
            Err(e) => VoiceList::Failed(e),
        })
    }

    /// An empty cache, filled later with [`set`](Self::set).
    pub fn loading() -> Self {
        Self::with(VoiceList::Loading)
    }

    /// A cache filled by `list` on a background thread named `name`. If the
    /// thread cannot start, `list` runs here.
    pub fn spawn(
        name: &str,
        list: impl FnOnce() -> Result<Vec<Voice>, SpeechError> + Send + 'static,
    ) -> Self {
        let cache = Self::loading();
        let filler = cache.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name(name.to_owned())
            .spawn(move || {
                let Ok(list) = rx.recv() else {
                    return;
                };
                let list: Box<dyn FnOnce() -> Result<Vec<Voice>, SpeechError> + Send> = list;
                filler.set(list());
            });
        let list: Box<dyn FnOnce() -> Result<Vec<Voice>, SpeechError> + Send> = Box::new(list);
        match spawned {
            Ok(_) => {
                if let Err(std::sync::mpsc::SendError(list)) = tx.send(list) {
                    cache.set(list());
                }
            }
            Err(e) => {
                log::warn!("cannot start the voice listing thread ({e}); listing here");
                cache.set(list());
            }
        }
        cache
    }

    /// Fills the cache (the first answer wins) and runs the hooks waiting
    /// for it.
    pub fn set(&self, result: Result<Vec<Voice>, SpeechError>) {
        let hooks = {
            let mut st = self.lock();
            if !st.list.is_loading() {
                return;
            }
            st.list = match result {
                Ok(v) => VoiceList::Ready(v),
                Err(e) => VoiceList::Failed(e),
            };
            std::mem::take(&mut st.hooks)
        };
        self.inner.ready.notify_all();
        for h in hooks {
            h();
        }
    }

    /// What is known now; never waits.
    pub fn get(&self) -> VoiceList {
        self.lock().list.clone()
    }

    /// Waits up to `timeout` for the list (tests, and one-shot tools that
    /// may block). Returns what is known then.
    pub fn wait(&self, timeout: Duration) -> VoiceList {
        let deadline = Instant::now() + timeout;
        let mut st = self.lock();
        while st.list.is_loading() {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                break;
            }
            st = self
                .inner
                .ready
                .wait_timeout(st, left)
                .map(|(g, _)| g)
                .unwrap_or_else(|e| e.into_inner().0);
        }
        st.list.clone()
    }

    /// Runs `hook` once the list is known: at once when it already is,
    /// else on the thread that fills it.
    pub fn on_ready(&self, hook: impl FnOnce() + Send + 'static) {
        let mut st = self.lock();
        if st.list.is_loading() {
            st.hooks.push(Box::new(hook));
        } else {
            drop(st);
            hook();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn voice(id: &str) -> Voice {
        Voice {
            id: id.into(),
            name: id.into(),
            ..Voice::default()
        }
    }

    #[test]
    fn a_background_list_arrives_and_runs_its_hooks_once() {
        let (go_tx, go_rx) = std::sync::mpsc::channel::<()>();
        let cache = VoiceCache::spawn("test-voices", move || {
            let _ = go_rx.recv();
            Ok(vec![voice("a"), voice("b")])
        });
        assert_eq!(cache.get(), VoiceList::Loading);
        let calls = Arc::new(AtomicU32::new(0));
        let c = Arc::clone(&calls);
        cache.on_ready(move || {
            c.fetch_add(1, Ordering::SeqCst);
        });
        go_tx.send(()).unwrap();
        let VoiceList::Ready(v) = cache.wait(Duration::from_secs(10)) else {
            panic!("the list arrives");
        };
        assert_eq!(v.len(), 2);
        // The hook runs on the filling thread, just after the list is set.
        let deadline = Instant::now() + Duration::from_secs(10);
        while calls.load(Ordering::SeqCst) == 0 && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        // Later hooks run at once; a second answer is ignored.
        let c = Arc::clone(&calls);
        cache.on_ready(move || {
            c.fetch_add(1, Ordering::SeqCst);
        });
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        cache.set(Ok(Vec::new()));
        assert!(matches!(cache.get(), VoiceList::Ready(v) if v.len() == 2));
    }

    #[test]
    fn failures_are_kept_too() {
        let cache = VoiceCache::ready(Err(SpeechError::Engine("no registry".into())));
        assert_eq!(
            cache.get().into_result(),
            Err(SpeechError::Engine("no registry".into()))
        );
        assert!(VoiceCache::loading().get().into_result().is_err());
        assert_eq!(
            VoiceCache::loading().wait(Duration::from_millis(10)),
            VoiceList::Loading
        );
    }
}
