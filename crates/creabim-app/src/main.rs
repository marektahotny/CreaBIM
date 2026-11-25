//! CreaBIM desktop app.

// A released app opens its window alone, with no terminal beside it
// (users took the console for a bug). Debug builds keep the console for
// what they print while being worked on.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod actlog;
mod align3d;
mod aligntool;
mod annotate;
mod app;
mod areas;
mod boxsel;
mod browser;
mod builds;
mod camera;
mod centres;
mod choices;
mod commands;
mod detailtool;
mod drawfont;
mod editmode;
mod files;
mod fillpreview;
mod flashingui;
mod ghost;
mod handles;
mod help;
mod hoverpick;
mod i18n;
mod icons;
mod memberstool;
mod modal;
mod modguides;
mod multiedit;
mod objecttool;
mod optionbar;
mod outline;
mod paneface;
mod partstool;
mod pickup;
mod pipesui;
mod placement;
mod planview;
mod printing;
mod props;
mod render3d;
mod roomtool;
mod schedulepane;
mod selection;
mod sheet;
mod sheetview;
mod sketch;
mod slabtool;
mod snaps;
mod sun;
mod svgicon;
mod takeoffpane;
mod tempdims;
mod textures;
mod theme;
mod tooltabs;
mod trace;
mod tracker;
mod ui_model;
mod units;
mod viewcube;
mod webui;

/// The window's and the taskbar's icon: the app tile, sage.
fn app_icon() -> std::sync::Arc<eframe::egui::IconData> {
    let png = include_bytes!("../assets/logo/png/creabim-app-icon-256.png");
    std::sync::Arc::new(eframe::icon_data::from_png_bytes(png).unwrap_or_default())
}

fn main() -> eframe::Result {
    // the window's thread answers first on a busy machine
    creabim_geom::prio::ui();
    // a crash says why in the activity log, with where it happened (MU39:
    // one closed the app with nothing to go on)
    let before = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let trace = std::backtrace::Backtrace::force_capture();
        actlog::line(format!(
            "CRASH: {info}
{trace}"
        ));
        before(info);
    }));
    #[cfg(target_os = "linux")]
    linux_setup();
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("CreaBIM")
            .with_inner_size([1400.0, 860.0])
            // opens over the whole screen (George 2026-11-25); a dev shot
            // keeps its fixed size
            .with_maximized(std::env::var_os("CREABIM_SHOT").is_none())
            .with_icon(app_icon()),
        renderer: eframe::Renderer::Wgpu,
        // the windows' sizes and places are kept for the next run (egui's
        // memory, George 2026-11-23); the main window keeps its own
        // (files::remember_main_window)
        persist_window: false,
        #[cfg(target_os = "linux")]
        event_loop_builder: Some(Box::new(|b| {
            use winit::platform::x11::EventLoopBuilderExtX11;
            b.with_x11();
        })),
        ..Default::default()
    };
    #[cfg(target_os = "macos")]
    let options = metal(options);
    eframe::run_native(
        "CreaBIM",
        options,
        Box::new(|cc| Ok(Box::new(app::CreaBimApp::new(cc)))),
    )
}

/// On a Mac the window and the 3D view draw through Metal, Apple's own
/// GPU interface (the only one Apple Silicon runs natively), on the GPU
/// that draws the screen; `WGPU_BACKEND` still overrides it.
#[cfg(target_os = "macos")]
fn metal(mut o: eframe::NativeOptions) -> eframe::NativeOptions {
    use eframe::egui_wgpu::{wgpu, WgpuSetup};
    if let WgpuSetup::CreateNew(s) = &mut o.wgpu_options.wgpu_setup {
        if std::env::var_os("WGPU_BACKEND").is_none() {
            s.instance_descriptor.backends = wgpu::Backends::METAL;
        }
    }
    o
}

/// The web panes are GTK child windows of an X11 window, so on Wayland the
/// app runs through XWayland, and GTK has to be started first.
#[cfg(target_os = "linux")]
fn linux_setup() {
    // Without this WebKit draws nothing on some GPUs and in virtual displays.
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
    std::env::set_var("GDK_BACKEND", "x11");
    if gtk::init().is_err() {
        // The app still runs, with the classic panels.
        std::env::set_var("CREABIM_CLASSIC_UI", "1");
    }
}
