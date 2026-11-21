//! References (a cadastral map, a geometric plan) on sheets. The app reads
//! their files (geom can't), so it registers a provider that turns the
//! references shown in a view into plan items; sheets lay them under (or
//! over) the view's drawing, cut to what the viewport shows.

use crate::plan::{PlanItem, PlanView};
use creabim_core::{ElementId, Model};
use glam::DVec2;
use std::sync::OnceLock;

/// The references shown in a view as plan items in its model metres:
/// those that go under the model, or with `over`, over it.
pub type Provider = fn(&Model, ElementId, bool) -> Vec<PlanItem>;

static HOOK: OnceLock<Provider> = OnceLock::new();

/// Register the provider; the first call wins.
pub fn set_provider(f: Provider) {
    let _ = HOOK.set(f);
}

/// What `view` shows of its references, under or `over` the model.
pub fn items(model: &Model, view: ElementId, over: bool) -> Vec<PlanItem> {
    HOOK.get().map(|f| f(model, view, over)).unwrap_or_default()
}

/// `d` (as it lies on paper, turned `turn` degrees) with `view`'s
/// references under and over it. Without a `crop` they are cut to the
/// drawing and a margin around it: a map runs on for kilometres.
pub fn with_references(
    model: &Model,
    view: ElementId,
    turn: f64,
    d: PlanView,
    crop: Option<[DVec2; 2]>,
) -> PlanView {
    let (under, over) = (items(model, view, false), items(model, view, true));
    if under.is_empty() && over.is_empty() {
        return d;
    }
    let lay = |items: Vec<PlanItem>| crate::turn::turned(PlanView { items }, turn);
    let (under, over) = (lay(under), lay(over));
    let window = crop.map(|[lo, hi]| (lo, hi)).or_else(|| {
        d.bounds().map(|(lo, hi)| {
            let m = ((hi - lo).max_element() * 0.5).max(20.0);
            (lo - DVec2::splat(m), hi + DVec2::splat(m))
        })
    });
    let cut = |v: PlanView| match window {
        Some((lo, hi)) => crate::crop::crop(&v, lo, hi),
        None => v,
    };
    let mut items = cut(under).items;
    items.extend(d.items);
    items.extend(cut(over).items);
    PlanView { items }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(a: DVec2, b: DVec2) -> PlanItem {
        PlanItem::Curve {
            id: ElementId::new(),
            points: vec![a, b],
            rgb: [0.0; 3],
            dashed: false,
        }
    }

    /// The one view the test's provider knows: no other test's views get
    /// a map.
    static VIEW: OnceLock<ElementId> = OnceLock::new();

    fn map(_: &Model, view: ElementId, over: bool) -> Vec<PlanItem> {
        if Some(&view) != VIEW.get() || over {
            return vec![];
        }
        // a map line running on for a kilometre
        vec![line(DVec2::new(-1000.0, 5.0), DVec2::new(1000.0, 5.0))]
    }

    #[test]
    fn a_map_lies_under_the_drawing_cut_to_a_margin_around_it() {
        let view = *VIEW.get_or_init(ElementId::new);
        set_provider(map);
        // a house 10 m across
        let d = PlanView {
            items: vec![line(DVec2::ZERO, DVec2::new(10.0, 10.0))],
        };
        let m = Model::default();
        let out = with_references(&m, view, 0.0, d.clone(), None);
        assert_eq!(out.items.len(), 2);
        // under the house, 20 m either side of it
        assert_eq!(out.items[1], d.items[0]);
        let (lo, hi) = PlanView {
            items: vec![out.items[0].clone()],
        }
        .bounds()
        .unwrap();
        assert!((lo.x + 20.0).abs() < 1e-9 && (hi.x - 30.0).abs() < 1e-9);
        // a viewport's crop cuts it instead
        let out = with_references(
            &m,
            view,
            0.0,
            d.clone(),
            Some([DVec2::ZERO, DVec2::splat(8.0)]),
        );
        let (lo, hi) = PlanView {
            items: vec![out.items[0].clone()],
        }
        .bounds()
        .unwrap();
        assert!(lo.x.abs() < 1e-9 && (hi.x - 8.0).abs() < 1e-9);
        // another view shows none
        let other = with_references(&m, ElementId::new(), 0.0, d, None);
        assert_eq!(other.items.len(), 1);
    }
}
