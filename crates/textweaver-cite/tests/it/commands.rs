//! The `tw cite` commands against a temporary library and recorded HTTP.

use std::path::{Path, PathBuf};

use textweaver_cite::commands::{self, Context, Show};
use textweaver_cite::{CiteError, Format, Library, OutputFormat, RecordedClient, formats};

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/p")
}

fn client() -> RecordedClient {
    let body = std::fs::read_to_string(fixtures().join("recorded/doi-nature12373.json")).unwrap();
    RecordedClient::new().with("https://doi.org/10.1038/nature12373", 200, &body)
}

fn ctx<'a>(dir: &Path, client: &'a RecordedClient) -> Context<'a> {
    Context {
        library: dir.join("user").join("references.json"),
        fallback: Vec::new(),
        cache_dir: Some(dir.join("cache")),
        client,
    }
}

#[test]
fn add_list_format_export_remove() {
    let dir = tempfile::tempdir().unwrap();
    let client = client();
    let ctx = ctx(dir.path(), &client);

    let empty = commands::list(&ctx, false).unwrap();
    assert!(
        empty.contains("is empty. Add references with tw cite add or tw cite import."),
        "{empty}"
    );

    let added = commands::add(&ctx, "10.1038/nature12373").unwrap();
    assert_eq!(
        added,
        "Added reference kucsko2013. Kucsko and others, 2013. Nanometre-scale thermometry in a living cell. Key kucsko2013."
    );
    let again = commands::add(&ctx, "https://doi.org/10.1038/nature12373").unwrap();
    assert!(
        again.starts_with("Updated reference kucsko2013, which was already in the library."),
        "{again}"
    );

    let list = commands::list(&ctx, false).unwrap();
    assert!(list.starts_with("1 reference in "), "{list}");
    assert!(list.ends_with(
        "Kucsko and others, 2013. Nanometre-scale thermometry in a living cell. Key kucsko2013."
    ));

    let json = commands::list(&ctx, true).unwrap();
    let parsed = formats::parse(&json, Format::CslJson).unwrap();
    assert_eq!(parsed[0].id, "kucsko2013");

    let apa = commands::format(
        &ctx,
        &["kucsko2013".into()],
        "apa",
        OutputFormat::Plain,
        Show::Both,
    )
    .unwrap();
    assert_eq!(
        apa,
        "Kucsko, G., Maurer, P. C., Yao, N. Y., Kubo, M., Noh, H. J., Lo, P. K., Park, H., & Lukin, M. D. (2013). \
         Nanometre-scale thermometry in a living cell. Nature, 500(7460), 54–58. https://doi.org/10.1038/nature12373\n\
         In text: (Kucsko et al., 2013)"
    );
    let ieee = commands::format(&ctx, &[], "ieee", OutputFormat::Plain, Show::Citation).unwrap();
    assert_eq!(ieee, "[1]");

    let bib = commands::export(&ctx, Format::BibTex, &[]).unwrap();
    assert!(bib.starts_with("@article{kucsko2013,\n"), "{bib}");
    let ris = commands::export(&ctx, Format::Ris, &["kucsko2013".into()]).unwrap();
    assert!(ris.starts_with("TY  - JOUR\r\n"), "{ris}");
    let missing = commands::export(&ctx, Format::Ris, &["nobody".into()]).unwrap_err();
    assert_eq!(
        missing.to_string(),
        "The citation key nobody is not in the reference library."
    );

    let removed = commands::remove(&ctx, "kucsko2013").unwrap();
    assert!(
        removed.starts_with("Removed reference kucsko2013: "),
        "{removed}"
    );
    assert!(Library::load(&ctx.library).unwrap().is_empty());
}

#[test]
fn offline_add_changes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let offline = RecordedClient::new();
    let ctx = ctx(dir.path(), &offline);
    let err = commands::add(&ctx, "10.1038/nature12373").unwrap_err();
    assert!(matches!(err, CiteError::Lookup(_)));
    assert!(err.to_string().contains("nothing was added"), "{err}");
    assert!(!ctx.library.exists(), "no library file is created");
}

#[test]
fn import_merges_and_reports() {
    let dir = tempfile::tempdir().unwrap();
    let client = client();
    let ctx = ctx(dir.path(), &client);
    let first = commands::import(&ctx, &fixtures().join("sample.bib")).unwrap();
    assert_eq!(first, "Imported 6 references: 6 new, 0 updated.");
    let second = commands::import(&ctx, &fixtures().join("sample.ris")).unwrap();
    assert_eq!(second, "Imported 3 references: 0 new, 3 updated.");
    let json = commands::import(&ctx, &fixtures().join("library.json")).unwrap();
    assert!(json.starts_with("Imported 7 references: "), "{json}");

    let err = commands::import(&ctx, &dir.path().join("nothing.bib")).unwrap_err();
    assert!(err.to_string().starts_with("Could not read "), "{err}");
    let odd = dir.path().join("notes.txt");
    std::fs::write(&odd, "just some words").unwrap();
    let err = commands::import(&ctx, &odd).unwrap_err().to_string();
    assert!(err.contains("Use a .bib, .ris, or .json file."), "{err}");
}

#[test]
fn folder_library_falls_back_to_the_user_library() {
    let dir = tempfile::tempdir().unwrap();
    let client = client();
    let user = ctx(dir.path(), &client);
    commands::import(&user, &fixtures().join("library.json")).unwrap();

    let folder = Context {
        library: dir.path().join("project").join("references.json"),
        fallback: vec![user.library.clone()],
        cache_dir: None,
        client: &client,
    };
    let text = "As shown [@dahl1988, p. 3] and [@nobody].";
    assert_eq!(
        commands::check(&folder, text).unwrap(),
        "2 citations found. One key is not in the library: nobody."
    );
    let out = commands::format(
        &folder,
        &["dahl1988".into()],
        "mla",
        OutputFormat::Markdown,
        Show::Bibliography,
    )
    .unwrap();
    assert_eq!(out, "Dahl, Roald. *Fantastic Mr. Fox*. Puffin, 1988.");
    assert!(commands::list(&folder, false).unwrap().contains("is empty"));
}

#[test]
fn styles_lists_featured_first() {
    let styles = commands::styles();
    let first: Vec<&str> = styles.lines().take(3).collect();
    assert_eq!(first[0], "apa: APA Style 7th edition");
    assert!(first[1].starts_with("mla: "));
    assert!(styles.lines().count() > 50);
}

#[test]
fn format_accepts_a_csl_file() {
    let dir = tempfile::tempdir().unwrap();
    let client = client();
    let ctx = ctx(dir.path(), &client);
    commands::import(&ctx, &fixtures().join("library.json")).unwrap();
    let csl = fixtures().join("styles/textweaver-plain.csl");
    let out = commands::format(
        &ctx,
        &["dahl1988".into()],
        csl.to_str().unwrap(),
        OutputFormat::Plain,
        Show::Both,
    )
    .unwrap();
    assert_eq!(
        out,
        "Dahl, Roald. 1988. Fantastic Mr. Fox. Puffin.\nIn text: (Dahl, 1988)"
    );
    let err = commands::format(
        &ctx,
        &["dahl1988".into()],
        "nonsense",
        OutputFormat::Plain,
        Show::Both,
    )
    .unwrap_err();
    assert!(
        err.to_string()
            .starts_with("There is no built-in citation style called nonsense."),
        "{err}"
    );
}
