use bevy_ecs::prelude::*;
use glam::Vec2;
use repame_atlas::{Atlas, AtlasDesc, UvRect, atlas_id};
use repame_view2d::{AtlasUpload, BatchDesc, Camera2d, SpriteInstance, WorldText};
use std::collections::HashMap;

use crate::view2d::FrameInput;

pub const ATLAS_SIZE: u32 = 2048;
pub const ATLAS_PAGES: u32 = 4;

pub struct RenderAssets {
    atlas: Atlas,
    defs: HashMap<String, UvRect>,
    white: UvRect,
    uploads: Vec<AtlasUpload>,
    bg: Option<UvRect>,
    bg_uploads: Vec<AtlasUpload>,
    tileset: Option<UvRect>,
    tileset_uploads: Vec<AtlasUpload>,
}

impl RenderAssets {
    pub fn batch_desc(&self) -> BatchDesc {
        BatchDesc {
            layer_size: ATLAS_SIZE,
            layers: self.atlas.page_count().max(1),
            ..Default::default()
        }
    }

    pub fn take_uploads(&mut self) -> Vec<AtlasUpload> {
        let mut out = std::mem::take(&mut self.uploads);
        out.extend(std::mem::take(&mut self.bg_uploads));
        out.extend(std::mem::take(&mut self.tileset_uploads));
        out
    }

    pub fn uv(&self, name: &str) -> Option<UvRect> {
        self.defs.get(name).copied()
    }

    pub fn white_uv(&self) -> Option<UvRect> {
        Some(self.white)
    }

    pub fn bg_uv(&self) -> Option<UvRect> {
        self.bg
    }

    pub fn tileset_uv(&self) -> Option<UvRect> {
        self.tileset
    }
}

fn blit_owned(
    atlas: &mut Atlas,
    uploads: &mut Vec<AtlasUpload>,
    defs: &mut HashMap<String, UvRect>,
    name: &str,
    w: u32,
    h: u32,
    rgba: Vec<u8>,
) -> Option<UvRect> {
    let key = atlas_id(name);
    let uv = atlas.alloc(key, w, h).ok()?;
    for write in atlas.drain_writes() {
        uploads.push(AtlasUpload::from_write(&write, rgba.clone()));
    }
    defs.insert(name.to_string(), uv);
    Some(uv)
}

const PART_ASSETS: [(&str, &[u8]); 7] = [
    ("body", include_bytes!("../../assets/generated/body.png")),
    ("head", include_bytes!("../../assets/generated/head.png")),
    (
        "upper_arm",
        include_bytes!("../../assets/generated/upper_arm.png"),
    ),
    ("hand", include_bytes!("../../assets/generated/hand.png")),
    ("leg", include_bytes!("../../assets/generated/leg.png")),
    ("bow", include_bytes!("../../assets/generated/bow.png")),
    ("arrow", include_bytes!("../../assets/generated/arrow.png")),
];
const BG_PNG: &[u8] = include_bytes!("../../assets/generated/bg.png");
const TILESET_PNG: &[u8] = include_bytes!("../../assets/generated/tileset.png");

pub struct AssetBake {
    atlas: Atlas,
    defs: HashMap<String, UvRect>,
    uploads: Vec<AtlasUpload>,
    part: usize,
    bg: Option<UvRect>,
    bg_uploads: Vec<AtlasUpload>,
    tileset: Option<UvRect>,
    tileset_uploads: Vec<AtlasUpload>,
    white: Option<UvRect>,
}

impl AssetBake {
    pub fn new() -> Self {
        Self {
            atlas: Atlas::new(AtlasDesc {
                size: ATLAS_SIZE,
                max_pages: ATLAS_PAGES,
                padding: 1,
            }),
            defs: HashMap::new(),
            uploads: Vec::new(),
            part: 0,
            bg: None,
            bg_uploads: Vec::new(),
            tileset: None,
            tileset_uploads: Vec::new(),
            white: None,
        }
    }

    pub fn total_steps() -> usize {
        PART_ASSETS.len() + 3
    }

    pub fn steps_done(&self) -> usize {
        self.part.min(PART_ASSETS.len())
            + usize::from(self.bg.is_some())
            + usize::from(self.tileset.is_some())
            + usize::from(self.white.is_some())
    }

    pub fn progress(&self) -> f32 {
        (self.steps_done() as f32 / Self::total_steps() as f32).clamp(0.0, 1.0)
    }

    /// Bakes one asset so the frame stays responsive. Returns true when finished.
    pub fn advance(&mut self) -> bool {
        if let Some((name, bytes)) = PART_ASSETS.get(self.part).copied() {
            let (width, height, rgba) = decode_png(name, bytes);
            assert!(
                blit_owned(
                    &mut self.atlas,
                    &mut self.uploads,
                    &mut self.defs,
                    name,
                    width,
                    height,
                    rgba,
                )
                .is_some(),
                "embedded asset {name} does not fit the sprite atlas"
            );
            self.part += 1;
            return false;
        }
        if self.bg.is_none() {
            let (bg, bg_uploads, _) = blit_artboard("bg", BG_PNG, &mut self.atlas);
            self.bg = bg.map(|(uv, _, _)| uv);
            self.bg_uploads = bg_uploads;
            return false;
        }
        if self.tileset.is_none() {
            let (tileset, tileset_uploads, tile_size) =
                blit_artboard("tileset", TILESET_PNG, &mut self.atlas);
            assert_eq!(tile_size[0] % crate::sim::arena::TILESET_COLS, 0);
            assert_eq!(tile_size[1] % crate::sim::arena::TILESET_ROWS, 0);
            assert!(tile_size[0] / crate::sim::arena::TILESET_COLS > 0);
            self.tileset = tileset.map(|(uv, _, _)| uv);
            self.tileset_uploads = tileset_uploads;
            return false;
        }
        self.white = Some(self.atlas.ensure_white().expect("atlas fits white texel"));
        for write in self.atlas.drain_writes() {
            self.uploads
                .push(AtlasUpload::white(write.page, write.x, write.y));
        }
        true
    }

    pub fn finish(self) -> RenderAssets {
        RenderAssets {
            atlas: self.atlas,
            defs: self.defs,
            white: self.white.expect("bake finished"),
            uploads: self.uploads,
            bg: self.bg,
            bg_uploads: self.bg_uploads,
            tileset: self.tileset,
            tileset_uploads: self.tileset_uploads,
        }
    }
}

fn decode_png(name: &str, bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let image =
        image::load_from_memory(bytes).unwrap_or_else(|error| panic!("decode {name}: {error}"));
    let rgba = image.into_rgba8();
    let (width, height) = rgba.dimensions();
    (width, height, rgba.into_raw())
}

fn blit_artboard(
    name: &str,
    bytes: &[u8],
    atlas: &mut Atlas,
) -> (Option<(UvRect, u32, u32)>, Vec<AtlasUpload>, [u32; 2]) {
    let (width, height, rgba) = decode_png(name, bytes);
    let key = atlas_id(&format!("artboard:{name}"));
    let Ok(uv) = atlas.alloc(key, width, height) else {
        return (None, Vec::new(), [width, height]);
    };
    let mut uploads = Vec::new();
    for write in atlas.drain_writes() {
        uploads.push(AtlasUpload::from_write(&write, rgba.clone()));
    }
    (Some((uv, width, height)), uploads, [width, height])
}

pub fn limb_sprite_name(kind: crate::sim::comps::LimbKind) -> &'static str {
    match kind {
        crate::sim::comps::LimbKind::Torso => "body",
        crate::sim::comps::LimbKind::Head => "head",
        crate::sim::comps::LimbKind::UpperArmL | crate::sim::comps::LimbKind::UpperArmR => {
            "upper_arm"
        }
        crate::sim::comps::LimbKind::ArmL | crate::sim::comps::LimbKind::ArmR => "hand",
        crate::sim::comps::LimbKind::LegL | crate::sim::comps::LimbKind::LegR => "leg",
    }
}

pub fn world_instances(
    world: &mut World,
    assets: &RenderAssets,
    camera: Camera2d,
    visible_size: Vec2,
) -> Vec<SpriteInstance> {
    use crate::sim::comps::{ArchetypeTint, Arrow, WarriorLimb};
    let mut out = Vec::new();
    if let Some(bg_uv) = assets.bg_uv() {
        let bg_size = Vec2::new(crate::sim::arena::BG_SIZE[0], crate::sim::arena::BG_SIZE[1]);
        let size = Vec2::new(
            bg_size.x.max(visible_size.x * 1.05),
            bg_size.y.max(visible_size.y * 1.05),
        );
        out.push(SpriteInstance {
            center: Vec2::new(
                camera.center.x * 0.75,
                -360.0 + (camera.center.y + 360.0) * 0.75,
            ),
            rotation: 0.0,
            size,
            color: [0.47, 0.47, 0.47, 1.0],
            uv_min: Vec2::new(bg_uv.min[0], bg_uv.min[1]),
            uv_max: Vec2::new(bg_uv.max[0], bg_uv.max[1]),
            page: bg_uv.page,
            z: -10.0,
            ..Default::default()
        });
    }
    let limb_rows: Vec<(crate::sim::comps::LimbKind, Vec2, f32, Vec2, [f32; 4], f32)> = world
        .query::<(&WarriorLimb, &repame_rapier2d::BodySnapshot2d)>()
        .iter(world)
        .map(|(limb, snap)| {
            let archetype = world
                .get::<ArchetypeTint>(limb.root)
                .map(|t| t.tint)
                .unwrap_or([1.0, 1.0, 1.0, 1.0]);
            let tint = crate::sim::comps::limb_tint(limb.kind, archetype);
            let fade = world
                .get::<crate::sim::state::DyingFade>(limb.root)
                .map(|f| (f.0 / 2.0).clamp(0.0, 1.0))
                .unwrap_or(1.0);
            let name = limb_sprite_name(limb.kind);
            let size = assets.uv(name).map(|_| limb.size).unwrap_or(limb.size);
            (limb.kind, snap.pos, snap.angle, size, tint, fade)
        })
        .collect();
    for (kind, pos, angle, size, mut tint, fade) in limb_rows {
        tint[3] *= fade;
        let name = limb_sprite_name(kind);
        if let Some(uv) = assets.uv(name) {
            out.push(SpriteInstance {
                center: pos,
                rotation: angle,
                size,
                color: tint,
                uv_min: Vec2::new(uv.min[0], uv.min[1]),
                uv_max: Vec2::new(uv.max[0], uv.max[1]),
                page: uv.page,
                z: 1.0,
                ..Default::default()
            });
        } else {
            out.push(SpriteInstance {
                center: pos,
                rotation: angle,
                size,
                color: tint,
                z: 1.0,
                ..Default::default()
            });
        }
    }
    tiles_instances(world, assets, camera, visible_size, &mut out);
    health_bar_instances(world, assets, &mut out);
    if let Some(white) = assets.white_uv() {
        for s in repame_fx::particle_sprites(world.query::<&repame_fx::Particle>().iter(world)) {
            let mut s = s;
            if s.uv_min == Vec2::ZERO && s.uv_max == Vec2::ONE {
                s.uv_min = Vec2::new(white.min[0], white.min[1]);
                s.uv_max = Vec2::new(white.max[0], white.max[1]);
                s.page = white.page;
            }
            s.z = 3.0;
            out.push(s);
        }
    }
    let arrows: Vec<(Vec2, f32)> = world
        .query::<&Arrow>()
        .iter(world)
        .map(|a| (a.pos, a.angle))
        .collect();
    for (pos, angle) in arrows {
        if let Some(uv) = assets.uv("arrow") {
            out.push(SpriteInstance {
                center: pos,
                rotation: angle,
                size: Vec2::new(30.0, 4.0),
                color: [1.0, 1.0, 1.0, 1.0],
                uv_min: Vec2::new(uv.min[0], uv.min[1]),
                uv_max: Vec2::new(uv.max[0], uv.max[1]),
                page: uv.page,
                z: 2.0,
                ..Default::default()
            });
        } else if let Some(white) = assets.white_uv() {
            out.push(SpriteInstance {
                center: pos,
                rotation: angle,
                size: Vec2::new(30.0, 4.0),
                color: [0.5, 0.35, 0.2, 1.0],
                uv_min: Vec2::new(white.min[0], white.min[1]),
                uv_max: Vec2::new(white.max[0], white.max[1]),
                page: white.page,
                z: 2.0,
                ..Default::default()
            });
        }
    }
    if let Some(bow) = bow_instance(world, assets) {
        out.push(bow);
    }
    out
}

fn bow_instance(world: &mut World, assets: &RenderAssets) -> Option<SpriteInstance> {
    use crate::sim::comps::{BowAim, BowState, BowVisual, PlayerTag, WarriorRoot};
    use crate::sim::rounds::{RunPhase, RunState};
    if world.resource::<RunState>().phase != RunPhase::Combat {
        return None;
    }
    let player = world.resource::<RunState>().player?;
    let warrior = world.get::<WarriorRoot>(player)?;
    if warrior.is_dead {
        return None;
    }
    world.get::<PlayerTag>(player)?;
    let aim_angle = world.get::<BowAim>(player).map(|a| a.angle).unwrap_or(0.0);
    let torso = world.get::<repame_rapier2d::BodySnapshot2d>(warrior.torso)?;
    let vis = world.get::<BowVisual>(player)?;
    let uv = assets.uv("bow")?;
    let world_angle = torso.angle + aim_angle;
    let dir = Vec2::from_angle(world_angle);
    let bow = world.get::<BowState>(player)?;
    let raw = if bow.drawing {
        (bow.draw_power / 100.0).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let t = raw * raw * (3.0 - 2.0 * raw);
    let size = Vec2::new(
        vis.base_size.x * (1.0 + 0.12 * t),
        vis.base_size.y * (1.0 + 0.55 * t),
    );
    let heat = t;
    let center = torso.pos + dir * (vis.rest_x - vis.pull_distance * t);
    Some(SpriteInstance {
        center,
        rotation: world_angle,
        size,
        color: [1.0, 1.0 - 0.18 * heat, 1.0 - 0.42 * heat, 1.0],
        uv_min: Vec2::new(uv.min[0], uv.min[1]),
        uv_max: Vec2::new(uv.max[0], uv.max[1]),
        page: uv.page,
        z: 1.5,
        ..Default::default()
    })
}

fn tiles_instances(
    _world: &mut World,
    assets: &RenderAssets,
    camera: Camera2d,
    visible_size: Vec2,
    out: &mut Vec<SpriteInstance>,
) {
    use crate::sim::arena::{
        GROUND_HALF_WIDTH, GROUND_TOP, TILE_COLUMN, TILE_FILL_DARK_ROW, TILE_FILL_ROWS,
        TILE_SURFACE_ROW, TILE_WORLD,
    };
    if assets.tileset_uv().is_none() {
        return;
    }
    let tw = TILE_WORLD;
    let width = GROUND_HALF_WIDTH * 2.0;
    let view_bottom = camera.center.y + visible_size.y * 0.5;
    let rows = (((view_bottom - GROUND_TOP) / tw).ceil() as u32).clamp(4, 14);
    let cols = (width / tw).ceil() as i32;
    let start_x = -width * 0.5 + tw * 0.5;
    let mut c = 0i32;
    while c < cols {
        let n = (cols - c).min(4);
        let x = start_x + (c as f32 + (n as f32 - 1.0) * 0.5) * tw;
        let size = Vec2::new(n as f32 * tw, tw);
        push_tile_strip(
            out,
            assets,
            x,
            GROUND_TOP + tw * 0.5,
            size,
            n,
            TILE_COLUMN,
            TILE_SURFACE_ROW,
            -1.0,
        );
        for r in 1..=rows {
            let row = if r == 4 {
                TILE_FILL_DARK_ROW
            } else if r % 2 == 0 {
                TILE_FILL_ROWS[1]
            } else {
                TILE_FILL_ROWS[0]
            };
            push_tile_strip(
                out,
                assets,
                x,
                GROUND_TOP + tw * 0.5 + r as f32 * tw,
                size,
                n,
                TILE_COLUMN,
                row,
                -1.1,
            );
        }
        c += n;
    }
}

fn push_tile_strip(
    out: &mut Vec<SpriteInstance>,
    assets: &RenderAssets,
    x: f32,
    y: f32,
    size: Vec2,
    cells: i32,
    col: u32,
    row: u32,
    z: f32,
) {
    use crate::sim::arena::{TILESET_COLS, TILESET_ROWS};
    let Some(tileset) = assets.tileset_uv() else {
        return;
    };
    let span_x = tileset.max[0] - tileset.min[0];
    let span_y = tileset.max[1] - tileset.min[1];
    let cell_w = span_x / TILESET_COLS as f32;
    let cell_h = span_y / TILESET_ROWS as f32;
    let (uv_min, uv_max) = (
        Vec2::new(
            tileset.min[0] + col as f32 * cell_w,
            tileset.min[1] + row as f32 * cell_h,
        ),
        Vec2::new(
            tileset.min[0] + (col as f32 + cells as f32) * cell_w,
            tileset.min[1] + (row as f32 + 1.0) * cell_h,
        ),
    );
    out.push(SpriteInstance {
        center: Vec2::new(x, y),
        rotation: 0.0,
        size,
        color: [0.5, 0.5, 0.5, 1.0],
        uv_min,
        uv_max,
        page: tileset.page,
        z,
        ..Default::default()
    });
}

fn health_bar_instances(world: &mut World, assets: &RenderAssets, out: &mut Vec<SpriteInstance>) {
    use crate::sim::comps::{WarriorRoot, WorldHealthBar};
    let Some(white) = assets.white_uv() else {
        return;
    };
    let white_min = Vec2::new(white.min[0], white.min[1]);
    let white_max = Vec2::new(white.max[0], white.max[1]);
    let bars: Vec<(Vec2, i32, i32, f32, f32, bool)> = world
        .query::<&WorldHealthBar>()
        .iter(world)
        .filter_map(|bar| {
            let warrior = world.get::<WarriorRoot>(bar.root)?;
            let torso = world.get::<repame_rapier2d::BodySnapshot2d>(warrior.torso)?;
            Some((
                torso.pos + Vec2::new(0.0, bar.y_offset),
                warrior.health,
                warrior.max_health,
                bar.width,
                0.0,
                warrior.is_dead,
            ))
        })
        .collect();
    for (center, health, max_health, width, _, dead) in bars {
        if dead {
            continue;
        }
        let pct = (health.max(0) as f32 / max_health.max(1) as f32).clamp(0.0, 1.0);
        out.push(SpriteInstance {
            center,
            rotation: 0.0,
            size: Vec2::new(width + 2.0, 8.0),
            color: [0.12, 0.12, 0.14, 0.95],
            uv_min: white_min,
            uv_max: white_max,
            page: white.page,
            z: 5.0,
            ..Default::default()
        });
        let fill_width = (width * pct).max(0.001);
        let fill_color = if pct > 0.6 {
            [0.28, 0.85, 0.4, 1.0]
        } else if pct > 0.3 {
            [0.95, 0.78, 0.2, 1.0]
        } else {
            [0.92, 0.28, 0.25, 1.0]
        };
        out.push(SpriteInstance {
            center: center + Vec2::new(-0.5 * (width - fill_width), 0.0),
            rotation: 0.0,
            size: Vec2::new(fill_width, 6.0),
            color: fill_color,
            uv_min: white_min,
            uv_max: white_max,
            page: white.page,
            z: 5.1,
            ..Default::default()
        });
    }
}

pub fn frame_input(
    world: &mut World,
    assets: Option<&RenderAssets>,
    cam: Camera2d,
    world_size: [f32; 2],
    viewport_dp: [f32; 2],
) -> FrameInput {
    let visible_size = Vec2::new(world_size[0], world_size[1]);
    let sprites = match assets {
        Some(assets) => world_instances(world, assets, cam, visible_size),
        None => Vec::new(),
    };
    let texts: Vec<WorldText> = world
        .query::<&repame_fx::DamageNumber>()
        .iter(world)
        .map(|n| WorldText {
            text: n.text.clone(),
            pos: Vec2::new(n.x, n.y),
            color: n.color,
            size: 18.0,
            // Damage numbers were drawn with the engine's default stack before
            // the field existed; `None` is that same stack.
            font_family: None,
        })
        .collect();
    let overlay_color = world
        .get_resource::<repame_fx::Flash>()
        .and_then(|f| f.rgba());
    FrameInput {
        cam,
        world_size,
        viewport_dp,
        sprites,
        texts,
        background: Some([0.03, 0.03, 0.05, 1.0]),
        overlay_color,
    }
}
