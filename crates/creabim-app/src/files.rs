//! Opening, saving, importing and exporting, and watching linked files so
//! a model saved in Blender shows up here without asking.

use creabim_core::Model;
use creabim_exchange::Incoming;
use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant, SystemTime};

/// The kinds of file that come in as linked objects.
pub const LINKABLE: &[&str] = &["blend", "fcstd"];

fn ext(path: &str) -> String {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

/// Is this a project file (opened in place of the current project)?
pub fn is_project(path: &str) -> bool {
    path.to_ascii_lowercase().ends_with(".creabim") || ext(path) == "json"
}

pub fn is_linkable(path: &str) -> bool {
    LINKABLE.contains(&ext(path).as_str())
}

/// The objects in a linked file.
pub fn read_incoming(path: &str) -> Result<Vec<Incoming>, String> {
    match ext(path).as_str() {
        "blend" => creabim_exchange::blend::read_file(Path::new(path)).map_err(|e| e.to_string()),
        "fcstd" => creabim_exchange::freecad::read_file(Path::new(path)).map_err(|e| e.to_string()),
        other => Err(format!("CreaBIM can't read .{other} files yet.")),
    }
}

/// The linked files the model mirrors.
pub fn linked_files(model: &Model) -> Vec<String> {
    let mut files: Vec<String> = model
        .iter()
        .filter_map(|e| e.source.as_ref().map(|s| s.file.clone()))
        .filter(|f| is_linkable(f))
        .collect();
    files.sort();
    files.dedup();
    files
}

/// Watches linked files for saves.
#[derive(Default)]
pub struct Watch {
    seen: HashMap<String, Option<SystemTime>>,
    last: Option<Instant>,
}

impl Watch {
    /// Remember a file as it is now, e.g. right after reading it.
    pub fn mark(&mut self, file: &str) {
        self.seen.insert(file.to_string(), modified(file));
    }

    /// Files saved since they were last seen. Looks at most once a second.
    pub fn changed(&mut self, model: &Model) -> Vec<String> {
        if self
            .last
            .is_some_and(|t| t.elapsed() < Duration::from_secs(1))
        {
            return vec![];
        }
        self.last = Some(Instant::now());
        let mut out = Vec::new();
        for f in linked_files(model) {
            let now = modified(&f);
            match self.seen.get(&f) {
                None => {
                    self.seen.insert(f, now);
                }
                // a save in progress is left to finish before reading
                Some(before)
                    if *before != now
                        && now.is_some_and(|t| {
                            t.elapsed().unwrap_or_default() > Duration::from_millis(300)
                        }) =>
                {
                    self.seen.insert(f.clone(), now);
                    out.push(f);
                }
                _ => {}
            }
        }
        out
    }
}

fn modified(file: &str) -> Option<SystemTime> {
    std::fs::metadata(file).and_then(|m| m.modified()).ok()
}

/// Files opened or saved lately, newest first, kept in the user's settings
/// folder between sessions.
#[derive(Debug, Default)]
pub struct Recent {
    pub files: Vec<String>,
    path: Option<std::path::PathBuf>,
}

const RECENT_MAX: usize = 10;

impl Recent {
    /// Read the list from `<settings>/CreaBIM/recent.json`, where settings
    /// is %APPDATA% on Windows and $XDG_CONFIG_HOME or ~/.config elsewhere.
    pub fn load() -> Self {
        let path = settings_dir().map(|d| d.join("recent.json"));
        let files = path
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|t| serde_json::from_str::<Vec<String>>(&t).ok())
            .unwrap_or_default();
        Self { files, path }
    }

    /// Put `file` at the top of the list and save it.
    pub fn add(&mut self, file: &str) {
        let full = std::fs::canonicalize(file)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| file.to_string());
        self.files.retain(|f| *f != full);
        self.files.insert(0, full);
        self.files.truncate(RECENT_MAX);
        self.store();
    }

    pub fn clear(&mut self) {
        self.files.clear();
        self.store();
    }

    fn store(&self) {
        if let Some(p) = &self.path {
            if let Some(dir) = p.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Ok(t) = serde_json::to_string_pretty(&self.files) {
                let _ = std::fs::write(p, t);
            }
        }
    }
}

/// Interface choices kept between sessions in `<settings>/CreaBIM/settings.json`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: String,
    /// The interface's language: "sk", "cs" or "en" (see `crate::i18n`).
    pub language: String,
    /// Width of the project browser and of the properties panel, in points.
    pub left_width: f32,
    pub right_width: f32,
    /// The browser folded to a rail of its groups (MU27).
    #[serde(default)]
    pub left_folded: bool,
    /// Properties folded to its tab column (MU27).
    #[serde(default)]
    pub right_folded: bool,
    /// One view at a time instead of the 3D view and a drawing side by
    /// side, and which of the two it shows.
    pub single_view: bool,
    pub single_shows_3d: bool,
    /// The ground grid in the 3D view, and its spacing in metres.
    pub ground_grid: bool,
    pub ground_grid_spacing: f64,
    /// The drawings' paper and the 3D background (A4-3): "theme" follows
    /// the theme (dark paper in a dark theme), "light" or "dark" always.
    /// Prints and PDFs are always on white.
    pub drawing_paper: String,
    /// The mouse (MU19): "creabim" (left selects and boxes, middle
    /// orbits, right or Shift + middle pans) or "classic" (left orbits).
    pub navigation: String,
    /// How the last IFC export wrote walls, slabs and roofs: "modelled"
    /// (recommended), "joined" or "layers" (MU20).
    pub ifc_body: String,
    /// Temporary dimensions muted in drawings (MU21).
    pub temp_dims_off: bool,
    /// What temporary dimensions measure walls from (core faces by default).
    pub temp_ref: creabim_core::walls::WallRef,
    /// Thin lines on screen (TL): every pen one pixel wide.
    pub thin_lines: bool,
    /// The kinds of element the 3D view leaves out (grids, annotation,
    /// guides…), from its Visibility settings.
    pub hidden_3d: Vec<creabim_core::Category>,
    /// The 3D view in X-ray: walls, slabs, roofs and masses see-through, so
    /// what is inside them shows.
    pub xray: bool,
    /// The 3D view shows what is hidden inside walls, slabs and roofs in its
    /// own colours, faintly through what hides it.
    pub hidden_colour_3d: bool,
    /// Guides hidden everywhere (they are kept, and come back when shown).
    pub guides_hidden: bool,
    /// How the 3D view draws: shaded, with edges, hidden line or wireframe.
    pub render_3d: crate::render3d::RenderMode,
    /// The 3D view's sun and its shadows.
    pub sun: crate::sun::Sun,
    /// Width of the Help panel, in points.
    pub help_width: f32,
    /// Which sections of Properties are open, by key, where it differs
    /// from how each opens at first.
    #[serde(skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub props_open: std::collections::BTreeMap<String, bool>,
    /// The tab the tools row shows: Model, Annotate, View or Manage.
    pub tools_tab: usize,
    /// The workspace last picked, by name, and the areas as they were left.
    pub workspace: String,
    pub areas: Option<crate::areas::Node>,
    /// Layouts saved with *Save current layout*.
    pub workspaces: Vec<crate::areas::Workspace>,
    /// How grips look (brief 02, 2.13): a personal choice.
    pub grips: crate::handles::GripStyle,
    /// Distance marks while dragging (the Grid snap menu).
    pub distance_marks: bool,
    /// The grid snap's and the distance marks' spacing, micrometres. Grid
    /// snap itself always starts off.
    pub snap_spacing_um: u32,
    /// Where Move, Rotate, Scale and the like show their bar: a slim pill
    /// at the top of the view, or a small palette by the pointer (MU38).
    pub edit_bar: String,
    /// The model's X and Y axes drawn red and green on the 3D ground
    /// grid, as Blender's (MU39).
    pub axis_lines: bool,
    /// The light millimetre grid behind plans, sections and details; off,
    /// plain paper (MU40).
    pub plan_grid: bool,
    /// The colour of what is worked on (selection, grips, ghosts): "orange",
    /// the default, or "theme" for the theme's accent (MU39).
    pub interaction: String,
    /// Edit mode's wires and corner dots, in points.
    pub edit_edge_width: f32,
    /// Minutes between autosaves while there are changes; 0 is off.
    pub autosave_minutes: u32,
    pub edit_vertex_size: f32,
    /// The drawings' typeface: a TrueType or OpenType file in place of
    /// the built-in sans; empty looks for Avenir Next Condensed beside
    /// the program and among the system's fonts.
    pub drawing_font: String,
    /// The width of the drawings' letters: 0.7 is 70 % of the face's own.
    pub text_width: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            // the approved look (brief 02); anyone's saved theme stays theirs
            theme: "sage".into(),
            language: "sk".into(),
            autosave_minutes: 5,
            left_width: 264.0,
            right_width: 290.0,
            left_folded: false,
            right_folded: false,
            single_view: false,
            single_shows_3d: true,
            ground_grid: true,
            ground_grid_spacing: 1.0,
            drawing_paper: "theme".into(),
            navigation: "creabim".into(),
            ifc_body: "modelled".into(),
            temp_dims_off: false,
            temp_ref: Default::default(),
            thin_lines: false,
            hidden_3d: vec![],
            xray: false,
            hidden_colour_3d: false,
            guides_hidden: false,
            render_3d: Default::default(),
            sun: Default::default(),
            help_width: 400.0,
            props_open: Default::default(),
            tools_tab: 0,
            workspace: "Drawing".into(),
            areas: None,
            workspaces: vec![],
            grips: Default::default(),
            distance_marks: false,
            snap_spacing_um: 50_000,
            edit_bar: "top".into(),
            axis_lines: true,
            plan_grid: true,
            interaction: "orange".into(),
            edit_edge_width: 1.0,
            edit_vertex_size: 4.0,
            drawing_font: String::new(),
            text_width: 0.7,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        settings_dir()
            .and_then(|d| std::fs::read_to_string(d.join("settings.json")).ok())
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn store(&self) {
        // tests change settings without touching the user's own
        if cfg!(test) {
            return;
        }
        if let Some(dir) = settings_dir() {
            let _ = std::fs::create_dir_all(&dir);
            if let Ok(t) = serde_json::to_string_pretty(self) {
                let _ = std::fs::write(dir.join("settings.json"), t);
            }
        }
    }
}

pub fn settings_dir() -> Option<std::path::PathBuf> {
    let base = std::env::var_os("APPDATA")
        .or_else(|| std::env::var_os("XDG_CONFIG_HOME"))
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join(".config")))?;
    Some(base.join("CreaBIM"))
}

/// The file of the tables kept in the application (the norms' tables
/// the designer saved once for every project).
pub fn app_tables_path() -> Option<std::path::PathBuf> {
    settings_dir().map(|d| d.join("tabulky-noriem.tsv"))
}

/// Reads the application's tables, if they were saved.
pub fn load_app_tables() {
    let tables = app_tables_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|t| creabim_core::fireparams::parse(&t))
        .unwrap_or_default();
    creabim_core::fireparams::set_app_tables(tables);
}

/// The main window, kept so that every file picker is owned by it.
#[cfg(windows)]
static MAIN_WINDOW: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

/// Remembers the main window (called every frame), so the system's file
/// pickers open as its modal dialogs: on top of it, with the app window
/// disabled meanwhile. A picker without an owner can fall behind the app
/// window, and so can its "replace the file?" question, while the app
/// waits on it: nothing could be typed or clicked anymore.
pub fn remember_main_window(frame: &eframe::Frame) {
    #[cfg(windows)]
    {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        if let Ok(h) = frame.window_handle() {
            if let RawWindowHandle::Win32(w) = h.as_raw() {
                MAIN_WINDOW.store(w.hwnd.get(), std::sync::atomic::Ordering::Relaxed);
            }
        }
    }
    #[cfg(not(windows))]
    let _ = frame;
}

#[cfg(windows)]
struct Owner(std::num::NonZeroIsize);

#[cfg(windows)]
impl raw_window_handle::HasWindowHandle for Owner {
    fn window_handle(
        &self,
    ) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        let raw = raw_window_handle::RawWindowHandle::Win32(
            raw_window_handle::Win32WindowHandle::new(self.0),
        );
        // the main window lives as long as the app does
        Ok(unsafe { raw_window_handle::WindowHandle::borrow_raw(raw) })
    }
}

#[cfg(windows)]
impl raw_window_handle::HasDisplayHandle for Owner {
    fn display_handle(
        &self,
    ) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        Ok(raw_window_handle::DisplayHandle::windows())
    }
}

/// The system's file picker, owned by the main window (see
/// [`remember_main_window`]); use it instead of `rfd::FileDialog::new()`.
pub fn file_dialog() -> rfd::FileDialog {
    let d = rfd::FileDialog::new();
    #[cfg(windows)]
    {
        let hwnd = MAIN_WINDOW.load(std::sync::atomic::Ordering::Relaxed);
        if let Some(h) = std::num::NonZeroIsize::new(hwnd) {
            return d.set_parent(&Owner(h));
        }
    }
    d
}

/// Ask for a file to open with the system's own file picker.
pub fn pick_open() -> Option<String> {
    file_dialog()
        .set_title(crate::i18n::tr("Open or link a file"))
        .add_filter(
            crate::i18n::tr("Everything CreaBIM reads"),
            &[
                "creabim", "json", "ifc", "ifcx", "blend", "FCStd", "fcstd", "FCMat", "fcmat",
                "gltf", "glb",
            ],
        )
        .add_filter(crate::i18n::tr("CreaBIM project"), &["creabim"])
        .add_filter("IFC", &["ifc", "ifcx"])
        .add_filter("Blender and FreeCAD (linked)", &["blend", "FCStd", "fcstd"])
        .add_filter(
            "Materials (FreeCAD cards, glTF)",
            &["FCMat", "fcmat", "gltf", "glb"],
        )
        .pick_file()
        .map(|p| p.display().to_string())
}

/// Show `dir` in the system's file browser.
pub fn open_folder(dir: &Path) -> Result<(), String> {
    if cfg!(test) {
        return Ok(());
    }
    let program = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    std::process::Command::new(program)
        .arg(dir)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("couldn't open {}: {e}", dir.display()))
}

/// Ask where to save, starting from `current`.
pub fn pick_save(current: &str, title: &str, ext: &str) -> Option<String> {
    // a project not saved yet starts in the folder CreaBIM was started in
    let abs = std::path::absolute(current).unwrap_or_else(|_| current.into());
    pick_save_in(&abs, &crate::i18n::tr(title), ext)
}

#[cfg(windows)]
fn pick_save_in(p: &Path, title: &str, ext: &str) -> Option<String> {
    win_save::pick(p, title, ext)
}

#[cfg(not(windows))]
fn pick_save_in(p: &Path, title: &str, ext: &str) -> Option<String> {
    let mut d = file_dialog()
        .set_title(title)
        .add_filter(ext.to_uppercase(), &[ext]);
    if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
        d = d.set_file_name(name);
    }
    if let Some(dir) = p.parent().filter(|d| d.is_dir()) {
        d = d.set_directory(dir);
    }
    d.save_file().map(|p| p.display().to_string())
}

/// Save as on Windows: the system's own picker (IFileSaveDialog), owned by
/// the main window, but with its "replace the file?" question switched off
/// (FOS_OVERWRITEPROMPT): that question opened behind the app window and
/// held everything. CreaBIM asks itself, with a message box kept on top of
/// every window. (GetSaveFileNameW kept the question on, whatever its
/// flags said.)
#[cfg(windows)]
mod win_save {
    use std::ffi::c_void;
    use std::path::Path;
    use windows_sys::core::{GUID, HRESULT, PCWSTR, PWSTR};
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_INPROC_SERVER,
        COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
    };
    use windows_sys::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
    use windows_sys::Win32::UI::Shell::{
        FileSaveDialog, SHCreateItemFromParsingName, FOS_FORCEFILESYSTEM, FOS_NOCHANGEDIR,
        FOS_OVERWRITEPROMPT, FOS_PATHMUSTEXIST, SIGDN, SIGDN_FILESYSPATH,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        MessageBoxW, SetForegroundWindow, IDYES, MB_DEFBUTTON2, MB_ICONWARNING, MB_SETFOREGROUND,
        MB_TOPMOST, MB_YESNO,
    };

    const IID_IFILESAVEDIALOG: GUID = GUID::from_u128(0x84bccd23_5fde_4cdb_aea4_af64b83d78ab);
    const IID_ISHELLITEM: GUID = GUID::from_u128(0x43826d1e_e718_42ee_bc55_a1e261c37bfe);

    type Release = unsafe extern "system" fn(*mut c_void) -> u32;

    /// IFileSaveDialog's table, as far as it is used (IUnknown, IModalWindow,
    /// IFileDialog in their order; the slots not called as plain words).
    #[repr(C)]
    struct DialogVt {
        _query_interface: usize,
        _add_ref: usize,
        release: Release,
        show: unsafe extern "system" fn(*mut c_void, HWND) -> HRESULT,
        set_file_types:
            unsafe extern "system" fn(*mut c_void, u32, *const COMDLG_FILTERSPEC) -> HRESULT,
        _set_file_type_index: usize,
        _get_file_type_index: usize,
        _advise: usize,
        _unadvise: usize,
        set_options: unsafe extern "system" fn(*mut c_void, u32) -> HRESULT,
        get_options: unsafe extern "system" fn(*mut c_void, *mut u32) -> HRESULT,
        _set_default_folder: usize,
        set_folder: unsafe extern "system" fn(*mut c_void, *mut c_void) -> HRESULT,
        _get_folder: usize,
        _get_current_selection: usize,
        set_file_name: unsafe extern "system" fn(*mut c_void, PCWSTR) -> HRESULT,
        _get_file_name: usize,
        set_title: unsafe extern "system" fn(*mut c_void, PCWSTR) -> HRESULT,
        _set_ok_button_label: usize,
        _set_file_name_label: usize,
        get_result: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> HRESULT,
        _add_place: usize,
        set_default_extension: unsafe extern "system" fn(*mut c_void, PCWSTR) -> HRESULT,
    }

    #[repr(C)]
    struct ItemVt {
        _query_interface: usize,
        _add_ref: usize,
        release: Release,
        _bind_to_handler: usize,
        _get_parent: usize,
        get_display_name: unsafe extern "system" fn(*mut c_void, SIGDN, *mut PWSTR) -> HRESULT,
    }

    /// A COM object, released when dropped.
    struct Com<V>(*mut *const V);

    impl<V> Com<V> {
        fn vt(&self) -> &V {
            unsafe { &**self.0 }
        }
        fn this(&self) -> *mut c_void {
            self.0.cast()
        }
    }

    impl<V> Drop for Com<V> {
        fn drop(&mut self) {
            // every table starts with IUnknown's: release is the third slot
            unsafe {
                let release: Release = *(*self.0).cast::<Release>().add(2);
                release(self.this());
            }
        }
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn owner() -> HWND {
        super::MAIN_WINDOW.load(std::sync::atomic::Ordering::Relaxed) as _
    }

    pub fn pick(start: &Path, title: &str, ext: &str) -> Option<String> {
        let hwnd = owner();
        let mut name: String = start
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        let dir = start
            .parent()
            .filter(|d| d.is_dir())
            .map(|d| d.to_path_buf());
        loop {
            let picked = unsafe {
                let init = CoInitializeEx(
                    std::ptr::null(),
                    (COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) as u32,
                );
                let picked = show(hwnd, dir.as_deref(), &name, title, ext);
                if init >= 0 {
                    CoUninitialize();
                }
                picked
            }?;
            if !Path::new(&picked).exists() || replace(hwnd, &picked, title) {
                return Some(picked);
            }
            // not replaced: the picker again, on the name chosen
            name = Path::new(&picked)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
        }
    }

    unsafe fn show(
        hwnd: HWND,
        dir: Option<&Path>,
        name: &str,
        title: &str,
        ext: &str,
    ) -> Option<String> {
        unsafe {
            let mut raw: *mut c_void = std::ptr::null_mut();
            if CoCreateInstance(
                &FileSaveDialog,
                std::ptr::null_mut(),
                CLSCTX_INPROC_SERVER,
                &IID_IFILESAVEDIALOG,
                &mut raw,
            ) < 0
                || raw.is_null()
            {
                return None;
            }
            let d: Com<DialogVt> = Com(raw.cast());
            let vt = d.vt();
            let mut opts = 0u32;
            (vt.get_options)(d.this(), &mut opts);
            let opts = (opts & !FOS_OVERWRITEPROMPT)
                | FOS_FORCEFILESYSTEM
                | FOS_PATHMUSTEXIST
                | FOS_NOCHANGEDIR;
            (vt.set_options)(d.this(), opts);
            let title_w = wide(title);
            (vt.set_title)(d.this(), title_w.as_ptr());
            let label = wide(&format!("{} (*.{ext})", ext.to_uppercase()));
            let spec = wide(&format!("*.{ext}"));
            let filter = COMDLG_FILTERSPEC {
                pszName: label.as_ptr(),
                pszSpec: spec.as_ptr(),
            };
            (vt.set_file_types)(d.this(), 1, &filter);
            let def_ext = wide(ext);
            (vt.set_default_extension)(d.this(), def_ext.as_ptr());
            let name_w = wide(name);
            (vt.set_file_name)(d.this(), name_w.as_ptr());
            if let Some(dir) = dir {
                let dir_w = wide(&dir.display().to_string());
                let mut item: *mut c_void = std::ptr::null_mut();
                if SHCreateItemFromParsingName(
                    dir_w.as_ptr(),
                    std::ptr::null_mut(),
                    &IID_ISHELLITEM,
                    &mut item,
                ) >= 0
                    && !item.is_null()
                {
                    let item: Com<ItemVt> = Com(item.cast());
                    (vt.set_folder)(d.this(), item.this());
                }
            }
            if !hwnd.is_null() {
                SetForegroundWindow(hwnd);
            }
            if (vt.show)(d.this(), hwnd) < 0 {
                return None;
            }
            let mut res: *mut c_void = std::ptr::null_mut();
            if (vt.get_result)(d.this(), &mut res) < 0 || res.is_null() {
                return None;
            }
            let item: Com<ItemVt> = Com(res.cast());
            let mut text: PWSTR = std::ptr::null_mut();
            if (item.vt().get_display_name)(item.this(), SIGDN_FILESYSPATH, &mut text) < 0
                || text.is_null()
            {
                return None;
            }
            let mut len = 0;
            while *text.add(len) != 0 {
                len += 1;
            }
            let path = String::from_utf16_lossy(std::slice::from_raw_parts(text, len));
            CoTaskMemFree(text.cast());
            Some(path)
        }
    }

    fn replace(hwnd: HWND, path: &str, title: &str) -> bool {
        let file = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(path);
        let text = format!(
            "{}\n\n{file}",
            crate::i18n::tr("The file already exists. Replace it?")
        );
        let text = wide(&text);
        let title = wide(title);
        unsafe {
            MessageBoxW(
                hwnd,
                text.as_ptr(),
                title.as_ptr(),
                MB_YESNO | MB_ICONWARNING | MB_DEFBUTTON2 | MB_SETFOREGROUND | MB_TOPMOST,
            ) == IDYES
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_from_before_the_layout_options_still_read() {
        let s: Settings = serde_json::from_str(r#"{"theme": "paper", "left_width": 300.0}"#)
            .expect("old settings read");
        assert_eq!(s.theme, "paper");
        assert!(!s.single_view);
        assert!(s.ground_grid);
        assert_eq!(s.ground_grid_spacing, 1.0);
        assert!(s.hidden_3d.is_empty() && !s.xray && !s.guides_hidden && !s.hidden_colour_3d);
        assert_eq!(s.render_3d, crate::render3d::RenderMode::Shaded);
        assert!(s.props_open.is_empty());
    }

    #[test]
    fn folded_properties_sections_round_trip_and_none_writes_nothing() {
        let mut s = Settings::default();
        s.props_open.insert("visibility.view".into(), false);
        s.props_open.insert("visibility.3d".into(), true);
        let back: Settings = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back.props_open, s.props_open);
        let plain = serde_json::to_string(&Settings::default()).unwrap();
        assert!(!plain.contains("props_open"));
    }

    #[test]
    fn the_3d_views_visibility_round_trips_through_the_settings() {
        let s = Settings {
            hidden_3d: vec![
                creabim_core::Category::Grids,
                creabim_core::Category::Guides,
            ],
            xray: true,
            hidden_colour_3d: true,
            guides_hidden: true,
            ground_grid_spacing: 1.2,
            render_3d: crate::render3d::RenderMode::Wireframe,
            ..Default::default()
        };
        let back: Settings = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn recent_files_keep_the_newest_first_without_repeats() {
        let mut r = Recent::default();
        for f in ["a.creabim", "b.creabim", "a.creabim"] {
            r.add(f);
        }
        assert_eq!(r.files.len(), 2);
        assert!(r.files[0].ends_with("a.creabim"));
        for i in 0..20 {
            r.add(&format!("{i}.creabim"));
        }
        assert_eq!(r.files.len(), RECENT_MAX);
    }
}
