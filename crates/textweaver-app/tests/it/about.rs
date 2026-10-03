//! Help's ways to the docs, About's facts, and asking first-run choices
//! again.

use std::sync::{Arc, Mutex};

use textweaver_app::keymap::ActionId;
use textweaver_app::{App, AppConfig, Command, Confirm, Effect};

/// An app whose opener records every address instead of opening it.
fn rig() -> (App, Arc<Mutex<Vec<String>>>) {
    let mut app = App::new(AppConfig::for_tests());
    let opened = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&opened);
    app.set_launcher(Arc::new(move |target: &str| {
        sink.lock().unwrap().push(target.to_owned());
        Ok(())
    }));
    (app, opened)
}

#[test]
fn report_a_problem_shows_the_address_and_asks_before_any_browser_opens() {
    let (mut app, opened) = rig();
    app.dispatch(Command::Action(ActionId::ReportProblem));
    let status = app.status_text().to_owned();
    assert!(status.starts_with("Report a problem: https://"), "{status}");
    assert!(status.contains("Nothing is sent."), "{status}");
    assert!(app.confirmation_pending());
    assert!(
        opened.lock().unwrap().is_empty(),
        "nothing opens before the answer"
    );
    app.dispatch(Command::Confirm(Confirm::No));
    assert!(opened.lock().unwrap().is_empty(), "no means nothing opens");
    assert!(!app.confirmation_pending());

    app.dispatch(Command::Action(ActionId::ReportProblem));
    app.dispatch(Command::Confirm(Confirm::Yes));
    assert_eq!(
        opened.lock().unwrap().as_slice(),
        ["https://github.com/leavesofgrass/textweaver/issues"]
    );
}

#[test]
fn documentation_asks_before_opening_the_site() {
    let (mut app, opened) = rig();
    app.dispatch(Command::Action(ActionId::Documentation));
    assert!(app.status_text().starts_with("Documentation: https://"));
    app.dispatch(Command::Confirm(Confirm::No));
    assert!(opened.lock().unwrap().is_empty());
    app.dispatch(Command::Action(ActionId::Documentation));
    app.dispatch(Command::Confirm(Confirm::Yes));
    assert_eq!(
        opened.lock().unwrap().as_slice(),
        ["https://leavesofgrass.github.io/textweaver/"]
    );
}

#[test]
fn about_lists_facts_each_starting_with_its_name() {
    let (mut app, _) = rig();
    let effects = app.dispatch(Command::Action(ActionId::About));
    let items = effects
        .iter()
        .find_map(|e| match e {
            Effect::ShowList { items, .. } => Some(items.clone()),
            _ => None,
        })
        .expect("About shows a list");
    assert!(items[0].starts_with("Version: textweaver "), "{items:?}");
    assert!(
        items[1].starts_with("Build: terminal reader, "),
        "{items:?}"
    );
    assert!(items.iter().any(|i| i.starts_with("License: ")));
    assert!(
        items
            .iter()
            .any(|i| i.starts_with("Speech engine in use: "))
    );
    // A session that keeps no files has no folders to report.
    assert!(items.iter().any(|i| i.starts_with("Folders: none")));
    assert!(app.status_text().starts_with("About textweaver, "));
}

#[test]
fn asking_first_run_choices_again_clears_both_markers() {
    let (mut app, _) = rig();
    app.update_settings(|s| {
        s.accessibility.hybrid_offered = true;
        s.components.chooser_shown = true;
    })
    .unwrap();
    app.dispatch(Command::Action(ActionId::AskFirstRunAgain));
    assert!(!app.settings().accessibility.hybrid_offered);
    assert!(!app.settings().components.chooser_shown);
    assert!(app.status_text().starts_with("First-run choices reset"));
    assert!(app.hybrid_offer_due());
}
