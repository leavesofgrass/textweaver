//! Optional components in the app (Wave 8, W8a-d and W8a-w): the
//! registry, Manage optional components, the first-run list, and the
//! question a feature asks when it needs a component that is missing.
//!
//! **One registry.** [`Registry::builtin`] lists every optional component
//! textweaver knows: the Whisper dictation models, the OCR model sets
//! (`textweaver_ocr::models`), and Lexend (`textweaver_fonts::downloaded`),
//! each with its title, size, license, credit, the features that need it,
//! and its files pinned by size and SHA-256. The components source's list
//! and a mirror's (`components.toml`, read only when one is set; see
//! `textweaver_components::manifest`) add more components for this
//! computer's platform, never different pins for these. Piper voices are
//! components too, made from the catalogue as each is chosen
//! (`textweaver_piper::download::component`).
//!
//! **Never installed unprompted.** Nothing is downloaded until the reader
//! says yes: in the manager (Download), in the first-run list (Download
//! the chosen ones), or when a feature asks ("Dictation needs the Whisper
//! model, 79.3 MB, MIT license. Download it now?"). A no is remembered for
//! the rest of the session, and the feature says how to get the component
//! later, in words.
//!
//! **Manage optional components** lists each component with whether it is
//! installed, its size, its license, and what it is for. Enter on one
//! offers Download, Verify, Remove, and Install from a zip file or a
//! folder (checked against the same pins; anything else is refused, with
//! the reason). Downloads run on a helper thread; progress is said every
//! 10 percent, and Escape stops one (the next download goes on from
//! there). When one finishes, the feature sees it at once: no restart.
//!
//! **The first-run list** (like abax): the same components, nothing
//! chosen, each naming the feature it enables; Space or Enter chooses,
//! Download the chosen ones gets them, Escape or Skip for now closes it.
//! It is shown once, by the GUI and the terminal reader; `tw` never shows
//! it.

use std::borrow::Cow;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};

use textweaver_a11y::{Importance, Priority, Verbosity};
use textweaver_components::credentials::Token;
pub use textweaver_components::{
    Action, Check, Component, ComponentError, Fetched, Fetcher, FilePin, FileState, InstallReport,
    MIRROR_ENV, Outcome, Platform, Progress, SignedInFetcher, Sources, StandardFetcher, Status,
    Tenths, can_download, credentials, fake, sha256_hex, size_text,
};
use textweaver_keymap::ActionId;
use textweaver_lexicon::args;
use textweaver_lexicon::i18n::Catalog;

use crate::app::{App, ListKind};
use crate::command::{Confirm, Effect, PromptPurpose};

/// The default dictation model's id.
pub const WHISPER_BASE_EN: &str = "whisper-base.en";

/// The larger English dictation model's id.
pub const WHISPER_SMALL_EN: &str = "whisper-small.en";

/// The Whisper models' license: OpenAI's Whisper is MIT; the onnx-community
/// exports declare none, so it is unconfirmed (the owner's note).
const WHISPER_LICENSE: &str = "MIT, unconfirmed";

/// `tokenizer.json`, the same file in both English models.
const WHISPER_TOKENIZER: (&str, u64, &str) = (
    "tokenizer.json",
    2_405_679,
    "5eb60cec1e77aeeb6869a2bb5a8e01a84c3fe5d072d75369343021fe6f5310d0",
);

/// A Whisper model in RTen's layout: the int8 encoder, the merged int8
/// decoder, and the tokenizer, from onnx-community at a fixed revision,
/// kept in `whisper/rten/<name>` (where dictation looks).
fn whisper(
    id: &'static str,
    name: &'static str,
    revision: &'static str,
    title: &'static str,
    encoder: (u64, &'static str),
    decoder: (u64, &'static str),
) -> Component {
    let url = |path: &str| {
        format!("https://huggingface.co/onnx-community/whisper-{name}/resolve/{revision}/{path}")
    };
    let pin = |file: &'static str, path: &str, (size, sha256): (u64, &'static str)| FilePin {
        name: Cow::Borrowed(file),
        url: Cow::Owned(url(path)),
        size,
        check: textweaver_components::Check::Sha256(Cow::Borrowed(sha256)),
    };
    Component {
        id: Cow::Borrowed(id),
        title: Cow::Borrowed(title),
        license: Cow::Borrowed(WHISPER_LICENSE),
        credit: Cow::Owned(format!(
            "OpenAI Whisper {name} (MIT), exported to ONNX by onnx-community, https://huggingface.co/onnx-community/whisper-{name}"
        )),
        features: Cow::Borrowed(&[Cow::Borrowed("dictation")]),
        folder: Cow::Owned(format!("whisper/rten/{name}")),
        files: Cow::Owned(vec![
            pin(
                "encoder_model_int8.onnx",
                "onnx/encoder_model_int8.onnx",
                encoder,
            ),
            pin(
                "decoder_model_merged_int8.onnx",
                "onnx/decoder_model_merged_int8.onnx",
                decoder,
            ),
            pin(
                WHISPER_TOKENIZER.0,
                "tokenizer.json",
                (WHISPER_TOKENIZER.1, WHISPER_TOKENIZER.2),
            ),
        ]),
        notice: None,
        listing: None,
    }
}

/// Whisper base.en, the default dictation model (the owner's pins,
/// Thursday, October 1, 2026).
pub fn whisper_base_en() -> Component {
    whisper(
        WHISPER_BASE_EN,
        "base.en",
        "51eefc0af78b103839eda9e7e4f4186acc6517fe",
        "Whisper base.en, English dictation",
        (
            23_201_297,
            "3d4c91680a8050bc13b8fd0f9acc9deb19f6e535c83bbf58f13b2c7bbcdf8e8d",
        ),
        (
            53_692_803,
            "dd4761a3f7add26afda3512abff4706920404c2517e85a9f2ff090b0c0987909",
        ),
    )
}

/// Whisper small.en, the larger, slower, more accurate English model.
pub fn whisper_small_en() -> Component {
    whisper(
        WHISPER_SMALL_EN,
        "small.en",
        "482fb8ba081b6e906f92efe103622316b2a0cc69",
        "Whisper small.en, better English dictation",
        (
            92_326_127,
            "0a143c26b5aa5f549bef89a9363a56a5610a00985afe1e56443a71852bd642d4",
        ),
        (
            156_750_076,
            "29132c41903c252d0f2b9e59ee1e7d75f7d0b64d37f3e36d1ecc4a1a1af62953",
        ),
    )
}

/// The dictation models offered, in order.
pub const DICTATION_MODELS: [&str; 2] = [WHISPER_BASE_EN, WHISPER_SMALL_EN];

/// Every optional component textweaver knows, and those a mirror added.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Registry {
    list: Vec<Component>,
    builtin: usize,
}

impl Default for Registry {
    fn default() -> Self {
        Registry::builtin()
    }
}

impl Registry {
    /// The built-in components: the Whisper models, the OCR model sets,
    /// and the downloadable reading fonts.
    pub fn builtin() -> Self {
        let mut list = vec![whisper_base_en(), whisper_small_en()];
        list.extend(textweaver_ocr::models::ALL.iter().map(|s| s.component()));
        list.extend(
            textweaver_fonts::downloaded::DOWNLOADABLE
                .iter()
                .map(|f| f.component()),
        );
        let builtin = list.len();
        Registry { list, builtin }
    }

    /// Adds the components of a mirror's manifest (its text). Returns the
    /// ids refused, each with the reason; a built-in id is always refused.
    pub fn add_manifest(&mut self, text: &str) -> Result<Vec<(String, String)>, ComponentError> {
        let known: Vec<String> = self.list.iter().map(|c| c.id.to_string()).collect();
        let parsed =
            textweaver_components::manifest::parse(text, &|id| known.iter().any(|k| k == id))?;
        self.list.extend(parsed.components);
        Ok(parsed.refused)
    }

    /// Every component, built-in ones first.
    pub fn components(&self) -> &[Component] {
        &self.list
    }

    /// The component `id`.
    pub fn get(&self, id: &str) -> Option<&Component> {
        self.list.iter().find(|c| c.id == id)
    }

    /// True for a built-in component.
    pub fn is_builtin(&self, id: &str) -> bool {
        self.list[..self.builtin].iter().any(|c| c.id == id)
    }

    /// The one components status line for `data_dir`, as
    /// [`status_line`] says it.
    pub fn status_line(&self, c: &Catalog, data_dir: &Path) -> String {
        status_line(c, self.installed_count(data_dir))
    }

    /// How many are installed under `data_dir`, and how many are known.
    pub fn installed_count(&self, data_dir: &Path) -> (usize, usize) {
        let n = self
            .list
            .iter()
            .filter(|c| c.status_in(&component_dir(c, data_dir)) == Status::Installed)
            .count();
        (n, self.list.len())
    }
}

/// Where `component` is installed: its folder under `data_dir`, except
/// the OCR models when `TEXTWEAVER_OCR_MODELS` names their folder (OCR
/// reads them there).
pub fn component_dir(component: &Component, data_dir: &Path) -> PathBuf {
    if component.id.starts_with("ocr-")
        && let Some(flat) = textweaver_ocr::models::flat_dir()
    {
        return flat;
    }
    component.dir_in(data_dir)
}

/// The one components status line, "Components: 3 of 12 installed",
/// from `(installed, known)`. About in both frontends, `tw info`, `tw
/// components list`, and the doctor scripts (through `tw components
/// list`) all say it this way (W9a-c).
pub fn status_line(c: &Catalog, (installed, known): (usize, usize)) -> String {
    c.fmt(
        "about-components",
        &args!["installed" => installed, "known" => known],
    )
}

/// Where components come from with these settings: the components
/// source (`[components] source`, a repository or a folder), then the
/// mirror (`TEXTWEAVER_COMPONENTS_MIRROR`, else `[components] mirror`),
/// then the public addresses.
pub fn sources(settings: &textweaver_store::Settings) -> Sources {
    Sources::from_env_or(&settings.components.mirror).with_source(&settings.components.source)
}

/// The OCR model sets, each a component in the registry.
pub fn ocr_sets() -> &'static [textweaver_ocr::ModelSet] {
    &textweaver_ocr::models::ALL
}

/// Downloads a component (the shared downloader), for `tw`.
pub use textweaver_components::download as download_component;

/// Installs a component from a zip or a folder (checked against its
/// pins), for `tw`.
pub use textweaver_components::install_from as install_component;

/// Reads a small text file at `address` (a mirror's manifest).
pub fn fetch_text(fetcher: &dyn Fetcher, address: &str) -> Result<String, String> {
    let bytes = textweaver_components::fetch_bytes(
        fetcher,
        address,
        textweaver_components::manifest::MAX_BYTES,
    )?;
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

/// The dictation model chosen in settings, or base.en.
pub fn dictation_model_id(settings: &textweaver_store::Settings) -> &str {
    let m = settings.dictation.model.trim();
    if m.is_empty() { WHISPER_BASE_EN } else { m }
}

/// The features a component enables, in words ("dictation, reading
/// scanned pages").
pub fn features_text(c: &Catalog, component: &Component) -> String {
    let names: Vec<String> = component
        .features
        .iter()
        .map(|f| {
            let id = format!("component-feature-{f}");
            let t = c.tr(&id);
            if t == id { f.to_string() } else { t }
        })
        .collect();
    names.join(", ")
}

/// The one-line reason for a failed download or install, in words, 40
/// cells or fewer.
pub fn error_text(c: &Catalog, e: &ComponentError) -> String {
    c.tr(match e {
        ComponentError::Fetch { .. } => "component-error-fetch",
        ComponentError::Size { .. } => "component-error-size",
        ComponentError::Hash { .. } => "component-error-hash",
        ComponentError::Missing { .. } => "component-error-missing",
        ComponentError::Cancelled => "component-error-cancelled",
        ComponentError::Busy(_) => "component-error-busy",
        ComponentError::NoSource { .. } => "component-error-no-source",
        ComponentError::BadName(_) => "component-error-name",
        ComponentError::Manifest(_) => "component-error-manifest",
        ComponentError::Io { .. } => "component-error-io",
    })
}

/// What happens when a download finishes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum After {
    /// Nothing more: say it is ready.
    Nothing,
    /// Start dictating (the dictation question's yes).
    Dictate,
}

/// A yes-or-no question about a component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Question {
    /// Download this component, then do this.
    Download(String, After),
    /// Remove this component's files.
    Remove(String),
}

/// What a helper thread does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum JobKind {
    Download,
    Install,
    Verify,
}

/// What a helper thread finished with.
enum JobDone {
    Fetched,
    Installed(InstallReport),
    Verified(Vec<(String, FileState)>),
}

/// Work on a helper thread.
struct Job {
    id: String,
    kind: JobKind,
    rx: Receiver<Result<JobDone, ComponentError>>,
    done: Arc<AtomicU64>,
    total: u64,
    tenths: Tenths,
    cancel: Arc<AtomicBool>,
    then: After,
}

/// The rows of a components list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ComponentsList {
    /// Manage optional components: one row per component.
    Manager(Vec<String>),
    /// What to do with one component.
    Actions(String),
    /// The first-run list: one row per component, then Download the
    /// chosen ones and Skip for now.
    Chooser(Vec<String>),
}

/// The actions offered for one component, in order.
const ACTIONS: [&str; 5] = [
    "components-action-download",
    "components-action-verify",
    "components-action-remove",
    "components-action-install-zip",
    "components-action-install-folder",
];

/// The app's components state.
#[derive(Default)]
pub(crate) struct ComponentsState {
    /// Set by tests and frontends; [`StandardFetcher`] otherwise.
    fetcher: Option<Arc<dyn Fetcher>>,
    /// The registry, with a mirror's components once read.
    registry: Option<Registry>,
    /// The mirror whose manifest was read (or tried).
    manifest_for: Option<String>,
    /// The question waiting for y or n.
    pub(crate) question: Option<Question>,
    /// A download, install, or verify on a helper thread.
    job: Option<Job>,
    /// More downloads to start, in order (the first-run list's choice).
    queue: Vec<String>,
    /// Components the reader said no to this session: not asked again.
    declined: HashSet<String>,
    /// The component an Install from a file is for.
    target: Option<String>,
    /// The first-run list's choices, by row.
    chosen: Vec<bool>,
    /// A frontend asked for the first-run list (its first run).
    first_run: bool,
    /// Moves on when a component was installed, removed, or verified.
    generation: u64,
    /// The token for a private components source, once looked up this
    /// session (`Some(None)`: there is none). Never printed.
    token: Option<Option<Token>>,
    /// The token was asked for this session (asked once).
    token_asked: bool,
    /// Tests: the fake GitHub API the signed-in fetcher goes to.
    #[cfg(test)]
    github_api: Option<String>,
}

impl std::fmt::Debug for ComponentsState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComponentsState")
            .field("question", &self.question)
            .field("job", &self.job.as_ref().map(|j| (&j.id, j.kind)))
            .field("queue", &self.queue)
            .field("declined", &self.declined)
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

/// Gives the commands their handlers.
pub(crate) fn register(app: &mut App) {
    app.register_handler(ActionId::ManageComponents, |app| app.manage_components());
    app.register_handler(ActionId::ForgetGitHubToken, |app| {
        app.forget_components_token()
    });
}

impl App {
    // ----- Setup -------------------------------------------------------

    /// Fetches components through `fetcher` from now on (tests pass a fake
    /// one; the default reads folders and, with the `publish` feature,
    /// goes to the network).
    pub fn set_component_fetcher(&mut self, fetcher: Arc<dyn Fetcher>) {
        self.components.fetcher = Some(fetcher);
    }

    /// Moves on each time a component was installed, removed, or
    /// verified, so a frontend knows to look again (the GUI registers a
    /// downloaded font).
    pub fn component_changes(&self) -> u64 {
        self.components.generation
    }

    /// Asks for the first-run list of optional components: the GUI and
    /// the terminal reader call it on their first run. It is shown once
    /// (`[components] chooser_shown`), when nothing else is open.
    pub fn offer_components_on_first_run(&mut self) {
        self.components.first_run = true;
    }

    /// The registry: the built-in components, and a mirror's once read.
    pub fn component_registry(&mut self) -> &Registry {
        self.components
            .registry
            .get_or_insert_with(Registry::builtin)
    }

    /// The data folder, when this session keeps files.
    fn data_dir(&self) -> Option<PathBuf> {
        self.paths.as_ref().map(|p| p.data_dir.clone())
    }

    /// The component `id` and its folder.
    pub(crate) fn component_and_dir(&mut self, id: &str) -> Option<(Component, PathBuf)> {
        let data = self.data_dir()?;
        if self.component_registry().get(id).is_none() {
            // Not built in: perhaps the mirror's (read once, only when a
            // mirror is set).
            self.load_mirror_manifest();
        }
        let c = self.component_registry().get(id)?.clone();
        let dir = component_dir(&c, &data);
        Some((c, dir))
    }

    /// The fetcher to use, when downloads can happen in this build: a
    /// test's; else, for a components source on GitHub with a token, the
    /// signed-in one; else the standard one with the `publish` feature or
    /// a mirror that is a folder on this computer.
    fn component_fetcher(&mut self) -> Option<Arc<dyn Fetcher>> {
        if let Some(f) = &self.components.fetcher {
            return Some(Arc::clone(f));
        }
        let sources = sources(&self.settings);
        if can_download()
            && sources.source_is_github()
            && let Some(token) = self.components_token()
        {
            let f = SignedInFetcher::new(token);
            #[cfg(test)]
            let f = match &self.components.github_api {
                Some(api) => f.with_api(api),
                None => f,
            };
            return Some(Arc::new(f));
        }
        let local = sources.has_local();
        (can_download() || local).then(|| Arc::new(StandardFetcher) as Arc<dyn Fetcher>)
    }

    // ----- Signing in to a private source (B1-c2) ----------------------

    /// The token for a private components source: the GitHub CLI's when
    /// it is signed in, else the one in the system credential store.
    /// Looked up once a session.
    fn components_token(&mut self) -> Option<Token> {
        if self.components.token.is_none() {
            self.components.token = Some(credentials::find_token().map(|(t, _)| t));
        }
        self.components.token.clone().flatten()
    }

    /// True when Manage optional components asks for a token first: the
    /// source is a GitHub repository, this build downloads, no token is
    /// known, and it was not asked this session.
    fn wants_components_token(&mut self) -> bool {
        can_download()
            && !self.components.token_asked
            && sources(&self.settings).source_is_github()
            && self.components_token().is_none()
    }

    /// The answer to the token prompt: a token is kept in the system
    /// credential store (and used this session even when the store
    /// refuses it); empty skips (a public repository needs none). Then the
    /// manager opens. Nothing said here holds the token.
    pub(crate) fn answer_github_token(&mut self, text: &str) -> Vec<Effect> {
        if !text.trim().is_empty() {
            let Some(token) = Token::new(text) else {
                let msg = self.msg("components-token-invalid");
                self.error(&msg);
                return self.prompt(PromptPurpose::GitHubToken);
            };
            match credentials::store_token(&token) {
                Ok(()) => {
                    let msg = self.msg("components-token-kept");
                    self.note(&msg);
                }
                Err(reason) => {
                    let msg =
                        self.msg_args("components-token-not-kept", &args!["reason" => reason]);
                    self.error(&msg);
                }
            }
            self.components.token = Some(Some(token));
            // Read the source's list again, signed in.
            self.components.manifest_for = None;
        }
        self.manage_components()
    }

    /// Forget the GitHub token: removes it from the system credential
    /// store, and asks again the next time Manage optional components
    /// needs one. The GitHub CLI's own sign-in is kept, and said.
    pub(crate) fn forget_components_token(&mut self) -> Vec<Effect> {
        let key = match credentials::forget_token() {
            Ok(true) => "components-token-forgotten",
            Ok(false) => "components-token-none",
            Err(reason) => {
                let msg =
                    self.msg_args("components-token-not-forgotten", &args!["reason" => reason]);
                self.error(&msg);
                return vec![Effect::Redraw];
            }
        };
        self.components.token = None;
        self.components.token_asked = false;
        self.components.manifest_for = None;
        let mut msg = self.msg(key);
        if credentials::gh_token().is_some() {
            msg = format!("{msg} {}", self.msg("components-token-gh-still"));
        }
        self.note(&msg);
        vec![Effect::Redraw]
    }

    /// Reads the components lists (`components.toml`) of the source and
    /// the mirror once per setting, when either is set: the source's
    /// first, so its components win.
    fn load_mirror_manifest(&mut self) {
        let addresses = sources(&self.settings).manifest_addresses();
        if addresses.is_empty() {
            return;
        }
        let key = addresses.join("\n");
        if self.components.manifest_for.as_deref() == Some(key.as_str()) {
            return;
        }
        self.components.manifest_for = Some(key);
        let Some(fetcher) = self.component_fetcher() else {
            return;
        };
        let mut registry = Registry::builtin();
        for address in addresses {
            match fetch_text(&*fetcher, &address).map(|t| registry.add_manifest(&t)) {
                Ok(Ok(refused)) => {
                    for (id, why) in refused {
                        log::warn!("component {id} in {address} refused: {why}");
                    }
                }
                Ok(Err(e)) => log::warn!("components list {address}: {e}"),
                Err(e) => log::info!("no components list at {address}: {e}"),
            }
        }
        self.components.registry = Some(registry);
    }

    // ----- The manager -------------------------------------------------

    /// Manage optional components: the list, with each one's state, size,
    /// license, and what it is for.
    pub(crate) fn manage_components(&mut self) -> Vec<Effect> {
        if self.data_dir().is_none() {
            let msg = self.msg("component-no-folder");
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        if self.wants_components_token() {
            // Asked once a session; Enter skips.
            self.components.token_asked = true;
            return self.prompt(PromptPurpose::GitHubToken);
        }
        self.load_mirror_manifest();
        self.show_component_manager(None)
    }

    fn component_state(&mut self, c: &Component) -> String {
        let Some(data) = self.data_dir() else {
            return self.msg("component-state-not-installed");
        };
        let busy = self.components.job.as_ref().is_some_and(|j| j.id == c.id);
        let key = if busy {
            "component-state-downloading"
        } else {
            match c.status_in(&component_dir(c, &data)) {
                Status::Installed => "component-state-installed",
                Status::NotInstalled => "component-state-not-installed",
                Status::Partial(_) => "component-state-partial",
                Status::Damaged(_) => "component-state-damaged",
            }
        };
        self.msg(key)
    }

    fn show_component_manager(&mut self, focus: Option<usize>) -> Vec<Effect> {
        let list: Vec<Component> = self.component_registry().components().to_vec();
        let mut items = Vec::with_capacity(list.len());
        for c in &list {
            let state = self.component_state(c);
            let features = features_text(self.cat(), c);
            items.push(self.msg_args(
                "components-item",
                &args![
                    "title" => c.title.as_ref(),
                    "state" => state,
                    "size" => c.size_text(),
                    "license" => c.license.as_ref(),
                    "features" => features
                ],
            ));
        }
        let ids = list.iter().map(|c| c.id.to_string()).collect();
        self.list = Some(ListKind::Components(ComponentsList::Manager(ids)));
        if let Some(n) = focus {
            self.pending_list_focus = Some(n);
        } else {
            let msg = self.msg_args("components-intro", &args!["n" => items.len()]);
            self.say_result(&msg);
        }
        vec![Effect::ShowList {
            title: self.msg("components-title"),
            items,
        }]
    }

    fn show_component_actions(&mut self, id: &str) -> Vec<Effect> {
        let Some(c) = self.component_registry().get(id).cloned() else {
            return vec![Effect::Redraw];
        };
        let items: Vec<String> = ACTIONS
            .iter()
            .map(|a| self.msg_args(a, &args!["size" => c.size_text()]))
            .collect();
        self.list = Some(ListKind::Components(ComponentsList::Actions(id.to_owned())));
        let state = self.component_state(&c);
        let msg = self.msg_args(
            "components-actions-intro",
            &args!["title" => c.title.as_ref(), "state" => state],
        );
        self.say_result(&msg);
        vec![Effect::ShowList {
            title: c.title.to_string(),
            items,
        }]
    }

    /// Enter in a components list.
    pub(crate) fn choose_component_row(&mut self, list: ComponentsList, n: usize) -> Vec<Effect> {
        match list {
            ComponentsList::Manager(ids) => match ids.get(n) {
                Some(id) => self.show_component_actions(id),
                None => vec![Effect::Redraw],
            },
            ComponentsList::Actions(id) => match n {
                0 => self.ask_component_download(&id, After::Nothing),
                1 => self.verify_component(&id),
                2 => self.ask_component_remove(&id),
                3 | 4 => {
                    self.components.target = Some(id);
                    let purpose = self.msg("components-install-purpose");
                    if n == 3 {
                        self.choose_file(&purpose, &["zip"], |app, path| {
                            app.install_component_from(path)
                        })
                    } else {
                        self.choose_folder(&purpose, |app, path| app.install_component_from(path))
                    }
                }
                _ => vec![Effect::Redraw],
            },
            ComponentsList::Chooser(ids) => self.choose_in_chooser(ids, n),
        }
    }

    /// Delete on a row of the manager: asks before removing.
    pub(crate) fn delete_component_row(&mut self, list: ComponentsList, n: usize) -> Vec<Effect> {
        let id = match list {
            ComponentsList::Manager(ids) => ids.get(n).cloned(),
            ComponentsList::Actions(id) => Some(id),
            ComponentsList::Chooser(_) => None,
        };
        match id {
            Some(id) => self.ask_component_remove(&id),
            None => vec![Effect::Redraw],
        }
    }

    // ----- Questions ---------------------------------------------------

    /// Asks before downloading `id` (its size and license said). Says why
    /// when it cannot be downloaded here, or is already installed.
    pub(crate) fn ask_component_download(&mut self, id: &str, then: After) -> Vec<Effect> {
        self.list = None;
        let Some((c, dir)) = self.component_and_dir(id) else {
            let msg = self.msg("component-no-folder");
            self.tell(&msg);
            return vec![Effect::Redraw];
        };
        if self.components.job.is_some() {
            let msg = self.msg("component-error-busy");
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        if then == After::Nothing && c.status_in(&dir) == Status::Installed {
            let msg = self.msg_args("component-already", &args!["title" => c.title.as_ref()]);
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        if self.component_fetcher().is_none() {
            let msg = self.msg(if then == After::Dictate {
                "dictation-model-not-in-build"
            } else {
                "component-not-in-build"
            });
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        let question = self.component_question(&c, then);
        self.components.question = Some(Question::Download(id.to_owned(), then));
        self.ask(&question);
        vec![Effect::Redraw]
    }

    fn component_question(&self, c: &Component, then: After) -> String {
        let key = match then {
            After::Dictate => "dictation-model-question",
            After::Nothing => "component-question",
        };
        self.msg_args(
            key,
            &args![
                "title" => c.title.as_ref(),
                "size" => c.size_text(),
                "license" => c.license.as_ref()
            ],
        )
    }

    fn ask_component_remove(&mut self, id: &str) -> Vec<Effect> {
        self.list = None;
        let Some((c, dir)) = self.component_and_dir(id) else {
            return vec![Effect::Redraw];
        };
        if c.status_in(&dir) == Status::NotInstalled {
            let msg = self.msg_args("component-not-there", &args!["title" => c.title.as_ref()]);
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        self.components.question = Some(Question::Remove(id.to_owned()));
        let q = self.msg_args(
            "component-remove-question",
            &args!["title" => c.title.as_ref()],
        );
        self.ask(&q);
        vec![Effect::Redraw]
    }

    /// The answer to a components question.
    pub(crate) fn confirm_component(&mut self, answer: Confirm) -> Vec<Effect> {
        let Some(q) = self.components.question.take() else {
            return vec![Effect::Redraw];
        };
        match (answer, q) {
            (Confirm::Repeat, q) => {
                let text = match &q {
                    Question::Download(id, then) => {
                        let c = self.component_registry().get(id).cloned();
                        c.map(|c| self.component_question(&c, *then))
                    }
                    Question::Remove(id) => {
                        let c = self.component_registry().get(id).cloned();
                        c.map(|c| {
                            self.msg_args(
                                "component-remove-question",
                                &args!["title" => c.title.as_ref()],
                            )
                        })
                    }
                };
                self.components.question = Some(q);
                if let Some(t) = text {
                    self.ask(&t);
                }
            }
            (Confirm::No, Question::Download(id, then)) => {
                self.components.declined.insert(id);
                let key = match then {
                    After::Dictate => "dictation-model-declined",
                    After::Nothing => "component-declined",
                };
                let msg = self.msg(key);
                self.tell(&msg);
            }
            (Confirm::No, Question::Remove(_)) => {
                let msg = self.msg("common-kept");
                self.tell(&msg);
            }
            (Confirm::Yes, Question::Download(id, then)) => {
                self.components.declined.remove(&id);
                self.start_component_job(&id, JobKind::Download, None, then);
            }
            (Confirm::Yes, Question::Remove(id)) => self.remove_component(&id),
        }
        vec![Effect::Redraw]
    }

    /// True when the reader said no to `id` earlier this session.
    #[cfg_attr(not(feature = "dictation"), allow(dead_code))]
    pub(crate) fn component_declined(&self, id: &str) -> bool {
        self.components.declined.contains(id)
    }

    // ----- Work --------------------------------------------------------

    fn start_component_job(&mut self, id: &str, kind: JobKind, from: Option<PathBuf>, then: After) {
        let Some((c, dir)) = self.component_and_dir(id) else {
            let msg = self.msg("component-no-folder");
            self.error(&msg);
            return;
        };
        let fetcher = match kind {
            JobKind::Download => match self.component_fetcher() {
                Some(f) => f,
                None => {
                    let msg = self.msg("component-not-in-build");
                    self.tell(&msg);
                    return;
                }
            },
            JobKind::Install | JobKind::Verify => Arc::new(StandardFetcher),
        };
        let sources = sources(&self.settings);
        let (tx, rx) = std::sync::mpsc::channel();
        let done = Arc::new(AtomicU64::new(0));
        let cancel = Arc::new(AtomicBool::new(false));
        let wake = self.waker_slot();
        let (done2, cancel2, c2, dir2) = (done.clone(), cancel.clone(), c.clone(), dir.clone());
        let spawned = std::thread::Builder::new()
            .name("textweaver-component".into())
            .spawn(move || {
                let result = match kind {
                    JobKind::Download => textweaver_components::download(
                        &c2,
                        &dir2,
                        &sources,
                        &*fetcher,
                        &mut |p| {
                            done2.store(p.done, Ordering::Relaxed);
                            wake.wake();
                        },
                        &cancel2,
                    )
                    .map(|_| JobDone::Fetched),
                    JobKind::Install => match from {
                        Some(path) => textweaver_components::install_from(&c2, &path, &dir2)
                            .map(JobDone::Installed),
                        None => Err(ComponentError::Missing {
                            file: c2.id.to_string(),
                            from: PathBuf::new(),
                        }),
                    },
                    JobKind::Verify => Ok(JobDone::Verified(c2.verify_in(&dir2))),
                };
                let _ = tx.send(result);
                wake.wake();
            });
        if spawned.is_err() {
            let msg = self.msg("component-error-io");
            self.error(&msg);
            return;
        }
        self.components.job = Some(Job {
            id: id.to_owned(),
            kind,
            rx,
            done,
            total: c.size(),
            tenths: Tenths::default(),
            cancel,
            then,
        });
        let key = match kind {
            JobKind::Download => "component-downloading",
            JobKind::Install => "component-installing",
            JobKind::Verify => "component-verifying",
        };
        let msg = self.msg_args(key, &args!["title" => c.title.as_ref()]);
        self.tell(&msg);
    }

    /// True while a component downloads, installs, or is checked.
    pub fn component_job_running(&self) -> bool {
        self.components.job.is_some()
    }

    /// Escape while a component downloads: stops it at once (the next
    /// download goes on from where it stopped).
    pub(crate) fn cancel_component_job(&mut self) -> Vec<Effect> {
        if let Some(job) = &self.components.job {
            job.cancel.store(true, Ordering::Relaxed);
        }
        self.components.queue.clear();
        vec![Effect::Redraw]
    }

    fn install_component_from(&mut self, path: PathBuf) -> Vec<Effect> {
        let Some(id) = self.components.target.take() else {
            return vec![Effect::Redraw];
        };
        if self.components.job.is_some() {
            let msg = self.msg("component-error-busy");
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        self.start_component_job(&id, JobKind::Install, Some(path), After::Nothing);
        vec![Effect::Redraw]
    }

    fn verify_component(&mut self, id: &str) -> Vec<Effect> {
        self.list = None;
        if self.components.job.is_some() {
            let msg = self.msg("component-error-busy");
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        self.start_component_job(id, JobKind::Verify, None, After::Nothing);
        vec![Effect::Redraw]
    }

    fn remove_component(&mut self, id: &str) {
        let Some((c, dir)) = self.component_and_dir(id) else {
            return;
        };
        if self.components.job.as_ref().is_some_and(|j| j.id == id) {
            let msg = self.msg("component-error-busy");
            self.tell(&msg);
            return;
        }
        match c.remove_in(&dir) {
            Ok(n) => {
                log::info!("removed {n} files of {id} from {}", dir.display());
                self.component_changed(id);
                let msg = self.msg_args(
                    "component-removed",
                    &args!["title" => c.title.as_ref(), "n" => n],
                );
                self.tell(&msg);
            }
            Err(e) => {
                log::warn!("removing {id}: {e}");
                let msg = error_text(self.cat(), &e);
                self.error(&msg);
            }
        }
    }

    /// Every feature sees a change at once: the dictation backend is
    /// dropped (it loads the model again, or asks), the font code looks
    /// again, and the GUI is told through the generation.
    fn component_changed(&mut self, id: &str) {
        self.components.generation += 1;
        self.dictation_component_changed(id);
        if textweaver_fonts::downloaded::downloadable(id).is_some() {
            self.font_component_changed();
        }
    }

    /// From [`App::tick`]: progress every 10 percent, a finished job, the
    /// next queued download, and the first-run list when it is due.
    pub(crate) fn components_tick(&mut self) -> Vec<Effect> {
        let mut effects = Vec::new();
        if self.components.first_run
            && !self.settings.components.chooser_shown
            && self.components.job.is_none()
            && self.list.is_none()
            && !self.mode.is_prompt()
            && !self.confirmation_pending()
            && self.paths.is_some()
        {
            self.components.first_run = false;
            effects.extend(self.show_components_chooser());
            return effects;
        }
        let Some(mut job) = self.components.job.take() else {
            return effects;
        };
        if job.kind == JobKind::Download {
            let p = Progress {
                done: job.done.load(Ordering::Relaxed),
                total: job.total,
            };
            if let Some(percent) = job.tenths.step(p)
                && percent < 100
            {
                let msg = self.msg_args("component-progress", &args!["percent" => percent]);
                self.say_kind(
                    &msg,
                    Verbosity::Normal,
                    Priority::Polite,
                    Importance::Progress,
                );
            }
        }
        match job.rx.try_recv() {
            Err(TryRecvError::Empty) => {
                self.components.job = Some(job);
                return effects;
            }
            Err(TryRecvError::Disconnected) => {
                let msg = self.msg("component-error-io");
                self.error(&msg);
            }
            Ok(result) => effects.extend(self.component_job_done(&job, result)),
        }
        if self.components.job.is_none()
            && let Some(next) =
                (!self.components.queue.is_empty()).then(|| self.components.queue.remove(0))
        {
            self.start_component_job(&next, JobKind::Download, None, After::Nothing);
        }
        effects.push(Effect::Redraw);
        effects
    }

    fn component_job_done(
        &mut self,
        job: &Job,
        result: Result<JobDone, ComponentError>,
    ) -> Vec<Effect> {
        let title = self
            .component_registry()
            .get(&job.id)
            .map_or_else(|| job.id.clone(), |c| c.title.to_string());
        match result {
            Ok(JobDone::Fetched) => self.component_ready(job, &title),
            Ok(JobDone::Installed(report)) => {
                let effects = self.component_ready(job, &title);
                if !report.refused.is_empty() {
                    let names: Vec<String> = report
                        .refused
                        .iter()
                        .map(|(n, why)| format!("{n}, {why}"))
                        .collect();
                    let msg = self.msg_args(
                        "component-refused",
                        &args!["n" => names.len(), "files" => names.join("; ")],
                    );
                    self.tell(&msg);
                }
                effects
            }
            Ok(JobDone::Verified(states)) => {
                self.components.generation += 1;
                let bad: Vec<String> = states
                    .iter()
                    .filter(|(_, s)| *s != FileState::Good)
                    .map(|(n, _)| n.clone())
                    .collect();
                let msg = if bad.is_empty() {
                    self.msg_args("component-verified", &args!["title" => title])
                } else {
                    self.msg_args(
                        "component-verify-failed",
                        &args!["n" => bad.len(), "files" => bad.join(", ")],
                    )
                };
                self.tell(&msg);
                Vec::new()
            }
            Err(ComponentError::Cancelled) => {
                let msg = self.msg("component-error-cancelled");
                self.tell(&msg);
                Vec::new()
            }
            Err(e) => {
                log::warn!("{}: {e}", job.id);
                let msg = error_text(self.cat(), &e);
                self.error(&msg);
                let detail = e.to_string();
                self.say_kind(
                    &detail,
                    Verbosity::Normal,
                    Priority::Polite,
                    Importance::Detail,
                );
                Vec::new()
            }
        }
    }

    /// A download or install finished: every feature sees it, the reader
    /// hears it, and dictation starts when that was the reason.
    fn component_ready(&mut self, job: &Job, title: &str) -> Vec<Effect> {
        self.component_changed(&job.id);
        let msg = self.msg_args("component-ready", &args!["title" => title]);
        self.tell(&msg);
        if job.then == After::Dictate {
            return self.dictate_after_download();
        }
        Vec::new()
    }

    // ----- The first-run list ------------------------------------------

    /// The first-run list: every component, nothing chosen.
    pub(crate) fn show_components_chooser(&mut self) -> Vec<Effect> {
        let ids: Vec<String> = self
            .component_registry()
            .components()
            .iter()
            .map(|c| c.id.to_string())
            .collect();
        self.components.chosen = vec![false; ids.len()];
        let _ = self.update_settings(|s| s.components.chooser_shown = true);
        let msg = self.msg("components-chooser-intro");
        self.say_result(&msg);
        self.chooser_list(ids, None)
    }

    fn chooser_list(&mut self, ids: Vec<String>, focus: Option<usize>) -> Vec<Effect> {
        let mut items = Vec::with_capacity(ids.len() + 2);
        for (i, id) in ids.iter().enumerate() {
            let Some(c) = self.component_registry().get(id).cloned() else {
                continue;
            };
            let mark = self.msg(if self.components.chosen.get(i).copied().unwrap_or(false) {
                "components-chosen"
            } else {
                "components-not-chosen"
            });
            let features = features_text(self.cat(), &c);
            items.push(self.msg_args(
                "components-chooser-item",
                &args![
                    "mark" => mark,
                    "title" => c.title.as_ref(),
                    "features" => features,
                    "size" => c.size_text(),
                    "license" => c.license.as_ref()
                ],
            ));
        }
        items.push(self.msg("components-chooser-download"));
        items.push(self.msg("components-chooser-skip"));
        self.list = Some(ListKind::Components(ComponentsList::Chooser(ids)));
        if let Some(n) = focus {
            self.pending_list_focus = Some(n);
        }
        vec![Effect::ShowList {
            title: self.msg("components-chooser-title"),
            items,
        }]
    }

    /// Space in the first-run list: chooses the row or takes it back.
    pub(crate) fn mark_component_row(&mut self, list: ComponentsList, n: usize) -> Vec<Effect> {
        let ComponentsList::Chooser(ids) = list else {
            let msg = self.msg("list-nothing-to-mark");
            self.tell(&msg);
            self.list = Some(ListKind::Components(list));
            return vec![Effect::Redraw];
        };
        if n >= ids.len() {
            self.list = Some(ListKind::Components(ComponentsList::Chooser(ids)));
            return vec![Effect::Redraw];
        }
        let on = !self.components.chosen.get(n).copied().unwrap_or(false);
        if let Some(slot) = self.components.chosen.get_mut(n) {
            *slot = on;
        }
        let msg = self.msg(if on {
            "components-chosen"
        } else {
            "components-not-chosen"
        });
        self.say_result(&msg);
        self.chooser_list(ids, Some(n))
    }

    fn choose_in_chooser(&mut self, ids: Vec<String>, n: usize) -> Vec<Effect> {
        if n < ids.len() {
            return self.mark_component_row(ComponentsList::Chooser(ids), n);
        }
        if n > ids.len() {
            let msg = self.msg("components-chooser-skipped");
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        let chosen: Vec<String> = ids
            .iter()
            .zip(&self.components.chosen)
            .filter(|(_, on)| **on)
            .map(|(id, _)| id.clone())
            .collect();
        if chosen.is_empty() {
            let msg = self.msg("components-chooser-none");
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        if self.component_fetcher().is_none() {
            let msg = self.msg("component-not-in-build");
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        // Choosing them and pressing Download was the yes.
        let mut queue = chosen;
        let first = queue.remove(0);
        self.components.queue = queue;
        self.start_component_job(&first, JobKind::Download, None, After::Nothing);
        vec![Effect::Redraw]
    }

    /// Waits up to `limit` for component work to finish (tests).
    pub fn wait_for_components(&mut self, limit: std::time::Duration) -> bool {
        let start = std::time::Instant::now();
        while self.components.job.is_some() || !self.components.queue.is_empty() {
            if start.elapsed() > limit {
                return false;
            }
            self.components_tick();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        true
    }
}

#[cfg(test)]
mod tests;
