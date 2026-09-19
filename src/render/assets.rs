use bevy_ecs::prelude::*;
use glam::Vec2;
use repame_atlas::{Atlas, AtlasDesc, UvRect, atlas_id};
use repame_sprite::{AtlasUpload, BatchDesc, Camera2d, FrameInput, SpriteInstance, WorldText};
use std::collections::HashMap;
use std::path::Path;

pub const ATLAS_SIZE: u32 = 2048;
pub const ATLAS_PAGES: u32 = 4;

pub struct RenderAssets {
    atlas: Atlas,
    defs: HashMap<String, UvRect>,
    white: UvRect,
    uploads: Vec<AtlasUpload>,
    bg: Option<(UvRect, u32, u32)>,
    bg_uploads: Vec<AtlasUpload>,
    tileset: Option<(UvRect, u32, u32)>,
    tileset_uploads: Vec<AtlasUpload>,
    tile_px: u32,
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
        self.bg.map(|(uv, _, _)| uv)
    }

    pub fn bg_size(&self) -> Option<[f32; 2]> {
        self.bg.map(|(_, w, h)| [w as f32, h as f32])
    }

    pub fn tileset_uv(&self) -> Option<UvRect> {
        self.tileset.map(|(uv, _, _)| uv)
    }

    pub fn tile_px(&self) -> u32 {
        self.tile_px
    }

    pub fn atlas_page_size(&self) -> u32 {
        ATLAS_SIZE
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

pub fn load_assets(assets_dir: &Path) -> RenderAssets {
    let mut atlas = Atlas::new(AtlasDesc {
        size: ATLAS_SIZE,
        max_pages: ATLAS_PAGES,
        padding: 1,
    });
    let mut defs = HashMap::new();
    let mut uploads = Vec::new();
    for name in ["body", "head", "upper_arm", "hand", "leg", "bow", "arrow"] {
        let path = assets_dir.join("images").join(format!("{name}.svg"));
        let _ = &path;
        let placeholder = vec![200u8, 200, 200, 255].repeat(64 * 64);
        blit_owned(
            &mut atlas,
            &mut uploads,
            &mut defs,
            name,
            64,
            64,
            placeholder,
        );
    }
    let white = atlas.ensure_white().expect("atlas fits white texel");
    for write in atlas.drain_writes() {
        uploads.push(AtlasUpload::white(write.page, write.x, write.y));
    }
    RenderAssets {
        atlas,
        defs,
        white,
        uploads,
        bg: None,
        bg_uploads: Vec::new(),
        tileset: None,
        tileset_uploads: Vec::new(),
        tile_px: 32,
    }
}

pub fn bake_svg_assets(assets_dir: &Path) -> RenderAssets {
    let images_dir = assets_dir.join("images");
    let mut atlas = Atlas::new(AtlasDesc {
        size: ATLAS_SIZE,
        max_pages: ATLAS_PAGES,
        padding: 1,
    });
    let mut defs = HashMap::new();
    let mut uploads = Vec::new();
    for name in ["body", "head", "upper_arm", "hand", "leg", "bow", "arrow"] {
        let svg_path = images_dir.join(format!("{name}.svg"));
        let (w, h, rgba) = rasterize_svg(&svg_path).unwrap_or_else(|| {
            let px = if name == "arrow" {
                arrow_placeholder()
            } else {
                vec![200u8, 200, 200, 255].repeat(64 * 64)
            };
            let side = if name == "arrow" { 32 } else { 64 };
            (side, side, px)
        });
        blit_owned(&mut atlas, &mut uploads, &mut defs, name, w, h, rgba);
    }
    let (bg, bg_uploads, bg_size) = rasterize_artboard(&images_dir.join("bg.svg"), &mut atlas);
    let (tileset, tileset_uploads, tile_px) =
        rasterize_tileset(&images_dir.join("tileset.svg"), &mut atlas);
    let _ = bg_size;
    let white = atlas.ensure_white().expect("atlas fits white texel");
    for write in atlas.drain_writes() {
        uploads.push(AtlasUpload::white(write.page, write.x, write.y));
    }
    RenderAssets {
        atlas,
        defs,
        white,
        uploads,
        bg: bg.map(|(uv, w, h)| (uv, w, h)),
        bg_uploads,
        tileset: tileset.map(|(uv, w, h)| (uv, w, h)),
        tileset_uploads,
        tile_px,
    }
}

fn arrow_placeholder() -> Vec<u8> {
    let (w, h) = (32usize, 8usize);
    let mut px = vec![0u8; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) * 4;
            let shaft = y >= 3 && y <= 4;
            let head = x >= 26 && y >= 1 && y <= 6 && x - 26 <= y.min(7 - y) + 2;
            if shaft || head || x < 3 {
                px[i] = 139;
                px[i + 1] = 90;
                px[i + 2] = 43;
                px[i + 3] = 255;
            }
        }
    }
    px
}

fn rasterize_svg(path: &Path) -> Option<(u32, u32, Vec<u8>)> {
    let bytes = std::fs::read(path).ok()?;
    let mut report = renamite_io_svg::import_with_report(&bytes).ok()?.value;
    grayscale_part(&mut report);
    render_document(&report, true)
}

fn grayscale_part(doc: &mut renamite_model::Document) {
    use renamite_model::{NodeKind, StylePaint};
    for node in doc.nodes.values_mut() {
        if let NodeKind::Style(style) = &mut node.kind {
            let lum = |c: renamite_model::Color| {
                let l = 0.299 * c.r + 0.587 * c.g + 0.114 * c.b;
                renamite_model::Color {
                    r: l,
                    g: l,
                    b: l,
                    a: c.a,
                }
            };
            let gray = match style.paint() {
                StylePaint::Solid { .. } => StylePaint::solid(lum(style.paint().base_color())),
                StylePaint::Gradient(g) => {
                    let mut out = g.clone();
                    for stop in out.stops.base.0.iter_mut() {
                        stop.color = lum(stop.color);
                    }
                    StylePaint::Gradient(out)
                }
            };
            let _ = style.swap_paint(gray);
        }
    }
}

fn rasterize_artboard(
    path: &Path,
    atlas: &mut Atlas,
) -> (Option<(UvRect, u32, u32)>, Vec<AtlasUpload>, [u32; 2]) {
    let Some(bytes) = std::fs::read(path).ok() else {
        return (None, Vec::new(), [0, 0]);
    };
    let Ok(report) = renamite_io_svg::import_with_report(&bytes).map(|r| r.value) else {
        return (None, Vec::new(), [0, 0]);
    };
    let Some((w, h, rgba)) = render_document(&report, false) else {
        return (None, Vec::new(), [0, 0]);
    };
    let key = atlas_id(&format!("artboard:{}", path.display()));
    let Ok(uv) = atlas.alloc(key, w, h) else {
        return (None, Vec::new(), [w, h]);
    };
    let mut uploads = Vec::new();
    for write in atlas.drain_writes() {
        uploads.push(AtlasUpload::from_write(&write, rgba.clone()));
    }
    (Some((uv, w, h)), uploads, [w, h])
}

fn rasterize_tileset(
    path: &Path,
    atlas: &mut Atlas,
) -> (Option<(UvRect, u32, u32)>, Vec<AtlasUpload>, u32) {
    let (uv, uploads, size) = rasterize_artboard(path, atlas);
    let tile_px = if size[0] > 0 {
        (size[0] / crate::sim::arena::TILESET_COLS).max(1)
    } else {
        32
    };
    (uv, uploads, tile_px)
}

fn render_document(doc: &renamite_model::Document, trim: bool) -> Option<(u32, u32, Vec<u8>)> {
    let (aw, ah) = doc.compositions[doc.main].size;
    if aw == 0 || ah == 0 {
        return None;
    }
    let canvas = 1024u32;
    let mut gpu =
        renamite_render_offscreen::OffscreenRenderer::new_blocking(canvas, canvas, 1).ok()?;
    let mut bridge = renamite_render_bridge::SceneRenderer::new();
    let view = renamite_render_offscreen::fit_view((aw, ah), canvas, canvas);
    let project = renamite_io_ren::RenFile::new(doc.clone(), "part");
    let mut player = renamite_player::Player::new(project).ok()?;
    player.engine.scrub(&player.project, 0.0);
    gpu.sync_document_images(&player.project.document).ok()?;
    let prepared = bridge.prepare(player.engine.scene(), &view);
    let mut scene = repose_core::Scene::default();
    bridge.append_repose_scene(&prepared, &mut scene);
    let rgba = gpu.render_rgba(&scene, Some([0.0, 0.0, 0.0, 0.0])).ok()?;
    if trim {
        trim_rgba(&rgba, canvas, canvas)
    } else {
        let ox = view.offset.x.round().max(0.0) as u32;
        let oy = view.offset.y.round().max(0.0) as u32;
        let pw = ((aw as f64) * view.scale).round().max(1.0) as u32;
        let ph = ((ah as f64) * view.scale).round().max(1.0) as u32;
        let pw = pw.min(canvas.saturating_sub(ox));
        let ph = ph.min(canvas.saturating_sub(oy));
        crop_rgba(&rgba, canvas, canvas, ox, oy, pw, ph)
    }
}

fn trim_rgba(rgba: &[u8], w: u32, h: u32) -> Option<(u32, u32, Vec<u8>)> {
    let (w, h) = (w as usize, h as usize);
    let (mut min_x, mut min_y) = (w, h);
    let (mut max_x, mut max_y) = (0usize, 0usize);
    for y in 0..h {
        for x in 0..w {
            if rgba[(y * w + x) * 4 + 3] != 0 {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }
    if max_x < min_x || max_y < min_y {
        return None;
    }
    let (cw, ch) = (max_x - min_x + 1, max_y - min_y + 1);
    let mut data = Vec::with_capacity(cw * ch * 4);
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let i = (y * w + x) * 4;
            data.extend_from_slice(&rgba[i..i + 4]);
        }
    }
    Some((cw as u32, ch as u32, data))
}

fn crop_rgba(
    rgba: &[u8],
    w: u32,
    h: u32,
    ox: u32,
    oy: u32,
    pw: u32,
    ph: u32,
) -> Option<(u32, u32, Vec<u8>)> {
    let w = w as usize;
    let h = h as usize;
    if (ox as usize + pw as usize) > w || (oy as usize + ph as usize) > h {
        return None;
    }
    let mut data = Vec::with_capacity((pw * ph * 4) as usize);
    for y in 0..ph as usize {
        for x in 0..pw as usize {
            let i = ((oy as usize + y) * w + (ox as usize + x)) * 4;
            data.extend_from_slice(&rgba[i..i + 4]);
        }
    }
    Some((pw, ph, data))
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

pub fn world_instances(world: &mut World, assets: &RenderAssets) -> Vec<SpriteInstance> {
    use crate::sim::comps::{ArchetypeTint, Arrow, WarriorLimb};
    let mut out = Vec::new();
    if let Some(bg_uv) = assets.bg_uv() {
        let bg_size = assets.bg_size().unwrap_or([2400.0, 1440.0]);
        out.push(SpriteInstance {
            center: Vec2::new(0.0, 40.0),
            rotation: 0.0,
            size: Vec2::new(bg_size[0], bg_size[1]),
            color: [0.28, 0.28, 0.28, 1.0],
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
    tiles_instances(world, assets, &mut out);
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
    if world.get::<PlayerTag>(player).is_none() {
        return None;
    }
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

fn tiles_instances(_world: &mut World, assets: &RenderAssets, out: &mut Vec<SpriteInstance>) {
    use crate::sim::arena::{
        GROUND_HALF_WIDTH, GROUND_TOP, TILE_FILL_DARK_ROW, TILE_FILL_ROWS, TILE_SURFACE_ROW,
        TILE_WORLD,
    };
    if assets.tileset_uv().is_none() {
        return;
    }
    let tw = TILE_WORLD;
    let width = GROUND_HALF_WIDTH * 2.0;
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
            GROUND_TOP - tw * 0.5,
            size,
            TILE_SURFACE_ROW,
            -1.0,
        );
        for r in 1..=4u32 {
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
                GROUND_TOP - tw * 0.5 - r as f32 * tw,
                size,
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
    row: u32,
    z: f32,
) {
    use crate::sim::arena::{TILESET_COLS, TILESET_ROWS};
    let Some(tileset) = assets.tileset_uv() else {
        return;
    };
    let tile_px = assets.tile_px().max(1);
    let span_x = tileset.max[0] - tileset.min[0];
    let span_y = tileset.max[1] - tileset.min[1];
    let cell_w = span_x / TILESET_COLS as f32;
    let cell_h = span_y / TILESET_ROWS as f32;
    let strip_cells = 4.0;
    let (uv_min, uv_max) = (
        Vec2::new(tileset.min[0], tileset.min[1] + row as f32 * cell_h),
        Vec2::new(
            tileset.min[0] + strip_cells * cell_w,
            tileset.min[1] + (row as f32 + 1.0) * cell_h,
        ),
    );
    let _ = tile_px;
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
    assets: &RenderAssets,
    cam: Camera2d,
    world_size: [f32; 2],
    viewport_dp: [f32; 2],
) -> FrameInput {
    let sprites = world_instances(world, assets);
    let texts: Vec<WorldText> = world
        .query::<&repame_fx::DamageNumber>()
        .iter(world)
        .map(|n| WorldText {
            text: n.text.clone(),
            pos: Vec2::new(n.x, n.y),
            color: n.color,
            size: 18.0,
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
        chroma: 0.0,
    }
}
