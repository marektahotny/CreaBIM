use creabim_core::{sample::sample_house, Document, Element, ElementKind, Level, Transaction};
use creabim_store::{import_json, load, open, open_mended, read_meta, save, Store, StoreError};
use std::path::{Path, PathBuf};

fn tmp() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("house.creabim");
    (dir, path)
}

fn first_wall(doc: &Document) -> creabim_core::ElementId {
    doc.model()
        .iter()
        .find(|e| matches!(e.kind, ElementKind::Wall(_)))
        .unwrap()
        .id
}

fn set_thickness(t: f64) -> impl FnOnce(&mut Element) {
    move |e| {
        if let ElementKind::Wall(w) = &mut e.kind {
            w.thickness = t;
        }
    }
}

fn add_level(doc: &mut Document, name: &str) -> creabim_core::ElementId {
    let l = Element::new(
        name,
        ElementKind::Level(Level {
            elevation: 6.0,
            height: 2.8,
        }),
    );
    let id = l.id;
    doc.commit(Transaction::new(format!("Add {name}")).insert(l))
        .unwrap();
    id
}

/// Model and both stacks, compared step by step.
fn assert_same(a: &Document, b: &Document) {
    assert_eq!(a.model(), b.model());
    let txs = |s: &[creabim_core::Step]| s.iter().map(|s| s.tx.clone()).collect::<Vec<_>>();
    assert_eq!(
        txs(a.undo_steps()),
        txs(b.undo_steps()),
        "undo stacks differ"
    );
    assert_eq!(
        txs(a.redo_steps()),
        txs(b.redo_steps()),
        "redo stacks differ"
    );
}

/// A few edits of every kind on the sample house, one of them undone.
fn edited_sample() -> Document {
    let mut doc = Document::from_model(sample_house());
    let wall = first_wall(&doc);
    doc.edit(wall, "Thicken wall", set_thickness(0.35)).unwrap();
    add_level(&mut doc, "Roof");
    let tx = Transaction::new("Delete wall").remove_cascade(doc.model(), wall);
    doc.commit(tx).unwrap();
    doc.undo();
    doc
}

#[test]
fn save_then_open_round_trips_the_sample_house() {
    let (_dir, path) = tmp();
    let doc = Document::from_model(sample_house());
    save(&path, &doc).unwrap();
    let back = open(&path).unwrap();
    assert_same(&doc, &back);
    // element order matters to views, so it is kept exactly
    let ids = |d: &Document| d.model().iter().map(|e| e.id).collect::<Vec<_>>();
    assert_eq!(ids(&doc), ids(&back));
}

#[test]
fn save_then_open_keeps_undo_and_redo() {
    let (_dir, path) = tmp();
    let mut doc = edited_sample();
    save(&path, &doc).unwrap();
    let mut back = open(&path).unwrap();
    assert_same(&doc, &back);
    assert_eq!(back.redo_label(), Some("Delete wall"));

    // the restored history behaves exactly like the live one
    assert!(back.redo() && doc.redo());
    assert_same(&doc, &back);
    while doc.undo() {
        assert!(back.undo());
        assert_eq!(doc.model(), back.model());
    }
    assert!(!back.undo());
}

#[test]
fn saving_over_an_existing_project_replaces_it() {
    let (_dir, path) = tmp();
    save(&path, &edited_sample()).unwrap();
    let doc = Document::from_model(sample_house());
    save(&path, &doc).unwrap();
    assert_same(&open(&path).unwrap(), &doc);
    assert_eq!(read_meta(&path).unwrap().project_name, "house");
}

#[test]
fn recording_each_change_then_reopening_gives_the_same_document() {
    let (_dir, path) = tmp();
    let mut doc = Document::from_model(sample_house());
    let mut store = Store::create(&path, &doc).unwrap();
    let wall = first_wall(&doc);

    doc.edit(wall, "Thicken wall", set_thickness(0.3)).unwrap();
    store.record(&doc).unwrap();
    add_level(&mut doc, "Roof");
    store.record(&doc).unwrap();
    doc.undo();
    store.record(&doc).unwrap();
    doc.redo();
    store.record(&doc).unwrap();
    doc.undo();
    store.record(&doc).unwrap();
    // a new edit after an undo drops the redo branch
    let tx = Transaction::new("Delete wall").remove_cascade(doc.model(), wall);
    doc.commit(tx).unwrap();
    store.record(&doc).unwrap();
    doc.undo();
    store.record(&doc).unwrap();
    assert_same(&open(&path).unwrap(), &doc);
    drop(store);

    let (store, back) = Store::open(&path).unwrap();
    assert_same(&doc, &back);
    store.close().unwrap();
}

#[test]
fn recorded_changes_survive_the_app_dying_without_closing() {
    let (_dir, path) = tmp();
    let mut doc = Document::from_model(sample_house());
    let mut store = Store::create(&path, &doc).unwrap();
    add_level(&mut doc, "Roof");
    store.record(&doc).unwrap();
    // no close, no checkpoint: the change lives only in the WAL
    std::mem::forget(store);
    assert_same(&open(&path).unwrap(), &doc);
}

#[test]
fn merged_edits_amend_one_journal_row() {
    let (_dir, path) = tmp();
    let mut doc = Document::from_model(sample_house());
    let mut store = Store::create(&path, &doc).unwrap();
    let wall = first_wall(&doc);
    let rows = |p: &Path| -> i64 {
        let c = rusqlite::Connection::open(p).unwrap();
        c.query_row("SELECT COUNT(*) FROM ops", [], |r| r.get(0))
            .unwrap()
    };
    for t in [0.21, 0.22, 0.23, 0.24] {
        doc.edit_merged(wall, "Wall thickness", set_thickness(t))
            .unwrap();
        store.record(&doc).unwrap();
    }
    assert_eq!(rows(&path), 1, "a drag is one step on disk as in memory");
    assert_same(&open(&path).unwrap(), &doc);

    // after an undo/redo the step is no longer the newest row: amending it
    // again is logged, and still reopens to the same state
    doc.undo();
    doc.redo();
    store.record(&doc).unwrap();
    doc.edit_merged(wall, "Wall thickness", set_thickness(0.5))
        .unwrap();
    store.record(&doc).unwrap();
    store.close().unwrap();
    assert_same(&open(&path).unwrap(), &doc);
}

#[test]
fn recording_several_changes_at_once_and_continuing_after_reopen() {
    let (_dir, path) = tmp();
    let mut doc = edited_sample();
    let mut store = Store::create(&path, &doc).unwrap();
    // several changes between records, including undo past the fork point
    doc.undo();
    doc.undo();
    add_level(&mut doc, "Attic");
    add_level(&mut doc, "Loft");
    doc.undo();
    store.record(&doc).unwrap();
    store.record(&doc).unwrap(); // nothing changed: a no-op
    drop(store);

    let (mut store, mut back) = Store::open(&path).unwrap();
    assert_same(&doc, &back);
    for d in [&mut doc, &mut back] {
        d.redo();
        add_level(d, "Mezzanine");
        d.undo();
    }
    store.record(&back).unwrap();
    store.close().unwrap();
    let reopened = open(&path).unwrap();
    assert_eq!(reopened.model(), doc.model());
    assert_eq!(reopened.redo_label(), Some("Add Mezzanine"));
    assert_eq!(reopened.undo_steps().len(), doc.undo_steps().len());
}

#[test]
fn elements_are_queryable_by_kind() {
    let (_dir, path) = tmp();
    let doc = Document::from_model(sample_house());
    save(&path, &doc).unwrap();
    let c = rusqlite::Connection::open(&path).unwrap();
    let walls: i64 = c
        .query_row(
            "SELECT COUNT(*) FROM elements WHERE kind = 'Wall'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let expected = doc
        .model()
        .iter()
        .filter(|e| matches!(e.kind, ElementKind::Wall(_)))
        .count();
    assert!(walls > 0);
    assert_eq!(walls as usize, expected);
    let name: String = c
        .query_row(
            "SELECT json_extract(data, '$.name') FROM elements WHERE kind = 'Level' ORDER BY ord",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(name, "Ground Floor");
}

#[test]
fn a_file_from_a_newer_version_is_refused_clearly() {
    let (_dir, path) = tmp();
    save(&path, &Document::from_model(sample_house())).unwrap();
    rusqlite::Connection::open(&path)
        .unwrap()
        .pragma_update(None, "user_version", 99)
        .unwrap();
    let err = open(&path).unwrap_err();
    assert!(
        matches!(
            err,
            StoreError::NewerFormat {
                found: 99,
                supported: 1
            }
        ),
        "{err:?}"
    );
    assert!(err.to_string().contains("newer CreaBIM"));
}

/// Run SQL against a file and close it again (Windows will not replace a
/// file that is still open).
fn sql(path: &Path, statement: &str) {
    rusqlite::Connection::open(path)
        .unwrap()
        .execute_batch(statement)
        .unwrap();
}

#[test]
fn damaged_or_foreign_files_give_errors_not_panics() {
    let (dir, path) = tmp();

    std::fs::write(
        &path,
        b"this is not a database, just some text padding it out",
    )
    .unwrap();
    assert!(matches!(open(&path), Err(StoreError::NotAProject)));

    let other = dir.path().join("other.db");
    rusqlite::Connection::open(&other)
        .unwrap()
        .execute_batch("CREATE TABLE t (x)")
        .unwrap();
    assert!(matches!(open(&other), Err(StoreError::NotAProject)));

    save(&path, &edited_sample()).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let cut = dir.path().join("cut.creabim");
    std::fs::write(&cut, &bytes[..bytes.len() / 2]).unwrap();
    assert!(open(&cut).is_err());

    sql(&path, "UPDATE elements SET data = '{oops' WHERE ord = 0");
    assert!(matches!(open(&path), Err(StoreError::Corrupt(_))));

    save(&path, &edited_sample()).unwrap();
    sql(&path, "UPDATE ops SET step = 1 WHERE action = 'undo'");
    assert!(matches!(open(&path), Err(StoreError::Corrupt(_))));

    save(&path, &edited_sample()).unwrap();
    // drop the undo row: the journal now claims "Delete wall" is applied
    sql(
        &path,
        "DELETE FROM ops WHERE seq = (SELECT MAX(seq) FROM ops)",
    );
    // a history that no longer fits the model is cut; the model opens
    let (doc, mended) = open_mended(&path).expect("the model opens");
    assert!(mended.is_some(), "and says the history was cut");
    assert_eq!(doc.model(), open(&path).unwrap().model());

    assert!(matches!(
        open(dir.path().join("missing.creabim")),
        Err(StoreError::Io(_))
    ));
    assert!(!dir.path().join("missing.creabim").exists());
}

#[test]
fn saving_over_a_project_that_is_open_is_refused() {
    let (_dir, path) = tmp();
    let doc = Document::from_model(sample_house());
    let store = Store::create(&path, &doc).unwrap();
    assert!(save(&path, &edited_sample()).is_err());
    store.close().unwrap();
    // the original is intact
    assert_same(&open(&path).unwrap(), &doc);
}

#[test]
fn json_projects_import_and_load_dispatches_by_extension() {
    let (dir, path) = tmp();
    let json = dir.path().join("house.creabim.json");
    let model = sample_house();
    std::fs::write(&json, model.to_json()).unwrap();
    let doc = import_json(&json).unwrap();
    assert_eq!(doc.model().len(), model.len());
    assert!(!doc.can_undo());
    save(&path, &doc).unwrap();
    assert_eq!(load(&path).unwrap().model(), doc.model());
    assert_eq!(load(&json).unwrap().model(), doc.model());
    assert!(matches!(
        load(dir.path().join("x.json")),
        Err(StoreError::Io(_))
    ));
}

#[test]
fn meta_records_the_project() {
    let (_dir, path) = tmp();
    let doc = Document::from_model(sample_house());
    let mut store = Store::create(&path, &doc).unwrap();
    store.set_project_name("Maison").unwrap();
    let meta = store.meta().unwrap();
    assert_eq!(meta.project_name, "Maison");
    assert_eq!(meta.format_version, creabim_store::FORMAT_VERSION);
    assert_eq!(meta.app_version, env!("CARGO_PKG_VERSION"));
    assert!(meta.created_ms > 0 && meta.modified_ms >= meta.created_ms);
}

#[test]
fn loading_the_sample_is_quick_to_do() {
    let (_dir, path) = tmp();
    let doc = Document::from_model(sample_house());
    save(&path, &doc).unwrap();
    for _ in 0..20 {
        assert_eq!(open(&path).unwrap().model().len(), doc.model().len());
    }
}

#[test]
fn the_material_library_round_trips_with_its_history() {
    let (_dir, path) = tmp();
    let mut doc = Document::from_model(sample_house());
    let incoming = creabim_core::library::LIBRARY
        .iter()
        .map(|e| (e.name.to_string(), e.material()))
        .collect();
    let (tx, merged, ids) = creabim_core::library::merge(doc.model(), incoming, "Add materials");
    assert_eq!(merged.added, creabim_core::library::LIBRARY.len());
    doc.commit(tx).unwrap();
    let (tx, n) = creabim_core::library::assign(doc.model(), &[first_wall(&doc)], Some(ids[0]));
    assert_eq!(n, 1);
    doc.commit(tx).unwrap();
    save(&path, &doc).unwrap();
    let back = open(&path).unwrap();
    assert_same(&doc, &back);
}

#[test]
fn materials_saved_before_the_library_open_unchanged() {
    let (_dir, path) = tmp();
    save(&path, &Document::from_model(sample_house())).unwrap();
    // what a project saved before materials had more than a hatch and a
    // colour holds
    sql(
        &path,
        "UPDATE elements SET data = json_remove(data, '$.kind.Material.category') WHERE kind = 'Material'",
    );
    let raw: String = rusqlite::Connection::open(&path)
        .unwrap()
        .query_row(
            "SELECT json_extract(data, '$.kind') FROM elements WHERE kind = 'Material' ORDER BY ord",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        raw,
        r#"{"Material":{"hatch":"Brick","color":[0.78,0.55,0.45]}}"#
    );
    let back = open(&path).unwrap();
    let m = back
        .model()
        .iter()
        .find_map(|e| match &e.kind {
            ElementKind::Material(m) if e.name == "Brick" => Some(m.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        m,
        creabim_core::Material::new(creabim_core::Hatch::Brick, [0.78, 0.55, 0.45])
    );
    assert_eq!((m.roughness, m.metallic, m.opacity), (0.8, 0.0, 1.0));
}

#[test]
fn saving_keeps_the_file_it_replaces_as_one_backup() {
    let (_dir, path) = tmp();
    let mut doc = Document::from_model(sample_house());
    creabim_store::save_with_backup(&path, &doc).unwrap();
    let bak = creabim_store::backup_path(&path);
    assert!(bak.ends_with("house.bak.creabim"));
    // nothing to back up the first time
    assert!(!bak.exists());
    let before = doc.model().clone();
    add_level(&mut doc, "Second");
    creabim_store::save_with_backup(&path, &doc).unwrap();
    assert_eq!(load(&bak).unwrap().model(), &before);
    assert_eq!(load(&path).unwrap().model(), doc.model());
    // a third save replaces the one backup
    add_level(&mut doc, "Third");
    let second = load(&path).unwrap().model().clone();
    creabim_store::save_with_backup(&path, &doc).unwrap();
    assert_eq!(load(&bak).unwrap().model(), &second);
    let files = std::fs::read_dir(path.parent().unwrap()).unwrap().count();
    assert_eq!(
        files, 3,
        "the project, its backup and the older saves' folder, nothing left over"
    );
}

#[test]
fn an_autosave_newer_than_its_project_is_found() {
    let (_dir, path) = tmp();
    let doc = Document::from_model(sample_house());
    save(&path, &doc).unwrap();
    assert!(creabim_store::newer_autosave(&path).is_none());
    let auto = creabim_store::autosave_path(&path);
    std::thread::sleep(std::time::Duration::from_millis(20));
    save(&auto, &doc).unwrap();
    assert_eq!(creabim_store::newer_autosave(&path), Some(auto));
}

#[test]
fn the_project_information_reads_without_the_model() {
    let (_dir, path) = tmp();
    let mut doc = Document::from_model(sample_house());
    save(&path, &doc).unwrap();
    assert!(creabim_store::read_project_info(&path).unwrap().is_none());
    let info = creabim_core::ProjectInfo {
        name: "Rodinný dom".into(),
        number: "24-017".into(),
        client: "Ján Novák".into(),
        ..Default::default()
    };
    let tx = creabim_core::set_project(doc.model(), info.clone());
    doc.commit(tx).unwrap();
    save(&path, &doc).unwrap();
    let read = creabim_store::read_project_info(&path).unwrap().unwrap();
    assert_eq!(
        (read.name, read.number, read.client),
        (info.name, info.number, info.client)
    );
}

/// Every sample project (between them every kind of element the samples
/// build) saves and opens back the same, history and all; saving again
/// keeps the file it replaces as the backup and the older ones in their
/// folder.
#[test]
fn every_sample_saves_and_opens_back_the_same() {
    use creabim_core::sample;
    let models = [
        sample::sample_house(),
        sample::layered_house(),
        sample::typed_house(),
        sample::furnished_house(),
        sample::sample_with_buried_beam().0,
    ];
    for (i, m) in models.into_iter().enumerate() {
        let (dir, path) = tmp();
        let mut doc = Document::from_model(m);
        let wall = doc
            .model()
            .iter()
            .find(|e| matches!(e.kind, ElementKind::Wall(_)))
            .map(|e| e.id);
        add_level(&mut doc, "Podkrovie");
        if let Some(w) = wall {
            doc.edit(w, "Wall thickness", set_thickness(0.45)).unwrap();
        }
        doc.undo();
        save(&path, &doc).unwrap();
        let back = open(&path).unwrap();
        assert_same(&doc, &back);
        let (_, mended) = open_mended(&path).unwrap();
        assert!(mended.is_none(), "sample {i}: nothing to mend");
        drop(dir);
    }
}

#[test]
fn saves_keep_a_backup_and_the_older_saves() {
    let (_dir, path) = tmp();
    let mut doc = edited_sample();
    for n in 0..(creabim_store::OLDER_BACKUPS + 4) {
        add_level(&mut doc, &format!("L{n}"));
        creabim_store::save_with_backup(&path, &doc).unwrap();
        // distinct times for the older saves' names
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(creabim_store::backup_path(&path).exists());
    let older = creabim_store::older_backups(&path);
    assert_eq!(older.len(), creabim_store::OLDER_BACKUPS);
    for o in &older {
        open(o).expect("every older save opens");
    }
    assert_same(&doc, &open(&path).unwrap());
}

/// A history that no longer fits the model (a merged edit once lost the
/// boards it added): the project still opens, the history is cut, and
/// saved again the file is whole.
#[test]
fn a_project_whose_history_lost_an_insert_opens_and_saves_whole() {
    let (_dir, path) = tmp();
    let mut doc = Document::from_model(sample_house());
    let l = add_level(&mut doc, "Podkrovie");
    // a wall on the new level, then made thicker
    let mut wall = doc.model().get(first_wall(&doc)).unwrap().clone();
    wall.id = creabim_core::ElementId::new();
    if let ElementKind::Wall(w) = &mut wall.kind {
        w.level = l;
    }
    let id = wall.id;
    doc.commit(Transaction::new("Add wall up").insert(wall))
        .unwrap();
    doc.edit(id, "Wall thickness", set_thickness(0.5)).unwrap();
    save(&path, &doc).unwrap();
    // the journal loses the wall's insert: undoing the level would now
    // take it from under the wall
    sql(&path, "DELETE FROM ops WHERE label = 'Add wall up'");
    let (back, mended) = open_mended(&path).expect("it opens");
    let m = mended.expect("and says what was cut");
    assert!(m.undo_dropped >= 1 && m.elements_dropped == 0);
    assert_eq!(back.model(), doc.model());
    creabim_store::save_with_backup(&path, &back).unwrap();
    let (_, again) = open_mended(&path).unwrap();
    assert!(again.is_none(), "saved again, nothing left to mend");
}

/// An element whose reference is gone is left out; the rest opens.
#[test]
fn an_element_with_a_missing_reference_is_left_out_not_the_project() {
    let (_dir, path) = tmp();
    let doc = edited_sample();
    save(&path, &doc).unwrap();
    // the first level goes from under everything standing on it
    let level = doc
        .model()
        .iter()
        .find(|e| matches!(e.kind, ElementKind::Level(_)))
        .unwrap()
        .id;
    let _ = level;
    sql(
        &path,
        "DELETE FROM elements WHERE ord = (SELECT MIN(ord) FROM elements WHERE kind = 'Level')",
    );
    let (back, mended) = open_mended(&path).expect("it opens");
    let m = mended.expect("and says what was left out");
    assert!(m.elements_dropped > 0);
    assert!(back.model().len() < doc.model().len());
}
