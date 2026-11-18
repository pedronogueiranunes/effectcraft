//! #3685: width changes stay in UI/workspace state and never edit layer timing.
use effectcraft_engine::Session;
use effectcraft_ui_egui::EffectcraftApp;
use egui::{Event, Modifiers, PointerButton, Pos2, pos2, vec2};
use egui_kittest::Harness;
use serde_json::json;

fn harness(width: f32) -> Harness<'static, EffectcraftApp> {
    let mut s = Session::default();
    s.execute("comp.new", json!({"width": 320, "height": 180, "duration": 5})).unwrap();
    s.execute("layer.newSolid", json!({"name": "A very long descriptive source name that needs more room", "width": 64, "height": 64})).unwrap();
    let mut h = Harness::builder().with_size(vec2(width, 1000.0)).with_step_dt(1.0 / 60.0).build_eframe(|_| EffectcraftApp::new(s));
    h.run_steps(4);
    h
}

fn rect(h: &Harness<'_, EffectcraftApp>, id: &str) -> [f32; 4] {
    h.state().auto.find(id).unwrap_or_else(|| panic!("missing {id}")).rect
}
fn center(r: [f32; 4]) -> Pos2 {
    pos2(r[0] + r[2] / 2.0, r[1] + r[3] / 2.0)
}
fn capture(h: &mut Harness<'_, EffectcraftApp>, name: &str) {
    if let Ok(dir) = std::env::var("EC_TIMELINE_WIDTH_SHOTS") {
        use crate::gpu_test::RenderSerial;
        h.render_serialized().unwrap().save(format!("{dir}/{name}.png")).unwrap();
    }
}

#[test]
fn timeline_name_width_drag_resizes_both_name_modes_without_editing_the_project() {
    let mut h = harness(1600.0);
    capture(&mut h, "before");
    let initial = rect(&h, "timeline.header.name")[2];
    let before = h.state().session.project.to_json();
    let undo = h.state().session.history.undo.len();
    let start = center(rect(&h, "timeline.nameColumn.edge"));
    h.event(Event::PointerMoved(start));
    h.step();
    h.event(Event::PointerButton { pos: start, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.step();
    let end = start + vec2(140.0, 0.0);
    h.event(Event::PointerMoved(end));
    h.step();
    h.event(Event::PointerButton { pos: end, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(3);
    assert!((rect(&h, "timeline.header.name")[2] - initial - 140.0).abs() < 2.0);
    assert!(!h.state().ui.timeline.source_name, "resizing must not toggle the header mode");
    capture(&mut h, "after");
    let resized = rect(&h, "timeline.header.name")[2];
    h.ctx.memory_mut(|m| m.request_focus(egui::Id::unique("timeline-name-edge")));
    h.event(Event::Key { key: egui::Key::ArrowRight, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE });
    h.run_steps(3);
    assert!((rect(&h, "timeline.header.name")[2] - resized - 10.0).abs() < 2.0, "width: {:?}", h.state().ui.timeline.name_width);
    let ctx = h.ctx.clone();
    effectcraft_ui_egui::menus::invoke(h.state_mut(), &ctx, "timeline.sourceName", json!({"value":true})).unwrap();
    h.run_steps(2);
    assert!((rect(&h, "timeline.header.name")[2] - resized - 10.0).abs() < 2.0);
    assert_eq!(h.state().session.project.to_json(), before);
    assert_eq!(h.state().session.history.undo.len(), undo);
    capture(&mut h, "source-name");
}

#[test]
fn timeline_name_width_command_validates_resets_and_keeps_narrow_graphs_visible() {
    let mut h = harness(720.0);
    let ctx = h.ctx.clone();
    let set = |h: &mut Harness<'_, EffectcraftApp>, p| effectcraft_ui_egui::menus::invoke(h.state_mut(), &ctx, "timeline.nameWidth", p);
    assert_eq!(set(&mut h, json!({"width": 600})).unwrap()["width"], 600.0);
    for bad in [json!(-1), json!(119), json!(1201), json!(1e100), json!("wide")] {
        assert!(set(&mut h, json!({"width": bad})).is_err());
        assert_eq!(set(&mut h, json!({})).unwrap()["width"], 600.0);
    }
    h.run_steps(3);
    let header = rect(&h, "timeline.header");
    assert!(header[2] < 720.0 * 0.75, "room remains for the time graph: {header:?}");
    capture(&mut h, "narrow");
    assert!(set(&mut h, json!({"width": null})).unwrap()["width"].is_null());
    h.run_steps(3);
    assert!(rect(&h, "timeline.header.name")[2].is_finite());
}

#[test]
fn timeline_name_width_double_click_restores_automatic_width() {
    let mut h = harness(1600.0);
    let ctx = h.ctx.clone();
    effectcraft_ui_egui::menus::invoke(h.state_mut(), &ctx, "timeline.nameWidth", json!({"width": 360})).unwrap();
    h.run_steps(3);
    let at = center(rect(&h, "timeline.nameColumn.edge"));
    h.event(Event::PointerMoved(at));
    h.step();
    for _ in 0..2 {
        for pressed in [true, false] {
            h.event(Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
            h.step();
        }
    }
    h.run_steps(3);
    let width = effectcraft_ui_egui::menus::invoke(h.state_mut(), &ctx, "timeline.nameWidth", json!({})).unwrap();
    assert!(width["width"].is_null(), "{width}");
    assert!(!h.state().ui.timeline.source_name);
}
