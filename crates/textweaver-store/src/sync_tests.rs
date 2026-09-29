//! `tests/test_sync.py` ported one-to-one (45 tests, the Star parity reference
//! Part 3 §3.6), numbered as in the inventory, followed by the sidecar tests
//! from `tests/test_library.py` and tests for the fixed Star bugs.

use serde_json::{Value, json};

use super::*;

fn merge(local: Value, remote: Value, policy: ConflictPolicy) -> (Value, Vec<Conflict>) {
    let (m, c) = merge_progress(&local, &remote, policy, Prefer::Remote);
    (m.to_value(), c)
}

fn ids(list: &Value) -> Vec<String> {
    list.as_array()
        .unwrap()
        .iter()
        .map(|a| a["id"].as_str().unwrap().to_owned())
        .collect()
}

fn id_set(list: &Value) -> std::collections::BTreeSet<String> {
    ids(list).into_iter().collect()
}

use ConflictPolicy::{HighestProgress, Manual, Newest};

// ---- Basic shape / no-conflict identity ----

#[test]
fn t01_empty_inputs_yield_empty_merge() {
    let (m, c) = merge(json!({}), json!({}), Newest);
    assert_eq!(m, json!({}));
    assert!(c.is_empty());
}

#[test]
fn t02_disjoint_keys_are_unioned_without_conflict() {
    let (m, c) = merge(
        json!({"a.md": {"pct": 10, "ts": "2026-01-01"}}),
        json!({"b.md": {"pct": 20, "ts": "2026-01-02"}}),
        Newest,
    );
    assert_eq!(m["a.md"]["pct"], 10);
    assert_eq!(m["b.md"]["pct"], 20);
    assert!(c.is_empty());
}

#[test]
fn t03_identical_entry_is_not_a_conflict() {
    let entry = json!({"offset": 5, "pct": 42, "ts": "2026-06-27T10:00:00"});
    let (m, c) = merge(
        json!({"doc.md": entry.clone()}),
        json!({"doc.md": entry.clone()}),
        Newest,
    );
    assert_eq!(m["doc.md"], entry);
    assert!(c.is_empty());
}

#[test]
fn t04_one_side_missing_takes_the_other_no_conflict() {
    let local = json!({"doc.md": {"pct": 1, "ts": "t1"}});
    let (m, c) = merge(local.clone(), json!({}), Newest);
    assert_eq!(m["doc.md"]["pct"], 1);
    assert!(c.is_empty());
    let (m, c) = merge(json!({}), local, Newest);
    assert_eq!(m["doc.md"]["pct"], 1);
    assert!(c.is_empty());
}

// ---- Policy: newest ----

#[test]
fn t05_newest_remote_newer_wins() {
    let (m, c) = merge(
        json!({"doc.md": {"pct": 10, "ts": "2026-01-01T00:00:00"}}),
        json!({"doc.md": {"pct": 90, "ts": "2026-06-01T00:00:00"}}),
        Newest,
    );
    assert_eq!(m["doc.md"]["pct"], 90);
    assert_eq!(c.len(), 1);
    assert_eq!(c[0].path, "doc.md");
    assert_eq!(c[0].field, None);
    assert_eq!(c[0].resolution, Resolution::Remote);
    assert_eq!(c[0].local["pct"], 10);
    assert_eq!(c[0].remote["pct"], 90);
}

#[test]
fn t06_newest_local_newer_wins() {
    let (m, c) = merge(
        json!({"doc.md": {"pct": 88, "ts": "2026-06-01T00:00:00"}}),
        json!({"doc.md": {"pct": 5, "ts": "2026-01-01T00:00:00"}}),
        Newest,
    );
    assert_eq!(m["doc.md"]["pct"], 88);
    assert_eq!(c[0].resolution, Resolution::Local);
}

#[test]
fn t07_newest_tie_prefers_remote_reproducing_last_write_wins() {
    let (m, c) = merge(
        json!({"doc.md": {"pct": 10, "ts": "2026-06-27T10:00:00"}}),
        json!({"doc.md": {"pct": 20, "ts": "2026-06-27T10:00:00"}}),
        Newest,
    );
    assert_eq!(m["doc.md"]["pct"], 20);
    assert_eq!(c[0].resolution, Resolution::Remote);
}

#[test]
fn t08_newest_reproduces_prior_whole_file_overwrite() {
    let (m, _) = merge(
        json!({"a.md": {"pct": 1, "ts": "2026-01-01"}, "b.md": {"pct": 2, "ts": "2026-01-01"}}),
        json!({"a.md": {"pct": 11, "ts": "2026-02-01"}, "b.md": {"pct": 22, "ts": "2026-02-01"}}),
        Newest,
    );
    assert_eq!(m["a.md"]["pct"], 11);
    assert_eq!(m["b.md"]["pct"], 22);
}

#[test]
fn t09_prefer_local_breaks_tie_toward_local() {
    let (m, c) = merge_progress(
        &json!({"doc.md": {"pct": 10, "ts": "2026-06-27T10:00:00"}}),
        &json!({"doc.md": {"pct": 20, "ts": "2026-06-27T10:00:00"}}),
        Newest,
        Prefer::Local,
    );
    assert_eq!(m.to_value()["doc.md"]["pct"], 10);
    assert_eq!(c[0].resolution, Resolution::Local);
}

#[test]
fn t10_missing_timestamp_loses_to_timestamped() {
    let (m, _) = merge(
        json!({"doc.md": {"pct": 10}}),
        json!({"doc.md": {"pct": 20, "ts": "2026-01-01"}}),
        Newest,
    );
    assert_eq!(m["doc.md"]["pct"], 20);
}

// ---- Policy: highest_progress ----

#[test]
fn t11_highest_progress_keeps_max_offset() {
    let (m, c) = merge(
        json!({"doc.md": {"offset": 500, "pct": 20, "ts": "2026-06-01"}}),
        json!({"doc.md": {"offset": 5000, "pct": 80, "ts": "2026-01-01"}}),
        HighestProgress,
    );
    assert_eq!(m["doc.md"]["offset"], 5000);
    assert_eq!(c[0].resolution, Resolution::Remote);
}

#[test]
fn t12_highest_progress_local_further_wins() {
    let (m, c) = merge(
        json!({"doc.md": {"offset": 9000, "pct": 95, "ts": "2026-01-01"}}),
        json!({"doc.md": {"offset": 100, "pct": 3, "ts": "2026-06-01"}}),
        HighestProgress,
    );
    assert_eq!(m["doc.md"]["offset"], 9000);
    assert_eq!(c[0].resolution, Resolution::Local);
}

#[test]
fn t13_highest_progress_falls_back_to_pct_when_no_offset() {
    let (m, _) = merge(
        json!({"doc.md": {"pct": 30, "ts": "2026-06-01"}}),
        json!({"doc.md": {"pct": 70, "ts": "2026-01-01"}}),
        HighestProgress,
    );
    assert_eq!(m["doc.md"]["pct"], 70);
}

#[test]
fn t14_highest_progress_equal_position_breaks_by_newest() {
    let (m, c) = merge(
        json!({"doc.md": {"offset": 500, "pct": 50, "ts": "2026-06-01", "note": "L"}}),
        json!({"doc.md": {"offset": 500, "pct": 50, "ts": "2026-01-01", "note": "R"}}),
        HighestProgress,
    );
    assert_eq!(m["doc.md"]["note"], "L");
    assert_eq!(c[0].resolution, Resolution::Local);
}

// ---- Policy: manual ----

#[test]
fn t15_manual_keeps_local_and_reports_conflict() {
    let (m, c) = merge(
        json!({"doc.md": {"pct": 10, "ts": "2026-01-01"}}),
        json!({"doc.md": {"pct": 90, "ts": "2026-06-01"}}),
        Manual,
    );
    assert_eq!(m["doc.md"]["pct"], 10);
    assert_eq!(c.len(), 1);
    assert_eq!(c[0].resolution, Resolution::Unresolved);
    assert_eq!(c[0].local["pct"], 10);
    assert_eq!(c[0].remote["pct"], 90);
}

#[test]
fn t16_manual_no_conflict_when_identical() {
    let entry = json!({"pct": 5, "ts": "t"});
    let (m, c) = merge(
        json!({"d.md": entry.clone()}),
        json!({"d.md": entry.clone()}),
        Manual,
    );
    assert!(c.is_empty());
    assert_eq!(m["d.md"], entry);
}

// ---- Unknown policy ----

#[test]
fn t17_unknown_policy_falls_back_to_newest() {
    let (m, _) = merge(
        json!({"doc.md": {"pct": 10, "ts": "2026-01-01"}}),
        json!({"doc.md": {"pct": 20, "ts": "2026-06-01"}}),
        ConflictPolicy::parse("bogus-policy"),
    );
    assert_eq!(m["doc.md"]["pct"], 20);
    assert_eq!(ConflictPolicy::parse("bogus-policy"), Newest);
}

#[test]
fn t18_all_named_policies_are_recognised() {
    for p in ConflictPolicy::ALL {
        assert_eq!(ConflictPolicy::parse(p.as_str()), p);
        let (m, _) = merge(json!({"d.md": {"pct": 1, "ts": "t"}}), json!({}), p);
        assert_eq!(m["d.md"]["pct"], 1);
    }
}

// ---- Annotations and highlights by id ----

#[test]
fn t19_annotations_disjoint_ids_both_kept() {
    let (m, c) = merge(
        json!({"annotations": [{"id": "a1", "note": "hello", "ts": "2026-01-01"}]}),
        json!({"annotations": [{"id": "b2", "note": "world", "ts": "2026-01-02"}]}),
        Newest,
    );
    assert_eq!(
        id_set(&m["annotations"]),
        ["a1", "b2"].map(String::from).into()
    );
    assert!(c.is_empty());
}

#[test]
fn t20_annotations_same_id_newest_content_wins() {
    let (m, c) = merge(
        json!({"annotations": [{"id": "x", "note": "old note", "ts": "2026-01-01"}]}),
        json!({"annotations": [{"id": "x", "note": "edited note", "ts": "2026-06-01"}]}),
        Newest,
    );
    let list = m["annotations"].as_array().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["note"], "edited note");
    assert_eq!(c.len(), 1);
    assert_eq!(c[0].field.as_deref(), Some("x"));
    assert_eq!(c[0].resolution, Resolution::Remote);
}

#[test]
fn t21_annotations_same_id_local_newer_wins() {
    let (m, c) = merge(
        json!({"annotations": [{"id": "x", "note": "local edit", "ts": "2026-06-01"}]}),
        json!({"annotations": [{"id": "x", "note": "stale", "ts": "2026-01-01"}]}),
        Newest,
    );
    let list = m["annotations"].as_array().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["note"], "local edit");
    assert_eq!(c[0].resolution, Resolution::Local);
}

#[test]
fn t22_annotations_edits_on_two_devices_both_survive() {
    let (m, _) = merge(
        json!({"annotations": [
            {"id": "shared", "note": "A's newer text", "ts": "2026-06-02"},
            {"id": "a1", "note": "A only", "ts": "2026-06-02"}
        ]}),
        json!({"annotations": [
            {"id": "shared", "note": "B's older text", "ts": "2026-06-01"},
            {"id": "b1", "note": "B only", "ts": "2026-06-01"}
        ]}),
        Newest,
    );
    let by_id: std::collections::HashMap<String, &Value> = m["annotations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| (a["id"].as_str().unwrap().to_owned(), a))
        .collect();
    assert_eq!(by_id.len(), 3);
    assert_eq!(by_id["shared"]["note"], "A's newer text");
    assert_eq!(by_id["a1"]["note"], "A only");
    assert_eq!(by_id["b1"]["note"], "B only");
}

#[test]
fn t23_annotations_identical_same_id_no_conflict() {
    let ann = json!({"id": "x", "note": "same", "ts": "t"});
    let (m, c) = merge(
        json!({"annotations": [ann.clone()]}),
        json!({"annotations": [ann]}),
        Newest,
    );
    assert_eq!(m["annotations"].as_array().unwrap().len(), 1);
    assert!(c.is_empty());
}

#[test]
fn t24_annotations_idless_entries_kept_from_both_sides() {
    let (m, _) = merge(
        json!({"annotations": [{"note": "no id local", "ts": "t"}]}),
        json!({"annotations": [{"note": "no id remote", "ts": "t"}]}),
        Newest,
    );
    let mut notes: Vec<&str> = m["annotations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["note"].as_str().unwrap())
        .collect();
    notes.sort_unstable();
    assert_eq!(notes, vec!["no id local", "no id remote"]);
}

#[test]
fn t25_annotations_merge_ignores_progress_policy() {
    let (m, _) = merge(
        json!({"annotations": [{"id": "x", "note": "L", "ts": "2026-06-01"}]}),
        json!({"annotations": [{"id": "y", "note": "R", "ts": "2026-01-01"}]}),
        HighestProgress,
    );
    assert_eq!(
        id_set(&m["annotations"]),
        ["x", "y"].map(String::from).into()
    );
}

#[test]
fn t26_merge_annotations_helper_preserves_order_local_first() {
    let local = json!([{"id": "1", "ts": "t"}, {"id": "2", "ts": "t"}]);
    let remote = json!([{"id": "3", "ts": "t"}, {"id": "2", "ts": "t"}]);
    let out = merge_annotations("k", Some(&local), Some(&remote), None, Prefer::Remote);
    assert_eq!(ids(&Value::Array(out)), vec!["1", "2", "3"]);
}

#[test]
fn t27_highlights_key_also_merges_by_id() {
    let (m, c) = merge(
        json!({"highlights": [{"id": "h1", "color": "cyan", "ts": "2026-01-01"}]}),
        json!({"highlights": [{"id": "h1", "color": "yellow", "ts": "2026-06-01"}]}),
        Newest,
    );
    let list = m["highlights"].as_array().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["color"], "yellow");
    assert_eq!(c[0].field.as_deref(), Some("h1"));
}

#[test]
fn t28_list_valued_key_under_any_name_merges_by_id() {
    let (m, _) = merge(
        json!({"my_notes": [{"id": "n1", "ts": "t"}]}),
        json!({"my_notes": [{"id": "n2", "ts": "t"}]}),
        Newest,
    );
    assert_eq!(
        id_set(&m["my_notes"]),
        ["n1", "n2"].map(String::from).into()
    );
}

// ---- _meta namespace ----

#[test]
fn t29_meta_namespace_merges_rel_by_rel_newest() {
    let (m, c) = merge(
        json!({"_meta": {"a.md": {"seconds": 10, "last_ts": "2026-01-01"}}}),
        json!({"_meta": {
            "a.md": {"seconds": 99, "last_ts": "2026-06-01"},
            "b.md": {"seconds": 5, "last_ts": "2026-01-01"}
        }}),
        Newest,
    );
    assert_eq!(m["_meta"]["a.md"]["seconds"], 99);
    assert_eq!(m["_meta"]["b.md"]["seconds"], 5);
    assert!(
        c.iter()
            .any(|c| c.path == "_meta" && c.field.as_deref() == Some("a.md"))
    );
}

#[test]
fn t30_meta_uses_last_ts_for_recency() {
    let (m, _) = merge(
        json!({"_meta": {"a.md": {"seconds": 100, "last_ts": "2026-06-01"}}}),
        json!({"_meta": {"a.md": {"seconds": 1, "last_ts": "2026-01-01"}}}),
        Newest,
    );
    assert_eq!(m["_meta"]["a.md"]["seconds"], 100);
}

// ---- Robustness ----

#[test]
fn t31_none_inputs_treated_as_empty() {
    let (m, c) = merge(Value::Null, Value::Null, Newest);
    assert_eq!(m, json!({}));
    assert!(c.is_empty());
    let (m, _) = merge(Value::Null, json!({"d.md": {"pct": 1, "ts": "t"}}), Newest);
    assert_eq!(m["d.md"]["pct"], 1);
}

#[test]
fn t32_non_dict_top_level_input_is_ignored() {
    for junk in [json!([1, 2, 3]), json!("corrupt"), json!(42), json!(2.5)] {
        let (m, c) = merge(json!({"d.md": {"pct": 5, "ts": "t"}}), junk.clone(), Newest);
        assert_eq!(m["d.md"]["pct"], 5);
        assert!(c.is_empty());
        let (m, _) = merge(junk, json!({"d.md": {"pct": 9, "ts": "t"}}), Newest);
        assert_eq!(m["d.md"]["pct"], 9);
    }
}

#[test]
fn t33_corrupt_entry_value_does_not_crash() {
    let (m, c) = merge(
        json!({"d.md": "not-a-dict"}),
        json!({"d.md": {"pct": 7, "ts": "2026-06-01"}}),
        Newest,
    );
    assert_eq!(m["d.md"]["pct"], 7);
    assert_eq!(c[0].resolution, Resolution::Remote);
}

#[test]
fn t34_corrupt_annotation_value_does_not_crash() {
    let (m, _) = merge(
        json!({"annotations": "corrupt"}),
        json!({"annotations": [{"id": "x", "ts": "t"}]}),
        Newest,
    );
    assert_eq!(ids(&m["annotations"]), vec!["x"]);
}

#[test]
fn t35_corrupt_meta_value_does_not_crash() {
    let (m, _) = merge(
        json!({"_meta": "corrupt"}),
        json!({"_meta": {"a.md": {"seconds": 3, "last_ts": "t"}}}),
        Newest,
    );
    assert_eq!(m["_meta"]["a.md"]["seconds"], 3);
}

#[test]
fn t36_valid_progress_dict_survives_a_corrupt_list_on_the_other_side() {
    let valid = json!({"offset": 4321, "pct": 55, "ts": "2026-06-30T09:00:00"});
    let (m, _) = merge_progress(
        &json!({"a": valid.clone()}),
        &json!({"a": ["garbage"]}),
        Newest,
        Prefer::Local,
    );
    assert_eq!(m.get("a"), Some(&valid));
    let (m, _) = merge(
        json!({"a": ["garbage"]}),
        json!({"a": valid.clone()}),
        Newest,
    );
    assert_eq!(m["a"], valid);
}

#[test]
fn t37_dict_vs_list_never_routes_to_annotation_merge() {
    let (d, l) = (json!({"offset": 1}), json!(["x"]));
    assert!(!is_annotation_list(Some(&d), Some(&l)));
    assert!(!is_annotation_list(Some(&l), Some(&d)));
    let (a, b) = (json!([{"id": "a"}]), json!([{"id": "b"}]));
    assert!(is_annotation_list(Some(&a), Some(&b)));
    assert!(is_annotation_list(Some(&a), None));
    assert!(is_annotation_list(None, Some(&a)));
}

#[test]
fn t38_boolean_offset_not_mistaken_for_number() {
    let (m, _) = merge(
        json!({"d.md": {"offset": true, "pct": 5, "ts": "2026-01-01"}}),
        json!({"d.md": {"offset": 100, "pct": 50, "ts": "2026-06-01"}}),
        HighestProgress,
    );
    assert_eq!(m["d.md"]["offset"], 100);
}

// ---- Conflict shape ----

#[test]
fn t39_conflict_dataclass_fields() {
    let c = Conflict::new(
        "doc.md",
        Some("pct".into()),
        json!({"pct": 1}),
        json!({"pct": 2}),
    )
    .resolved(Resolution::Remote);
    assert_eq!(c.path, "doc.md");
    assert_eq!(c.field.as_deref(), Some("pct"));
    assert_eq!(c.local, json!({"pct": 1}));
    assert_eq!(c.remote, json!({"pct": 2}));
    assert_eq!(c.resolution, Resolution::Remote);
}

#[test]
fn t40_conflict_default_resolution_is_unresolved() {
    let c = Conflict::new("doc.md", None, json!(1), json!(2));
    assert_eq!(c.resolution, Resolution::Unresolved);
    assert_eq!(Resolution::default(), Resolution::Unresolved);
}

// ---- library.py integration ----

#[test]
fn t41_meta_key_constant_matches_library() {
    // Star pinned `sync._META_KEY == library._META_KEY`; here one constant
    // serves both the merge and the sidecar store.
    assert_eq!(META_KEY, "_meta");
    let mut m = SidecarMap::new();
    m.insert(META_KEY, json!({}));
    m.insert("a.md", json!({"pct": 1}));
    assert_eq!(m.documents().keys().collect::<Vec<_>>(), vec!["a.md"]);
}

fn library() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("lib");
    std::fs::create_dir_all(folder.join(SIDECAR_DIR)).unwrap();
    (dir, folder)
}

fn write_disk(folder: &Path, v: Value) {
    std::fs::write(sidecar_file(folder), serde_json::to_string(&v).unwrap()).unwrap();
}

#[test]
fn t42_reconcile_before_write_no_remote_returns_local() {
    let (_d, folder) = library();
    let local = SidecarMap::from_value(&json!({"a.md": {"pct": 5, "ts": "t"}}));
    let (out, c) = reconcile_before_write(&folder, &local, Newest);
    assert_eq!(out, local);
    assert!(c.is_empty());
}

#[test]
fn t43_reconcile_before_write_local_edit_survives_equal_ts() {
    let (_d, folder) = library();
    write_disk(
        &folder,
        json!({"a.md": {"pct": 10, "ts": "2026-06-27T10:00:00"}}),
    );
    let local = SidecarMap::from_value(&json!({"a.md": {"pct": 40, "ts": "2026-06-27T10:00:00"}}));
    let (out, _) = reconcile_before_write(&folder, &local, Newest);
    assert_eq!(out.get("a.md").unwrap()["pct"], 40);
}

#[test]
fn t44_reconcile_before_write_newer_remote_survives() {
    let (_d, folder) = library();
    write_disk(
        &folder,
        json!({"other.md": {"pct": 77, "ts": "2026-06-30"}}),
    );
    let local = SidecarMap::from_value(&json!({"a.md": {"pct": 40, "ts": "2026-06-27"}}));
    let (out, _) = reconcile_before_write(&folder, &local, Newest);
    assert_eq!(out.get("a.md").unwrap()["pct"], 40);
    assert_eq!(out.get("other.md").unwrap()["pct"], 77);
}

#[test]
fn t45_conflict_policy_default() {
    assert_eq!(
        crate::Settings::default().reading.sync_conflict_policy,
        Newest
    );
    assert_eq!(SidecarStore::default().policy(), Newest);
}

// ---- tests/test_library.py sidecar tests ----

#[test]
fn library_record_and_read_uses_the_relative_key() {
    let (_d, folder) = library();
    std::fs::create_dir_all(folder.join("sub")).unwrap();
    let book = folder.join("sub").join("book.epub");
    std::fs::write(&book, "x").unwrap();
    let (root, rel) = folder_for(std::slice::from_ref(&folder), &book).unwrap();
    assert_eq!(rel, "sub/book.epub");
    let store = SidecarStore::new(Newest);
    let entry = ProgressEntry::new(CharPos(1200), 42, 1_790_344_987).to_value();
    let rec = store
        .record_progress(&root, &rel, entry.clone(), None)
        .unwrap();
    assert!(rec.written);
    assert_eq!(store.progress_for(&root, &rel), Some(entry.clone()));
    // A second machine mounting the folder elsewhere reads the same key.
    let other = SidecarStore::new(Newest);
    assert_eq!(other.progress_for(&root, "sub/book.epub"), Some(entry));
}

#[test]
fn library_no_op_outside_a_library() {
    let (d, folder) = library();
    let outside = d.path().join("elsewhere.md");
    assert_eq!(folder_for(&[folder], &outside), None);
}

#[test]
fn library_deepest_folder_wins() {
    let (_d, folder) = library();
    let inner = folder.join("inner");
    std::fs::create_dir_all(&inner).unwrap();
    let doc = inner.join("doc.md");
    std::fs::write(&doc, "x").unwrap();
    let (root, rel) = folder_for(&[folder.clone(), inner.clone()], &doc).unwrap();
    assert_eq!(rel, "doc.md");
    assert_eq!(root, crate::library::resolve_path(&inner));
    let (_, rel) = folder_for(&[folder], &doc).unwrap();
    assert_eq!(rel, "inner/doc.md");
}

#[test]
fn library_atomic_write_leaves_no_temp_file() {
    let (_d, folder) = library();
    let store = SidecarStore::with_debounce(Newest, Duration::ZERO);
    for pct in [1, 2, 3] {
        store
            .record_progress(&folder, "a.md", json!({"pct": pct, "ts": "t"}), None)
            .unwrap();
    }
    let names: Vec<String> = std::fs::read_dir(folder.join(SIDECAR_DIR))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, vec![SIDECAR_FILE.to_owned()]);
}

#[test]
fn library_bad_target_returns_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("lib");
    std::fs::create_dir_all(&folder).unwrap();
    // `.textweaver` is a file, so the sidecar cannot be created.
    std::fs::write(folder.join(SIDECAR_DIR), "blocker").unwrap();
    let store = SidecarStore::new(Newest);
    assert!(
        store
            .record_progress(&folder, "a.md", json!({"pct": 1}), None)
            .is_err()
    );
}

#[test]
fn library_debounce_coalesces_and_flush_writes() {
    let (_d, folder) = library();
    let store = SidecarStore::with_debounce(Newest, Duration::from_secs(60));
    for pct in [10, 20, 30, 40] {
        store
            .record_progress(
                &folder,
                "a.md",
                json!({"pct": pct, "ts": format!("2026-09-25T10:00:{pct}Z")}),
                None,
            )
            .unwrap();
    }
    assert_eq!(store.writes(), 1);
    assert_eq!(store.progress_for(&folder, "a.md").unwrap()["pct"], 40);
    assert_eq!(read_sidecar(&folder).get("a.md").unwrap()["pct"], 10);
    store.flush(&folder).unwrap();
    assert_eq!(read_sidecar(&folder).get("a.md").unwrap()["pct"], 40);
    assert!(!store.has_pending());
}

#[test]
fn library_lock_under_twelve_threads_keeps_every_key() {
    let (_d, folder) = library();
    let store = SidecarStore::with_debounce(Newest, Duration::from_millis(5));
    let handles: Vec<_> = (0..12)
        .map(|i| {
            let store = store.clone();
            let folder = folder.clone();
            std::thread::spawn(move || {
                for _ in 0..5 {
                    store
                        .record_progress(
                            &folder,
                            &format!("doc{i}.md"),
                            json!({"pct": i, "ts": "2026-09-25T10:00:00Z"}),
                            None,
                        )
                        .unwrap();
                }
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
    store.flush_all().unwrap();
    let disk = read_sidecar(&folder);
    for i in 0..12 {
        assert_eq!(disk.get(&format!("doc{i}.md")).unwrap()["pct"], i);
    }
}

#[test]
fn library_meta_round_trip_and_hidden_from_documents() {
    let (_d, folder) = library();
    let store = SidecarStore::new(Newest);
    let meta =
        json!({"seconds": 12.5, "pct": 55, "last_ts": "2026-09-25T10:00:00Z", "annotations": 3});
    store
        .record_progress(
            &folder,
            "a.md",
            json!({"pct": 55, "ts": "t"}),
            Some(meta.clone()),
        )
        .unwrap();
    let fresh = SidecarStore::new(Newest);
    let got = fresh.metadata_for(&folder, "a.md").unwrap();
    assert_eq!(got, meta);
    assert!(got.get("words_read").is_none());
    assert_eq!(fresh.load(&folder).keys().collect::<Vec<_>>(), vec!["a.md"]);
}

#[test]
fn library_legacy_sidecar_is_compatible() {
    let (_d, folder) = library();
    write_disk(
        &folder,
        json!({"old.epub": {"offset": 1200, "pct": 42, "ts": "2026-06-27T10:00:00"}}),
    );
    let store = SidecarStore::new(Newest);
    let entry =
        ProgressEntry::from_value(&store.progress_for(&folder, "old.epub").unwrap()).unwrap();
    assert_eq!(entry.offset, CharPos(1200));
    assert_eq!(entry.pct, 42);
    assert_eq!(
        entry.ts_secs(),
        crate::time::parse_timestamp("2026-06-27T10:00:00")
    );
    store
        .record_progress(
            &folder,
            "new.md",
            json!({"pct": 1, "ts": "2026-09-25T10:00:00Z"}),
            None,
        )
        .unwrap();
    let disk = read_sidecar(&folder);
    assert_eq!(disk.keys().collect::<Vec<_>>(), vec!["old.epub", "new.md"]);
}

// ---- Fixed Star bugs ----

/// Star's `record_progress` re-asserted the local entry after merging, so
/// a glance at 5% on device B overwrote 80% from device A even under
/// `highest_progress` (Part 3 §7 item 23).
#[test]
fn fix_highest_progress_protects_the_written_document() {
    let (_d, folder) = library();
    write_disk(
        &folder,
        json!({"book.md": {"offset": 8000, "pct": 80, "ts": "2026-09-20T10:00:00Z"}}),
    );
    let store = SidecarStore::new(HighestProgress);
    let glance = json!({"offset": 500, "pct": 5, "ts": "2026-09-25T10:00:00Z"});
    let rec = store
        .record_progress(&folder, "book.md", glance, None)
        .unwrap();
    assert!(rec.written);
    assert_eq!(read_sidecar(&folder).get("book.md").unwrap()["pct"], 80);
    assert_eq!(
        rec.conflicts.len(),
        1,
        "the conflict is reported, not dropped"
    );
    assert_eq!(rec.conflicts[0].resolution, Resolution::Remote);
}

#[test]
fn fix_manual_keeps_local_but_reports() {
    let (_d, folder) = library();
    write_disk(
        &folder,
        json!({"book.md": {"pct": 80, "ts": "2026-09-26T10:00:00Z"}}),
    );
    let store = SidecarStore::new(Manual);
    let rec = store
        .record_progress(
            &folder,
            "book.md",
            json!({"pct": 5, "ts": "2026-09-25T10:00:00Z"}),
            None,
        )
        .unwrap();
    assert_eq!(read_sidecar(&folder).get("book.md").unwrap()["pct"], 5);
    assert_eq!(rec.conflicts[0].resolution, Resolution::Unresolved);
}

#[test]
fn fix_newest_still_writes_the_fresh_local_entry() {
    let (_d, folder) = library();
    write_disk(
        &folder,
        json!({"book.md": {"pct": 80, "ts": "2026-09-20T10:00:00Z"}}),
    );
    let store = SidecarStore::new(Newest);
    store
        .record_progress(
            &folder,
            "book.md",
            json!({"pct": 5, "ts": "2026-09-25T10:00:00Z"}),
            None,
        )
        .unwrap();
    assert_eq!(read_sidecar(&folder).get("book.md").unwrap()["pct"], 5);
}

/// Star never called `flush_pending`, so a position recorded within the
/// debounce window of the last write was lost at exit (item 22).
#[test]
fn fix_pending_sidecar_data_is_flushed_on_drop() {
    let (_d, folder) = library();
    {
        let store = SidecarStore::with_debounce(Newest, Duration::from_secs(60));
        for pct in [1, 2] {
            store
                .record_progress(
                    &folder,
                    "a.md",
                    json!({"pct": pct, "ts": format!("2026-09-25T10:00:0{pct}Z")}),
                    None,
                )
                .unwrap();
        }
        assert!(store.has_pending());
    }
    assert_eq!(read_sidecar(&folder).get("a.md").unwrap()["pct"], 2);
}

#[test]
fn numeric_and_python_equal_values() {
    assert!(json_eq(&json!({"a": 1}), &json!({"a": 1.0})));
    assert!(!json_eq(&json!(true), &json!(1)));
    // A numeric Unix-seconds ts compares as a date, not as digits.
    let old = json!({"pct": 1, "ts": "2026-01-01T00:00:00Z"});
    let new = json!({"pct": 2, "ts": 1_790_344_987});
    let (winner, _) = newest(&old, &new, Prefer::Local);
    assert_eq!(winner["pct"], 2);
}

#[test]
fn resolve_entry_for_resume_honours_policy() {
    let local = json!({"offset": 10, "pct": 1, "ts": "2026-09-25T10:00:00Z"});
    let side = json!({"offset": 900, "pct": 90, "ts": "2026-09-24T10:00:00Z"});
    let (v, c) = resolve_entry("doc.md", Some(&local), Some(&side), Newest, Prefer::Remote);
    assert_eq!(v["pct"], 1);
    assert_eq!(c.unwrap().resolution, Resolution::Local);
    let (v, _) = resolve_entry(
        "doc.md",
        Some(&local),
        Some(&side),
        HighestProgress,
        Prefer::Remote,
    );
    assert_eq!(v["pct"], 90);
    let (v, c) = resolve_entry("doc.md", None, Some(&side), Manual, Prefer::Remote);
    assert_eq!(v, side);
    assert!(c.is_none());
}

#[test]
fn sidecar_map_keeps_order_and_ignores_non_objects() {
    let m: SidecarMap = serde_json::from_str(r#"{"z": 1, "a": 2, "m": 3}"#).unwrap();
    assert_eq!(m.keys().collect::<Vec<_>>(), vec!["z", "a", "m"]);
    let text = serde_json::to_string(&m).unwrap();
    assert_eq!(text, r#"{"z":1,"a":2,"m":3}"#);
    assert!(serde_json::from_str::<SidecarMap>("[1,2]").is_err());
    let (_d, folder) = library();
    std::fs::write(sidecar_file(&folder), "[1, 2, 3]").unwrap();
    assert!(read_sidecar(&folder).is_empty());
    std::fs::write(sidecar_file(&folder), "{ not json").unwrap();
    assert!(read_sidecar(&folder).is_empty());
}

#[test]
fn merge_keeps_local_order_then_remote_only_keys() {
    let local: SidecarMap =
        serde_json::from_str(r#"{"z.md": {"pct": 1}, "a.md": {"pct": 2}}"#).unwrap();
    let remote: SidecarMap =
        serde_json::from_str(r#"{"q.md": {"pct": 3}, "a.md": {"pct": 2}}"#).unwrap();
    let (m, c) = merge_maps(&local, &remote, Newest, Prefer::Remote);
    assert_eq!(m.keys().collect::<Vec<_>>(), vec!["z.md", "a.md", "q.md"]);
    assert!(c.is_empty());
}

#[test]
fn progress_entry_round_trip() {
    let e = ProgressEntry::new(CharPos(7), 3, 1_790_344_987);
    assert_eq!(e.ts, "2026-09-25T14:03:07Z");
    assert_eq!(ProgressEntry::from_value(&e.to_value()), Some(e.clone()));
    assert_eq!(e.ts_secs(), Some(1_790_344_987));
    assert_eq!(ProgressEntry::from_value(&json!({"note": "x"})), None);
    assert_eq!(ProgressEntry::from_value(&json!("x")), None);
}
