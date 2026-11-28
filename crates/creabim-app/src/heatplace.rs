//! Placing a heat source step by step (George 2026-11-27): Place it in
//! the Heat source window tucks the open windows away as small tabs side
//! by side in the bottom right of the workspace, so the plan is clear;
//! the indoor unit is placed first, the pointer saying so, then the
//! outdoor unit that goes with it; after both, the windows come back
//! where they were, in the order they lay over each other. A boiler, a
//! cylinder or a heat pump of one piece is one step. Esc ends it all.

use super::CreaBimApp;
use super::Tool;
use creabim_core::catalogue::heat::{self, HeatKind};
use creabim_core::objects::CatalogueObject;
use eframe::egui::{self, emath::TSTransform, LayerId, Order, Vec2};

/// One unit to place, and what the pointer says while it is.
#[derive(Clone, Debug)]
pub(super) struct Step {
    pub note: String,
    pub unit: CatalogueObject,
}

/// A window tucked away: its layer, its title, and whether its tab has
/// brought it back for a while.
#[derive(Clone, Debug)]
pub(super) struct Tucked {
    layer: LayerId,
    title: String,
    shown: bool,
}

#[derive(Clone, Debug)]
pub(super) enum Windows {
    /// Waiting a frame for the windows' titles (AccessKit's tree is built
    /// from the next frame on); whether AccessKit was off before.
    Reading { frames: u8, was_off: bool },
    /// Out of the way, bottom to top as they lay.
    Tucked(Vec<Tucked>),
}

/// The placing under way.
#[derive(Clone, Debug)]
pub(super) struct HeatPlace {
    steps: Vec<Step>,
    at: usize,
    /// The Object tool's count of placed objects when this step began.
    placed: u64,
    windows: Windows,
    /// The tool bar's tab when it began, to come back to.
    tab: usize,
}

/// Far enough off the screen for any window.
const AWAY: f32 = 100_000.0;

/// The units `kind` of `size` is placed as, in order: the indoor unit
/// first, then the outdoor one that goes with it; one unit for the rest.
pub(super) fn pair(kind: HeatKind, size: u32, midea_indoor: HeatKind) -> Vec<(HeatKind, u32)> {
    let indoor = match kind {
        HeatKind::HpOutdoor => Some(HeatKind::HpIndoor),
        HeatKind::HpOutdoorMidea => Some(midea_indoor),
        HeatKind::HpIndoor
        | HeatKind::HpIndoorCylinder
        | HeatKind::HpIndoorMidea
        | HeatKind::HpIndoorMideaTank190
        | HeatKind::HpIndoorMideaTank240 => Some(kind),
        HeatKind::Cylinder | HeatKind::Boiler | HeatKind::BoilerCombi => None,
    };
    match indoor {
        None => vec![(kind, kind.size(size))],
        Some(i) => {
            let outdoor = if i.is_midea() {
                HeatKind::HpOutdoorMidea
            } else {
                HeatKind::HpOutdoor
            };
            vec![(i, i.size(size)), (outdoor, outdoor.size(size))]
        }
    }
}

impl CreaBimApp {
    /// Start placing `units` (from [`pair`]).
    pub(super) fn heat_place_start(&mut self, ctx: &egui::Context, units: &[(HeatKind, u32)]) {
        let t = crate::i18n::t;
        let two = units.len() > 1;
        let steps: Vec<Step> = units
            .iter()
            .map(|(k, s)| {
                let unit = heat::unit(*k, *s);
                let what = if !two {
                    t("Placing")
                } else if heat::is_hp_outdoor(*k) {
                    t("Placing the outdoor unit")
                } else {
                    t("Placing the indoor unit")
                };
                Step {
                    note: format!("{what}: {}", unit.name),
                    unit,
                }
            })
            .collect();
        if steps.is_empty() {
            return;
        }
        // a step under way already: its windows stay tucked
        let tab = self.shell.settings.tools_tab;
        let (windows, tab) = match self.heat_place.take() {
            Some(p) => (p.windows, p.tab),
            None => (begin_windows(ctx), tab),
        };
        ctx.memory_mut(|m| m.stop_text_input());
        let mut p = HeatPlace {
            steps,
            at: 0,
            placed: self.plan.objects.placed,
            windows,
            tab,
        };
        self.heat_place_begin_step(&mut p);
        self.heat_place = Some(p);
        ctx.request_repaint();
    }

    fn heat_place_begin_step(&mut self, p: &mut HeatPlace) {
        let s = &p.steps[p.at];
        self.status = format!(
            "{}. {}",
            s.note,
            crate::i18n::t("Click in a plan or the 3D view; Space turns it, Esc ends.")
        );
        self.plan.objects.current = Some(s.unit.clone());
        self.plan.objects.offset = None;
        self.plan.objects.project_type = None;
        self.plan.cancel();
        self.plan.tool = Tool::Object;
        p.placed = self.plan.objects.placed;
    }

    /// Each frame, after the windows: tuck them away, follow the steps,
    /// say at the pointer what is being placed, bring the windows back.
    pub(super) fn heat_place_frame(&mut self, ctx: &egui::Context) {
        self.restore_order_step(ctx);
        let Some(mut p) = self.heat_place.take() else {
            if self.parked.is_none() {
                bring_back_left_over(ctx);
            }
            return;
        };
        read_windows(ctx, &mut p.windows);
        // a click placed it: the next one, or the end
        let mut done = false;
        if self.plan.objects.placed != p.placed {
            p.at += 1;
            if p.at < p.steps.len() {
                self.heat_place_begin_step(&mut p);
            } else {
                done = true;
                self.plan.tool = Tool::Select;
                self.plan.objects.current = None;
                self.status = crate::i18n::t("Heat source placed.").to_string();
            }
        }
        // Esc, or another tool or object taken: it ends there
        let key = &p.steps[p.at.min(p.steps.len() - 1)].unit.key;
        let esc = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        if !done
            && (esc
                || self.plan.tool != Tool::Object
                || self.plan.objects.current.as_ref().map(|c| &c.key) != Some(key))
        {
            done = true;
            if self.plan.tool == Tool::Object && esc {
                self.plan.tool = Tool::Select;
            }
        }
        if done {
            self.shell.settings.tools_tab = p.tab;
            self.untuck(ctx, &p.windows);
            return;
        }
        if let Windows::Tucked(w) = &mut p.windows {
            tabs(
                ctx,
                w,
                "Shows the window for a while; it comes back where it was once the heat source is placed.",
            );
        }
        pointer_note(ctx, &p.steps[p.at].note);
        self.heat_place = Some(p);
    }

    /// Bring the tucked windows back where they were, and their order.
    pub(super) fn untuck(&mut self, ctx: &egui::Context, w: &Windows) {
        match w {
            Windows::Reading { was_off, .. } => {
                if *was_off {
                    ctx.disable_accesskit();
                }
            }
            Windows::Tucked(w) => {
                for t in w {
                    ctx.set_transform_layer(t.layer, TSTransform::IDENTITY);
                }
                // one a frame, bottom first: each goes on top of the last
                self.restore_order = w.iter().map(|t| t.layer).collect();
            }
        }
        ctx.request_repaint();
    }

    fn restore_order_step(&mut self, ctx: &egui::Context) {
        if self.restore_order.is_empty() {
            return;
        }
        let l = self.restore_order.remove(0);
        ctx.move_to_top(l);
        ctx.request_repaint();
    }
}

/// Windows left out of the way when the app closed while placing (egui
/// keeps their move in its memory): back where they were.
fn bring_back_left_over(ctx: &egui::Context) {
    let away: Vec<LayerId> =
        ctx.memory(|m| m.layer_ids().filter(|l| l.order == Order::Middle).collect());
    for l in away {
        if ctx
            .layer_transform_to_global(l)
            .is_some_and(|t| t.translation.x >= AWAY * 0.5)
        {
            ctx.set_transform_layer(l, TSTransform::IDENTITY);
        }
    }
}

/// Starts reading the open windows (their titles come with AccessKit's
/// tree from the next frame on).
pub(super) fn begin_windows(ctx: &egui::Context) -> Windows {
    let was_off = ctx
        .accesskit_node_builder(egui::Id::new("heat.place.probe"), |_| ())
        .is_none();
    ctx.enable_accesskit();
    Windows::Reading { frames: 0, was_off }
}

/// Once a frame: the windows, read and tucked away once they have drawn
/// with AccessKit on.
pub(super) fn read_windows(ctx: &egui::Context, w: &mut Windows) {
    if let Windows::Reading { frames, was_off } = w {
        *frames += 1;
        ctx.request_repaint();
        if *frames >= 2 {
            let was_off = *was_off;
            *w = Windows::Tucked(tuck(ctx));
            if was_off {
                ctx.disable_accesskit();
            }
        }
    }
}

/// The windows open now, bottom to top, moved off the screen.
fn tuck(ctx: &egui::Context) -> Vec<Tucked> {
    let layers: Vec<LayerId> = ctx.memory(|m| {
        m.layer_ids()
            .filter(|l| l.order == Order::Middle)
            .filter(|l| m.areas().visible_last_frame(l))
            .filter(|l| m.areas().parent_layer(*l).is_none())
            .collect()
    });
    let mut out = vec![];
    for layer in layers {
        if egui::AreaState::load(ctx, layer.id).is_none() {
            continue;
        }
        let title = ctx
            .accesskit_node_builder(layer.id.with("move"), |n| n.label().map(str::to_string))
            .flatten()
            .unwrap_or_default();
        if title.is_empty() {
            // not a window of the app's (no title bar to read)
            continue;
        }
        ctx.set_transform_layer(layer, TSTransform::from_translation(Vec2::splat(AWAY)));
        out.push(Tucked {
            layer,
            title,
            shown: false,
        });
    }
    out
}

/// The tucked windows' tabs, side by side in the bottom right of the
/// workspace; a click on one brings its window back for a while (and
/// again tucks it away).
pub(super) fn tabs(ctx: &egui::Context, w: &mut [Tucked], hint: &str) {
    if w.is_empty() {
        return;
    }
    let room = super::shell::window_room(ctx);
    let gap = 6.0;
    egui::Area::new(egui::Id::new("heat.place.tabs"))
        .order(Order::Foreground)
        .pivot(egui::Align2::RIGHT_BOTTOM)
        .fixed_pos(room.right_bottom() - Vec2::splat(10.0))
        .interactable(true)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for t in w.iter_mut() {
                    let mut frame = egui::Frame::window(ui.style())
                        .inner_margin(egui::Margin::symmetric(10, 6))
                        .corner_radius(4.0);
                    if t.shown {
                        frame = frame
                            .stroke(egui::Stroke::new(1.5, ui.visuals().selection.stroke.color));
                    }
                    let r = frame
                        .show(ui, |ui| {
                            ui.set_max_width(220.0);
                            ui.add(
                                egui::Label::new(egui::RichText::new(&t.title).strong())
                                    .truncate()
                                    .selectable(false)
                                    .sense(egui::Sense::click()),
                            )
                        })
                        .inner
                        .on_hover_text(crate::i18n::t(hint));
                    if r.clicked() {
                        t.shown = !t.shown;
                        let to = if t.shown {
                            TSTransform::IDENTITY
                        } else {
                            TSTransform::from_translation(Vec2::splat(AWAY))
                        };
                        ctx.set_transform_layer(t.layer, to);
                        if t.shown {
                            ctx.move_to_top(t.layer);
                        }
                    }
                }
            });
        });
}

/// What is being placed, on a light plate beside the pointer, when it
/// is over a view (not over a window).
pub(super) fn pointer_note(ctx: &egui::Context, text: &str) {
    let Some(at) = ctx.pointer_hover_pos() else {
        return;
    };
    if ctx
        .layer_id_at(at)
        .is_some_and(|l| l.order != Order::Background)
    {
        return;
    }
    let painter = ctx.layer_painter(LayerId::new(
        Order::Tooltip,
        egui::Id::new("heat.place.note"),
    ));
    let ink = egui::Color32::from_rgb(20, 60, 140);
    let font = egui::FontId::proportional(13.0);
    let galley = painter.layout_no_wrap(text.to_owned(), font, ink);
    let clip = ctx.content_rect();
    // under the ghost's own label and the snap's name
    let mut pos = at + Vec2::new(18.0, 34.0);
    if pos.x + galley.size().x + 10.0 > clip.right() {
        pos.x = at.x - galley.size().x - 18.0;
    }
    if pos.y + galley.size().y + 8.0 > clip.bottom() {
        pos.y = at.y - galley.size().y - 20.0;
    }
    let rect = egui::Rect::from_min_size(pos, galley.size()).expand2(Vec2::new(6.0, 4.0));
    painter.rect(
        rect,
        3.0,
        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 240),
        egui::Stroke::new(1.0, ink.gamma_multiply(0.6)),
        egui::StrokeKind::Middle,
    );
    painter.galley(pos, galley, ink);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_split_is_its_indoor_unit_then_its_outdoor_one() {
        let mi = HeatKind::HpIndoorMidea;
        assert_eq!(
            pair(HeatKind::HpOutdoorMidea, 8, HeatKind::HpIndoorMideaTank240),
            vec![
                (HeatKind::HpIndoorMideaTank240, 8),
                (HeatKind::HpOutdoorMidea, 8)
            ]
        );
        assert_eq!(
            pair(mi, 10, mi),
            vec![(mi, 10), (HeatKind::HpOutdoorMidea, 10)]
        );
        assert_eq!(
            pair(HeatKind::HpOutdoor, 9, mi),
            vec![(HeatKind::HpIndoor, 9), (HeatKind::HpOutdoor, 9)]
        );
        assert_eq!(
            pair(HeatKind::HpIndoorCylinder, 7, mi),
            vec![(HeatKind::HpIndoorCylinder, 7), (HeatKind::HpOutdoor, 7)]
        );
    }

    #[test]
    fn a_boiler_or_a_cylinder_is_one_unit() {
        let mi = HeatKind::HpIndoorMidea;
        assert_eq!(pair(HeatKind::Boiler, 20, mi), vec![(HeatKind::Boiler, 20)]);
        assert_eq!(
            pair(HeatKind::BoilerCombi, 25, mi),
            vec![(HeatKind::BoilerCombi, 25)]
        );
        assert_eq!(
            pair(HeatKind::Cylinder, 200, mi),
            vec![(HeatKind::Cylinder, 200)]
        );
    }
}
