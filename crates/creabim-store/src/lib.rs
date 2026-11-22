//! The `.creabim` project file.
//!
//! A project is a SQLite database holding the current elements and the
//! journal of every edit (see [`schema`] for the tables). Two ways to use it:
//!
//! * [`save`] / [`open`]: write or read a whole [`Document`], model and undo /
//!   redo stacks, in one go. Saving is atomic: the file is built next to the
//!   target and renamed over it, so an interrupted save leaves the old file.
//! * [`Store`]: keep the file open and call [`Store::record`] after every
//!   commit, undo or redo. Only the change is written, inside one SQLite
//!   transaction in WAL mode, which makes autosave cheap and crash-safe.
//!
//! Older `.creabim.json` files are read with [`import_json`]; [`load`]
//! accepts either format by extension.

pub mod schema;
mod store;

pub use schema::FORMAT_VERSION;
pub use store::Store;

use creabim_core::{Document, Model, ModelError, Op, Step, Transaction};
use rusqlite::{params, Connection, ErrorCode, OptionalExtension};
use std::path::{Path, PathBuf};

/// Extension of project files, without the dot.
pub const EXTENSION: &str = "creabim";

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("database error: {0}")]
    Sqlite(rusqlite::Error),
    #[error("not a CreaBIM project file")]
    NotAProject,
    #[error(
        "this project was saved by a newer CreaBIM (file format {found}; \
         this version reads up to {supported}). Please update CreaBIM."
    )]
    NewerFormat { found: i32, supported: i32 },
    #[error("the project file is damaged: {0}")]
    Corrupt(String),
    #[error("invalid project JSON: {0}")]
    Json(String),
    #[error("the saved history does not fit the model: {0}")]
    History(#[from] ModelError),
    #[error("{} is open elsewhere; close it before saving over it", .0.display())]
    InUse(PathBuf),
}

impl From<rusqlite::Error> for StoreError {
    /// SQLite reports a damaged or foreign file through error codes; surface
    /// those as such instead of a generic database error.
    fn from(e: rusqlite::Error) -> Self {
        match e.sqlite_error_code() {
            Some(ErrorCode::NotADatabase) => StoreError::NotAProject,
            Some(ErrorCode::DatabaseCorrupt) => StoreError::Corrupt(e.to_string()),
            _ => StoreError::Sqlite(e),
        }
    }
}

pub type Result<T, E = StoreError> = std::result::Result<T, E>;

/// Facts about a project kept in the `meta` table. Times are Unix milliseconds.
#[derive(Clone, Debug, PartialEq)]
pub struct Meta {
    pub format_version: i32,
    /// Version of the CreaBIM that last wrote the file.
    pub app_version: String,
    pub created_ms: i64,
    pub modified_ms: i64,
    pub project_name: String,
}

/// Write `doc` (model and undo/redo history) to `path`, replacing any file
/// there. Fails with [`StoreError::InUse`] rather than replace a file a
/// [`Store`] still has open.
pub fn save(path: impl AsRef<Path>, doc: &Document) -> Result<()> {
    save_fresh(path.as_ref(), doc).map(|_| ())
}

/// The one backup kept beside a project: the file as it was before the
/// last save (`House.creabim` → `House.bak.creabim`).
pub fn backup_path(path: impl AsRef<Path>) -> PathBuf {
    beside(path.as_ref(), "bak")
}

/// Where the project is saved on its own every few minutes while it has
/// changes (`House.creabim` → `House.autosave.creabim`); gone once the
/// project is saved.
pub fn autosave_path(path: impl AsRef<Path>) -> PathBuf {
    beside(path.as_ref(), "autosave")
}

fn beside(path: &Path, tag: &str) -> PathBuf {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".into());
    path.with_file_name(format!("{stem}.{tag}.creabim"))
}

/// [`save`], keeping the file it replaces as the project's one backup
/// ([`backup_path`]). The backup is only replaced once the new file is
/// written in full.
pub fn save_with_backup(path: impl AsRef<Path>, doc: &Document) -> Result<()> {
    let path = path.as_ref();
    let bak = backup_path(path);
    let mut tmp = bak.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    let had = path.exists() && std::fs::copy(path, &tmp).is_ok();
    match save(path, doc) {
        Ok(()) => {
            if had {
                // the backup before goes among the older ones
                keep_older_backup(path, &bak);
                let _ = std::fs::rename(&tmp, &bak);
            }
            Ok(())
        }
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            Err(e)
        }
    }
}

/// How many older saves are kept besides the backup ([`backups_dir`]).
pub const OLDER_BACKUPS: usize = 10;

/// Where the older saves of a project are kept: a folder beside it
/// (`CreaBIM zálohy`), each named after the project and when it was saved
/// (`House.1763775600000.creabim`), the newest [`OLDER_BACKUPS`] only.
pub fn backups_dir(path: impl AsRef<Path>) -> PathBuf {
    path.as_ref()
        .parent()
        .unwrap_or(Path::new("."))
        .join("CreaBIM zálohy")
}

/// The older saves of `path` kept in [`backups_dir`], oldest first.
pub fn older_backups(path: impl AsRef<Path>) -> Vec<PathBuf> {
    let path = path.as_ref();
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let prefix = format!("{stem}.");
    let mut found: Vec<(i64, PathBuf)> = std::fs::read_dir(backups_dir(path))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let ms = name
                .strip_prefix(&prefix)?
                .strip_suffix(&format!(".{EXTENSION}"))?
                .parse()
                .ok()?;
            Some((ms, e.path()))
        })
        .collect();
    found.sort();
    found.into_iter().map(|(_, p)| p).collect()
}

/// Move the backup `bak` of `path` among its older saves, and let the
/// oldest beyond [`OLDER_BACKUPS`] go. Never fails a save.
fn keep_older_backup(path: &Path, bak: &Path) {
    if !bak.exists() {
        return;
    }
    let dir = backups_dir(path);
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let ms = std::fs::metadata(bak)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or_else(now_ms, |d| d.as_millis() as i64);
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".into());
    let to = dir.join(format!("{stem}.{ms}.{EXTENSION}"));
    if std::fs::rename(bak, &to).is_err() && std::fs::copy(bak, &to).is_err() {
        return;
    }
    let older = older_backups(path);
    for old in older.iter().take(older.len().saturating_sub(OLDER_BACKUPS)) {
        let _ = std::fs::remove_file(old);
    }
}

/// Is there an autosave of `path` newer than the file itself (left by a
/// crash or a close without saving)?
pub fn newer_autosave(path: impl AsRef<Path>) -> Option<PathBuf> {
    let path = path.as_ref();
    let auto = autosave_path(path);
    let when = |p: &Path| std::fs::metadata(p).and_then(|m| m.modified()).ok();
    let a = when(&auto)?;
    match when(path) {
        Some(f) if f >= a => None,
        _ => Some(auto),
    }
}

/// Read a project: the model plus the undo and redo stacks it was saved with.
pub fn open(path: impl AsRef<Path>) -> Result<Document> {
    Ok(open_mended(path)?.0)
}

/// What opening a project had to mend: undo and redo steps of a saved
/// history that no longer fitted the model, dropped (the model is whole).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mended {
    pub undo_dropped: usize,
    pub redo_dropped: usize,
    /// Elements left out: their references were missing (or they were
    /// broken), so the rest could open.
    pub elements_dropped: usize,
}

/// Like [`open`], and says what of the history had to be dropped.
pub fn open_mended(path: impl AsRef<Path>) -> Result<(Document, Option<Mended>)> {
    let mut conn = schema::open_existing(path.as_ref())?;
    let tx = conn.transaction()?;
    let (doc, _, mended) = read_document(&tx)?;
    Ok((doc, mended))
}

/// Like [`load`], and says what of the history had to be dropped.
pub fn load_mended(path: impl AsRef<Path>) -> Result<(Document, Option<Mended>)> {
    let path = path.as_ref();
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("json"))
    {
        Ok((import_json(path)?, None))
    } else {
        open_mended(path)
    }
}

/// Read a legacy `.creabim.json` model. It has no history, so the document
/// starts with empty undo and redo stacks.
pub fn import_json(path: impl AsRef<Path>) -> Result<Document> {
    let s = std::fs::read_to_string(path)?;
    let model = Model::from_json(&s).map_err(StoreError::Json)?;
    Ok(Document::from_model(model))
}

/// Read a project in either format: `*.json` through [`import_json`],
/// anything else as a `.creabim` database.
pub fn load(path: impl AsRef<Path>) -> Result<Document> {
    let path = path.as_ref();
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("json"))
    {
        import_json(path)
    } else {
        open(path)
    }
}

/// The project information of a project file (name, job number, client),
/// read without opening the whole model: for the list of recent projects.
pub fn read_project_info(path: impl AsRef<Path>) -> Result<Option<creabim_core::ProjectInfo>> {
    let conn = schema::open_existing(path.as_ref())?;
    let data: Option<String> = conn
        .query_row(
            "SELECT data FROM elements WHERE kind = 'Project information' LIMIT 1",
            [],
            |r| r.get(0),
        )
        .optional()?;
    let Some(data) = data else {
        return Ok(None);
    };
    let e: creabim_core::Element = from_json(&data)?;
    Ok(match e.kind {
        creabim_core::ElementKind::Project(p) => Some(p),
        _ => None,
    })
}

/// Read the `meta` table of a project file.
pub fn read_meta(path: impl AsRef<Path>) -> Result<Meta> {
    read_meta_from(&schema::open_existing(path.as_ref())?)
}

pub(crate) fn read_meta_from(conn: &Connection) -> Result<Meta> {
    let get = |key: &str| -> Result<String> {
        conn.query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| r.get(0))
            .optional()?
            .ok_or_else(|| StoreError::Corrupt(format!("meta.{key} is missing")))
    };
    let num = |key: &str| -> Result<i64> {
        get(key)?
            .parse()
            .map_err(|_| StoreError::Corrupt(format!("meta.{key} is not a number")))
    };
    Ok(Meta {
        format_version: num("format_version")? as i32,
        app_version: get("app_version")?,
        created_ms: num("created")?,
        modified_ms: num("modified")?,
        project_name: get("project_name")?,
    })
}

pub(crate) fn set_meta(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO meta (key, value) VALUES (?1, ?2)
         ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

/// Mark the file as written now by this build.
pub(crate) fn touch(conn: &Connection) -> Result<()> {
    set_meta(conn, "modified", &now_ms().to_string())?;
    set_meta(conn, "app_version", env!("CARGO_PKG_VERSION"))
}

pub(crate) fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

/// `house.creabim` -> `house`.
fn project_name(path: &Path) -> String {
    let file = path
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_default();
    let suffix = format!(".{EXTENSION}");
    file.strip_suffix(&suffix).unwrap_or(&file).to_string()
}

/// The undo stack followed by the redo stack reversed: the history as one
/// line, with the document positioned after the first `undo.len()` steps.
pub(crate) fn timeline(doc: &Document) -> impl Iterator<Item = &Step> {
    doc.undo_steps().iter().chain(doc.redo_steps().iter().rev())
}

/// Write a whole document to a new file beside `path`, then rename it into
/// place. Returns the journal row of each timeline step, for [`Store`].
pub(crate) fn save_fresh(path: &Path, doc: &Document) -> Result<Vec<i64>> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(format!(".tmp-{}", std::process::id()));
    let tmp = PathBuf::from(tmp);
    let _ = std::fs::remove_file(&tmp);

    let write = |doc: &Document| -> Result<Vec<i64>> {
        let _ = std::fs::remove_file(&tmp);
        let mut conn = Connection::open(&tmp)?;
        // a plain rollback journal, so the finished file is self-contained
        conn.pragma_update(None, "journal_mode", "DELETE")?;
        let tx = conn.transaction()?;
        let seqs = write_document(&tx, doc, &project_name(path))?;
        // SQLite fsyncs the file on commit (synchronous = FULL by default)
        tx.commit()?;
        conn.close().map_err(|(_, e)| e)?;
        Ok(seqs)
    };
    // never put a file in place that wouldn't open again as it is: read
    // back, it must give the same model with its whole history. If the
    // history is what doesn't fit, the model is saved without it.
    let written = write(doc).and_then(|seqs| match verify(&tmp, doc) {
        Ok(()) => Ok(seqs),
        Err(why) => {
            eprintln!("save: {why}; saved again without the undo history");
            let bare = Document::from_model(doc.model().clone());
            let seqs = write(&bare)?;
            verify(&tmp, &bare).map_err(|why| {
                StoreError::Corrupt(format!(
                    "the file written did not read back ({why}); the saved file was left as it was"
                ))
            })?;
            Ok(seqs)
        }
    });
    let seqs = match written {
        Ok(seqs) => seqs,
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            return Err(e);
        }
    };
    if let Err(e) = retire_sidecars(path).and_then(|()| Ok(std::fs::rename(&tmp, path)?)) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    Ok(seqs)
}

/// Does the file at `path` read back as `doc`: the same model (compared
/// as saved text, so even a value that can't equal itself is caught) and
/// its history whole?
fn verify(path: &Path, doc: &Document) -> std::result::Result<(), String> {
    let mut conn = schema::open_existing(path).map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let (back, _, mended) = read_document(&tx).map_err(|e| e.to_string())?;
    if mended.is_some() {
        return Err("its history did not fit the model".into());
    }
    let text = |m: &Model| serde_json::to_string(m).unwrap_or_default();
    if text(back.model()) != text(doc.model()) {
        return Err("the model read back differs".into());
    }
    if back.undo_steps().len() != doc.undo_steps().len()
        || back.redo_steps().len() != doc.redo_steps().len()
    {
        return Err("the history read back differs".into());
    }
    Ok(())
}

/// Before replacing `path`, fold any WAL or hot journal it has back into it.
/// SQLite finds those sidecar files by name, so left in place they would be
/// replayed onto the new file and corrupt it.
fn retire_sidecars(path: &Path) -> Result<()> {
    let sidecar = |s: &str| {
        let mut p = path.as_os_str().to_owned();
        p.push(s);
        PathBuf::from(p)
    };
    let (wal, journal) = (sidecar("-wal"), sidecar("-journal"));
    if !wal.exists() && !journal.exists() {
        return Ok(());
    }
    // opening rolls back a hot journal; leaving WAL mode checkpoints and
    // deletes the WAL, which only succeeds when nobody else has the file open
    if let Ok(conn) = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE)
    {
        let _ = conn.pragma_update_and_check(None, "journal_mode", "DELETE", |_| Ok(()));
        let _ = conn.close();
    }
    if wal.exists() || journal.exists() {
        return Err(StoreError::InUse(path.to_path_buf()));
    }
    Ok(())
}

/// Fill an empty database with `doc`. The journal gets one commit row per
/// timeline step, then undo rows for the steps currently undone, so replaying
/// it rebuilds both stacks.
fn write_document(conn: &Connection, doc: &Document, name: &str) -> Result<Vec<i64>> {
    schema::create(conn)?;
    let now = now_ms();
    let now_s = now.to_string();
    set_meta(conn, "format_version", &FORMAT_VERSION.to_string())?;
    set_meta(conn, "created", &now_s)?;
    set_meta(conn, "project_name", name)?;
    touch(conn)?;

    let mut insert = conn
        .prepare("INSERT INTO elements (id, ord, kind, name, data) VALUES (?1, ?2, ?3, ?4, ?5)")?;
    for (ord, e) in doc.model().iter().enumerate() {
        insert.execute(params![
            e.id.0.hyphenated().to_string(),
            ord as i64,
            e.kind.name(),
            e.name,
            to_json(e)?,
        ])?;
    }

    let seqs = timeline(doc)
        .map(|s| append_commit(conn, now, &s.tx))
        .collect::<Result<Vec<_>>>()?;
    for &seq in seqs[doc.undo_steps().len()..].iter().rev() {
        append_move(conn, now, Move::Undo, seq)?;
    }
    Ok(seqs)
}

/// Rebuild the document from the `elements` table and the journal. Returns
/// the journal row of each timeline step alongside, for [`Store`].
pub(crate) fn read_document(conn: &Connection) -> Result<(Document, Vec<i64>, Option<Mended>)> {
    let mut stmt = conn.prepare("SELECT data FROM elements ORDER BY ord")?;
    let elements = stmt
        .query_map([], |r| r.get::<_, String>(0))?
        .map(|data| from_json(&data?))
        .collect::<Result<Vec<_>>>()?;
    // never refused for one broken element: it is left out, with what
    // hangs on it, and the rest opens
    let (model, left_out) = Model::from_elements_mended(elements);
    for (id, why) in &left_out {
        eprintln!("open: left out element {id}: {why}");
    }

    // First replay only the row kinds to learn which commits are still on the
    // stacks; steps dropped by a later commit are never parsed.
    let (mut undo, mut redo) = (Vec::<i64>::new(), Vec::<i64>::new());
    let mut stmt = conn.prepare("SELECT seq, action, step FROM ops ORDER BY seq")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let (seq, action, step): (i64, String, Option<i64>) =
            (row.get(0)?, row.get(1)?, row.get(2)?);
        let (from, to) = match action.as_str() {
            "commit" => {
                undo.push(seq);
                redo.clear();
                continue;
            }
            "undo" => (&mut undo, &mut redo),
            "redo" => (&mut redo, &mut undo),
            other => return Err(StoreError::Corrupt(format!("op {seq}: action {other:?}"))),
        };
        match from.pop() {
            Some(top) if Some(top) == step => to.push(top),
            _ => {
                return Err(StoreError::Corrupt(format!(
                    "op {seq}: {action} of a step that is not next"
                )))
            }
        }
    }

    let mut stmt = conn.prepare_cached("SELECT label, forward FROM ops WHERE seq = ?1")?;
    let mut load = |seq: &i64| -> Result<Transaction> {
        let (label, forward): (String, String) =
            stmt.query_row([seq], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(Transaction {
            label,
            ops: from_json(&forward)?,
        })
    };
    let undo_txs = undo.iter().map(&mut load).collect::<Result<Vec<_>>>()?;
    let redo_txs = redo.iter().map(&mut load).collect::<Result<Vec<_>>>()?;
    // a history that no longer fits the model is cut, never the project
    // refused: the model is whole (MU: a merged edit once lost inserts)
    let (doc, undo_dropped, redo_dropped) = Document::from_parts_mended(model, undo_txs, redo_txs);
    let mended = (undo_dropped + redo_dropped + left_out.len() > 0).then_some(Mended {
        undo_dropped,
        redo_dropped,
        elements_dropped: left_out.len(),
    });
    undo.drain(..undo_dropped);
    if redo_dropped > 0 {
        redo.clear();
    }
    undo.extend(redo.iter().rev());
    Ok((doc, undo, mended))
}

/// Append a commit row for `tx`; returns its sequence number.
pub(crate) fn append_commit(conn: &Connection, ts: i64, tx: &Transaction) -> Result<i64> {
    conn.prepare_cached(
        "INSERT INTO ops (ts, action, label, forward, inverse) VALUES (?1, 'commit', ?2, ?3, ?4)",
    )?
    .execute(params![
        ts,
        tx.label,
        to_json(&tx.ops)?,
        to_json(&tx.inverse().ops)?
    ])?;
    Ok(conn.last_insert_rowid())
}

#[derive(Clone, Copy)]
pub(crate) enum Move {
    Undo,
    Redo,
}

/// Append an undo or redo row for the commit at `step`, copying its ops in the
/// direction of travel. Returns the new row and the ops it applies.
pub(crate) fn append_move(
    conn: &Connection,
    ts: i64,
    mv: Move,
    step: i64,
) -> Result<(i64, Vec<Op>)> {
    let (label, forward, inverse): (String, String, String) = conn
        .prepare_cached("SELECT label, forward, inverse FROM ops WHERE seq = ?1")?
        .query_row([step], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
    let (action, forward, inverse) = match mv {
        Move::Undo => ("undo", inverse, forward),
        Move::Redo => ("redo", forward, inverse),
    };
    conn.prepare_cached(
        "INSERT INTO ops (ts, action, label, forward, inverse, step)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?
    .execute(params![ts, action, label, forward, inverse, step])?;
    Ok((conn.last_insert_rowid(), from_json(&forward)?))
}

pub(crate) fn to_json<T: serde::Serialize + ?Sized>(v: &T) -> Result<String> {
    serde_json::to_string(v).map_err(|e| StoreError::Corrupt(e.to_string()))
}

pub(crate) fn from_json<T: serde::de::DeserializeOwned>(s: &str) -> Result<T> {
    serde_json::from_str(s).map_err(|e| StoreError::Corrupt(format!("bad JSON: {e}")))
}
