//! The game's 2D sprite viewport, rebuilt on the current `repame-view2d` pieces.
//!
//! The engine's `Viewport2dGpuWithHud`, `FrameInput`, `FrameGeom`, `GeomHandle`
//! and `pick_world` were deleted when `repame-sprite` merged into
//! `repame-view2d`: the instanced sprite batch, the camera fit law
//! (`effective_fit` / `world_to_dp_with_roll` / `dp_to_world_with_roll`) and the
//! pick vocabulary still ship, but nothing assembles them any more. That
//! assembly is what this module owns:
//!
//! * [`viewport`] fills its parent with the frame's background, draws the
//!   sprites through [`SpriteBatch`] clipped to the contain-fitted world box,
//!   stacks the world texts and the flash tint above them, and reports
//!   [`PickEvent`]s from a transparent surface on top. It publishes
//!   [`FrameGeom`] on every paint, so picks unproject through the exact frame
//!   that was painted, shake and roll included.
//! * [`pick_world`] turns window-physical pointer px back into world coords
//!   through that painted geometry.
//!
//! Unit discipline is unchanged: world units and dp, never physical px. The
//! game does not depend on `repose-canvas`, so the world texts and the tint
//! emit the same [`SceneNode`]s the deleted viewport's HUD half produced from
//! `DrawScope::draw_text_with_family` / `DrawScope::draw_rect`, layer alpha
//! included.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use glam::Vec2;
use repame_view2d::{
    AtlasUpload, BatchDesc, BatchFrame, Camera2d, PickEvent, SpriteBatch, SpriteInstance,
    WorldText, click_within_slop_dp, dp_to_world_with_roll, effective_fit, rgba8,
    world_to_dp_with_roll,
};
use repose_core::input::{PointerButton, PointerEvent, PointerEventKind, PointerKind};
use repose_core::locals::effective_density_scale;
use repose_core::prelude::{Modifier, Px, View};
use repose_core::{Brush, ClipOp, Color, Rect, Scene, SceneNode, TextPaintStyle};
use repose_ui::Box as UiBox;
use repose_ui::ViewExt as _;

/// The sprite pass rebuilds its batch every frame and the engine keys the atlas
/// and instance buffers by batch id, so this string has to stay put.
const BATCH: &str = "viewport2d.main";

/// Physical px per dp at this moment. Read live rather than published: the
/// pointer path needs it at event time, and it is the same value paint measured.
fn density() -> f32 {
    effective_density_scale().max(1e-6)
}

/// Everything the viewport draws this frame.
#[derive(Clone, Default, Debug)]
pub struct FrameInput {
    pub cam: Camera2d,
    /// World-space extent the contain-fit maps into the viewport.
    pub world_size: [f32; 2],
    /// Cold-start viewport size in dp (physical px / density), used until the
    /// first painted rect arrives, exactly as the deleted viewport's prepare did.
    pub viewport_dp: [f32; 2],
    pub sprites: Vec<SpriteInstance>,
    /// World-anchored text, drawn above the sprites.
    pub texts: Vec<WorldText>,
    /// Full-viewport fill under everything (letterbox included).
    pub background: Option<[f32; 4]>,
    /// Fullscreen tint, applied above the sprites and their texts.
    pub overlay_color: Option<[f32; 4]>,
}

/// One painted frame's geometry: the dp fit of the world box, the look point
/// the camera rolls about, and the roll itself. Paint publishes it ahead of the
/// render, so a reader built during composition sees the previous frame; picks
/// read it at event time.
#[derive(Clone, Copy, Debug)]
pub struct FrameGeom {
    /// `(scale, off_x, off_y)` of the world box inside the dp canvas.
    pub fit: (f32, f32, f32),
    /// Camera look point in world coords: `center + offset`.
    pub pivot: [f32; 2],
    /// Camera roll in radians about the look point.
    pub roll: f32,
}

impl Default for FrameGeom {
    fn default() -> Self {
        Self {
            fit: (1.0, 0.0, 0.0),
            pivot: [0.0, 0.0],
            roll: 0.0,
        }
    }
}

/// Shared paint-time geometry. One per viewport; the pick surface reads it, so
/// picks track the frame that was actually painted.
#[derive(Clone, Default)]
pub struct GeomHandle(Rc<Cell<FrameGeom>>);

impl GeomHandle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self) -> FrameGeom {
        self.0.get()
    }

    pub fn set(&self, geom: FrameGeom) {
        self.0.set(geom);
    }
}

/// Window-physical px (y-down) to world, through the painted frame. Picks track
/// the painted (shaken) view by design.
pub fn pick_world(px: [f32; 2], geom: FrameGeom, world_size: [f32; 2]) -> [f32; 2] {
    let d = density();
    dp_to_world_with_roll(
        [px[0] / d, px[1] / d],
        world_size,
        geom.pivot,
        geom.fit,
        geom.roll,
    )
}

/// The dp canvas a painted rect maps to: physical px over density, falling back
/// to the snapshot's own size until the rect has been laid out.
fn canvas_dp(rect: Rect, density: f32, fallback: [f32; 2]) -> [f32; 2] {
    if rect.w > 1.0 && rect.h > 1.0 {
        [rect.w / density, rect.h / density]
    } else {
        fallback
    }
}

/// The clip rectangle of the contain-fitted world box inside `rect`, in
/// physical px: the deleted viewport scissored its batch to this box so sprites
/// could not paint over the letterbox bars, and a clip node is that scissor now.
/// The box rotates with the camera roll about the look point, so the clip is the
/// axis-aligned bounds of the rotated box, held inside `rect` because a painter
/// may only paint inside its own rect. The renderer rounds a clip inward to whole
/// pixels and a clip that rounds away blocks every draw inside it, so a box that
/// is sub-pixel, zero-sized or lands between whole pixels falls back to `rect`,
/// which is the deleted `None => draw unclipped` path.
fn fitted_box(
    rect: Rect,
    world_size: [f32; 2],
    fit: (f32, f32, f32),
    roll: f32,
    density: f32,
) -> Rect {
    let (s, ox, oy) = fit;
    let (bw, bh) = (world_size[0] * s * density, world_size[1] * s * density);
    let cx = rect.x + ox * density + bw * 0.5;
    let cy = rect.y + oy * density + bh * 0.5;
    let (hw, hh) = (bw * 0.5, bh * 0.5);
    let (c, sn) = (roll.cos().abs(), roll.sin().abs());
    let (ex, ey) = (hw * c + hh * sn, hw * sn + hh * c);
    let aabb = Rect {
        x: cx - ex,
        y: cy - ey,
        w: 2.0 * ex,
        h: 2.0 * ey,
    };
    if !aabb.x.is_finite() || !aabb.y.is_finite() || !aabb.w.is_finite() || !aabb.h.is_finite() {
        return rect;
    }
    let x = aabb.x.max(rect.x);
    let y = aabb.y.max(rect.y);
    let clip = Rect {
        x,
        y,
        w: ((aabb.x + aabb.w).min(rect.x + rect.w) - x).max(0.0),
        h: ((aabb.y + aabb.h).min(rect.y + rect.h) - y).max(0.0),
    };
    let x0 = clip.x.ceil();
    let y0 = clip.y.ceil();
    if (clip.x + clip.w).floor() - x0 >= 1.0 && (clip.y + clip.h).floor() - y0 >= 1.0 {
        clip
    } else {
        rect
    }
}

/// Layer alpha on a colour, the multiply `DrawScope` applied to every brush it
/// emitted.
fn alpha_color(color: Color, alpha: f32) -> Color {
    if alpha == 1.0 {
        return color;
    }
    Color(
        color.0,
        color.1,
        color.2,
        (color.3 as f32 * alpha.clamp(0.0, 1.0)) as u8,
    )
}

/// Button carried by a down/up event. Moves have no button, so they report
/// Primary; callers ignore it there.
fn button_of(event: &PointerEvent) -> PointerButton {
    match event.event {
        PointerEventKind::Down(button) | PointerEventKind::Up(button) => button,
        _ => PointerButton::Primary,
    }
}

/// Touch/pen pointers drive the game's touch zones; the mouse stays on the
/// click/hover path.
fn is_contact(event: &PointerEvent) -> bool {
    matches!(event.kind, PointerKind::Touch | PointerKind::Pen)
}

/// Window-physical px (y-down), the space every consumer of [`PickEvent`]
/// expects.
fn screen_of(event: &PointerEvent) -> [f32; 2] {
    let position = event.position_in_window();
    [position.x, position.y]
}

/// Pointer plumbing for [`viewport`]: a painter has no hit channel of its own,
/// so the fit law is replayed here to rebuild [`PickEvent`]s.
struct Picks {
    sink: Rc<dyn Fn(PickEvent)>,
    geom: GeomHandle,
    world_size: [f32; 2],
    press: HashMap<u64, [f32; 2]>,
}

impl Picks {
    /// Window-physical px for `screen` plus the world point under the pointer,
    /// unprojected from the region-local position through the painted geometry.
    fn at(&self, event: &PointerEvent) -> ([f32; 2], Vec2) {
        let local = [event.position.x, event.position.y];
        let [x, y] = pick_world(local, self.geom.get(), self.world_size);
        (screen_of(event), Vec2::new(x, y))
    }
}

/// The viewport: background, sprites clipped to the fitted world box, world
/// texts, flash tint, picks. Fills its parent.
pub fn viewport(
    frame: FrameInput,
    uploads: Arc<[AtlasUpload]>,
    desc: BatchDesc,
    on_event: impl Fn(PickEvent) + 'static,
) -> View {
    let FrameInput {
        cam,
        world_size,
        viewport_dp,
        sprites,
        texts,
        background,
        overlay_color,
    } = frame;

    let geom = GeomHandle::new();
    let painted = geom.clone();
    let surface = UiBox(
        Modifier::new()
            .fill_max_size()
            .hit_passthrough()
            .painter(move |scene: &mut Scene, rect: Rect, alpha: f32| {
                let density = density();
                let canvas_dp = canvas_dp(rect, density, viewport_dp);
                let pivot = cam.effective_center();
                let fit = effective_fit(canvas_dp, world_size, &cam);
                painted.set(FrameGeom {
                    fit,
                    pivot,
                    roll: cam.roll,
                });
                if let Some(fill) = background {
                    scene.nodes.push(SceneNode::Rect {
                        rect,
                        brush: Brush::Solid(alpha_color(rgba8(fill), alpha)),
                        radius: [Px(0.0); 4],
                    });
                }
                let mut batch = SpriteBatch::with_id(BATCH, desc);
                batch.set_camera(cam.fit_matrix(canvas_dp, world_size));
                for sprite in &sprites {
                    batch.push_sprite(sprite);
                }
                // A callback's rect is the viewport its matrix maps into, so the
                // batch fills the surface and the fitted box rides a clip node:
                // the deleted viewport painted the whole surface and scissored
                // to the box, which lands on the same pixels.
                let clip = fitted_box(rect, world_size, fit, cam.roll, density);
                scene.nodes.push(SceneNode::PushClip {
                    rect: clip,
                    radius: [Px(0.0); 4],
                    op: ClipOp::Intersect,
                });
                scene.nodes.push(BatchFrame::new(batch, Arc::clone(&uploads)).node(rect));
                scene.nodes.push(SceneNode::PopClip);
                for text in &texts {
                    let [dx, dy] = world_to_dp_with_roll(
                        [text.pos.x, text.pos.y],
                        world_size,
                        pivot,
                        fit,
                        cam.roll,
                    );
                    let size = Px(text.size * fit.0 * density);
                    scene.nodes.push(SceneNode::Text {
                        rect: Rect {
                            x: rect.x + dx * density,
                            y: rect.y + dy * density,
                            w: 0.0,
                            h: size.0,
                        },
                        text: Arc::<str>::from(text.text.as_str()),
                        color: alpha_color(rgba8(text.color), alpha),
                        size,
                        style: TextPaintStyle {
                            font_family: text.font_family,
                            ..Default::default()
                        },
                    });
                }
                if let Some(tint) = overlay_color {
                    scene.nodes.push(SceneNode::Rect {
                        rect,
                        brush: Brush::Solid(alpha_color(rgba8(tint), alpha)),
                        radius: [Px(0.0); 4],
                    });
                }
            }),
    );

    let state = Rc::new(RefCell::new(Picks {
        sink: Rc::new(on_event),
        geom,
        world_size,
        press: HashMap::new(),
    }));
    let down = state.clone();
    let up = state.clone();
    let moved = state.clone();
    let leave = state.clone();
    let picks = UiBox(
        Modifier::new()
            .fill_max_size()
            .on_pointer_down(move |event: PointerEvent| {
                let mut state = down.borrow_mut();
                let (screen, world) = state.at(&event);
                let button = button_of(&event);
                state.press.insert(event.id.0, screen);
                (state.sink)(PickEvent::Press {
                    world,
                    screen,
                    button,
                });
                if is_contact(&event) {
                    (state.sink)(PickEvent::TouchDown {
                        id: event.id.0,
                        screen,
                    });
                }
            })
            .on_pointer_up(move |event: PointerEvent| {
                let mut state = up.borrow_mut();
                let (screen, world) = state.at(&event);
                let button = button_of(&event);
                let start = state.press.remove(&event.id.0);
                (state.sink)(PickEvent::Release {
                    world,
                    screen,
                    button,
                });
                if is_contact(&event) {
                    (state.sink)(PickEvent::TouchUp { id: event.id.0 });
                }
                if start.is_some_and(|start| click_within_slop_dp(start, screen, density())) {
                    (state.sink)(PickEvent::Click {
                        world,
                        screen,
                        button,
                    });
                }
            })
            .on_pointer_move(move |event: PointerEvent| {
                let state = moved.borrow();
                let (screen, world) = state.at(&event);
                (state.sink)(PickEvent::Hover { world, screen });
                if is_contact(&event) {
                    (state.sink)(PickEvent::TouchMove {
                        id: event.id.0,
                        screen,
                    });
                }
            })
            .on_pointer_leave(move |event: PointerEvent| {
                let mut state = leave.borrow_mut();
                if state.press.remove(&event.id.0).is_some() {
                    (state.sink)(PickEvent::TouchUp { id: event.id.0 });
                }
            }),
    );

    // The later children stack on top: the transparent hit surface sits above
    // the sprite and HUD layers it reports picks for.
    repose_ui::ZStack(Modifier::new().fill_max_size())
        .child(surface)
        .child(picks)
}
