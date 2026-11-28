//! Where an underlay's file is (a map's orthophoto and site plan, a DXF,
//! a PDF, a picture, a point cloud): kept relative to the project file
//! when it lies in the project's folder, so the project still finds its
//! underlays when the folder is moved or opened from a USB stick
//! (George 2026-11-23).
//!
//! A path saved before this was absolute (`D:\Architektura\…\ortofoto.jpg`);
//! where that is gone, the file is looked for under the project's folder
//! by the path's own tail (`…-mapy-1795290772\ortofoto.jpg`), longest
//! tail first.

use crate::{Element, ElementKind, Model, RefSource};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

/// The folder of the project open now: what relative paths start from.
static BASE: RwLock<Option<PathBuf>> = RwLock::new(None);

/// The project's folder, from the project file's path; none for a
/// project never saved.
pub fn folder_of(project: &str) -> Option<PathBuf> {
    if project.is_empty() {
        return None;
    }
    let dir = Path::new(project).parent()?;
    let dir = if dir.as_os_str().is_empty() {
        std::env::current_dir().ok()?
    } else if dir.is_relative() {
        std::env::current_dir().ok()?.join(dir)
    } else {
        dir.to_path_buf()
    };
    Some(dir)
}

/// The project open now is in `dir`.
pub fn set_base(dir: Option<PathBuf>) {
    if let Ok(mut b) = BASE.write() {
        if *b != dir {
            *b = dir;
        }
    }
}

/// The folder relative paths start from now.
pub fn base() -> Option<PathBuf> {
    BASE.read().ok().and_then(|b| b.clone())
}

/// A path's parts, split at either slash (a Windows path read anywhere).
fn parts(p: &str) -> Vec<&str> {
    p.split(['/', '\\'])
        .filter(|s| !s.is_empty() && *s != ".")
        .collect()
}

/// Absolute on any system: `/…`, `\\server\…` or `C:\…`.
pub fn is_absolute(p: &str) -> bool {
    let b = p.as_bytes();
    p.starts_with('/')
        || p.starts_with('\\')
        || (b.len() >= 2 && b[1] == b':' && b[0].is_ascii_alphabetic())
}

/// The file a saved path names, read from `base` (the project's folder).
pub fn resolve_in(stored: &str, base: Option<&Path>) -> PathBuf {
    if stored.is_empty() {
        return PathBuf::new();
    }
    if !is_absolute(stored) {
        let join = |b: &Path| {
            let mut p = b.to_path_buf();
            for s in parts(stored) {
                p.push(s);
            }
            p
        };
        let Some(base) = base else {
            return join(Path::new(""));
        };
        // a copy opened from a folder inside the project's (its backups)
        return base
            .ancestors()
            .take(3)
            .map(join)
            .find(|p| p.is_file())
            .unwrap_or_else(|| join(base));
    }
    let own = PathBuf::from(stored);
    if own.exists() {
        return own;
    }
    // moved with the project: the same tail under the project's folder
    if let Some(base) = base {
        let ps = parts(stored);
        // the drive (C:) is never part of a tail
        let from = usize::from(ps.first().is_some_and(|d| d.ends_with(':')));
        for start in from..ps.len() {
            let mut p = base.to_path_buf();
            for s in &ps[start..] {
                p.push(s);
            }
            if p.is_file() {
                return p;
            }
        }
    }
    own
}

/// The file a saved path names, from the project open now.
pub fn resolve(stored: &str) -> PathBuf {
    resolve_in(stored, base().as_deref())
}

/// [`resolve`] as text, for readers that take a `&str`.
pub fn resolved(stored: &str) -> String {
    resolve(stored).to_string_lossy().into_owned()
}

/// How `file` is saved in a project in `base`: relative (with `/`) when
/// it is in the project's folder, else as it is.
pub fn stored_in(file: &Path, base: &Path) -> String {
    match file.strip_prefix(base) {
        Ok(rel) if !rel.as_os_str().is_empty() => rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/"),
        _ => file.to_string_lossy().into_owned(),
    }
}

/// The file path of an underlay, to change.
pub fn path_mut(source: &mut RefSource) -> Option<&mut String> {
    match source {
        RefSource::View(_) => None,
        RefSource::Image { path, .. }
        | RefSource::Dxf { path, .. }
        | RefSource::Pdf { path, .. }
        | RefSource::Points { path, .. } => Some(path),
    }
}

/// The underlays whose paths change when the project, read from `old`
/// (its folder before), is kept in `new`: each found file relative to
/// `new` when in it, else absolute; a file not found keeps its path.
pub fn rebase(model: &Model, old: Option<&Path>, new: &Path) -> Vec<Element> {
    let mut out = vec![];
    for e in model.iter() {
        let ElementKind::Reference(r) = &e.kind else {
            continue;
        };
        let mut r = r.clone();
        let Some(path) = path_mut(&mut r.source) else {
            continue;
        };
        let file = resolve_in(path, old);
        if !file.is_file() {
            continue;
        }
        let now = stored_in(&file, new);
        if now != *path {
            *path = now;
            let mut after = e.clone();
            after.kind = ElementKind::Reference(r);
            out.push(after);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Reference, Transaction};

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("creabim-refpath-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn windows_paths_are_absolute_anywhere() {
        assert!(is_absolute(r"D:\Architektura\a.jpg"));
        assert!(is_absolute("/home/a.jpg"));
        assert!(is_absolute(r"\\server\share\a.jpg"));
        assert!(!is_absolute("mapy/a.jpg"));
    }

    #[test]
    fn an_old_absolute_path_is_found_under_the_moved_project() {
        // the project on a USB stick, its map folder copied with it
        let usb = tmp("usb");
        let maps = usb.join("RD-Balaz_Dlha-mapy-1795290772");
        std::fs::create_dir_all(&maps).unwrap();
        std::fs::write(maps.join("ortofoto.jpg"), b"x").unwrap();
        let old = r"D:\Architektura\RD-BalazDLHA\RD-Balaz_Dlha-mapy-1795290772\ortofoto.jpg";
        assert_eq!(resolve_in(old, Some(&usb)), maps.join("ortofoto.jpg"));
        // and a relative one from wherever the project is
        assert_eq!(
            resolve_in("RD-Balaz_Dlha-mapy-1795290772/ortofoto.jpg", Some(&usb)),
            maps.join("ortofoto.jpg")
        );
        // a backup in a folder of the project's finds them too
        let backups = usb.join("RD-Balaz_Dlha.zalohy");
        std::fs::create_dir_all(&backups).unwrap();
        assert_eq!(
            resolve_in("RD-Balaz_Dlha-mapy-1795290772/ortofoto.jpg", Some(&backups)),
            maps.join("ortofoto.jpg")
        );
        // nothing there: left as it was
        assert_eq!(
            resolve_in(r"D:\x\nothing.jpg", Some(&usb)),
            PathBuf::from(r"D:\x\nothing.jpg")
        );
        let _ = std::fs::remove_dir_all(&usb);
    }

    #[test]
    fn saving_keeps_underlays_in_the_folder_relative() {
        let dir = tmp("save");
        std::fs::create_dir_all(dir.join("mapy")).unwrap();
        std::fs::write(dir.join("mapy").join("situacia.dxf"), b"0\nEOF\n").unwrap();
        let outside = tmp("outside").join("plan.pdf");
        std::fs::write(&outside, b"%PDF").unwrap();
        let mut model = Model::new();
        let inside = Element::new(
            "Situácia",
            ElementKind::Reference(Reference::new(
                RefSource::Dxf {
                    path: dir
                        .join("mapy")
                        .join("situacia.dxf")
                        .to_string_lossy()
                        .into(),
                    unit: Some(1.0),
                },
                vec![],
            )),
        );
        let far = Element::new(
            "PDF",
            ElementKind::Reference(Reference::new(
                RefSource::Pdf {
                    path: outside.to_string_lossy().into(),
                    page: 1,
                    scale: 100.0,
                },
                vec![],
            )),
        );
        let (a, b) = (inside.id, far.id);
        Transaction::new("refs")
            .insert(inside)
            .insert(far)
            .apply(&mut model)
            .unwrap();
        let changed = rebase(&model, Some(&dir), &dir);
        assert_eq!(changed.len(), 1);
        assert_eq!(changed[0].id, a);
        let ElementKind::Reference(r) = &changed[0].kind else {
            unreachable!()
        };
        assert!(matches!(&r.source, RefSource::Dxf { path, .. } if path == "mapy/situacia.dxf"));
        assert!(!changed.iter().any(|e| e.id == b));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(outside.parent().unwrap());
    }
}
