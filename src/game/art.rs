use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use renamite_behavior_common::ViewTransform;
use renamite_model::{Color, Document, NodeKind, StylePaint};
use renamite_render_bridge::SceneRenderer;
use renamite_render_offscreen::OffscreenRenderer;

/// Body-part textures rasterized from the Inkscape SVGs in `assets/images/`.
/// Sprites reference these handles, then set `custom_size` for scale and
/// `color` for team/archetype tinting.
#[derive(Resource, Clone)]
pub struct WarriorArt {
    /// Torso with tunic + shoulder trim.
    pub body: Handle<Image>,
    /// Head with face.
    pub head: Handle<Image>,
    /// Short upper-arm segment.
    pub upper_arm: Handle<Image>,
    /// Forearm + fist (attached to the upper arm).
    pub hand: Handle<Image>,
    /// Shank.
    pub leg: Handle<Image>,
    /// Bow held on the bow pivot (drawn white, never tinted).
    pub bow: Handle<Image>,
    /// Landscape title backdrop.
    pub bg: Handle<Image>,
    /// Full tileset atlas (no alpha-trim). 8 columns × 4 rows of cells.
    pub tileset: Handle<Image>,
    /// Pixel size of one cell in `tileset` (width / 8).
    pub tile_px: u32,
}

/// One-shot startup rasterizer: SVG -> Document (renamite) -> Scene
/// (renamite player evaluation) -> repose scene -> offscreen RGBA, all
/// through the renamite/repose crates on a single wgpu device.
struct SvgBaker {
    gpu: OffscreenRenderer,
    bridge: SceneRenderer,
    /// Square offscreen target; the view transform maps each artboard into it.
    canvas: u32,
}

const CANVAS: u32 = 2048;

impl SvgBaker {
    fn new() -> anyhow::Result<Self> {
        Ok(Self {
            gpu: OffscreenRenderer::new_blocking(CANVAS, CANVAS, 4)?,
            bridge: SceneRenderer::new(),
            canvas: CANVAS,
        })
    }

    fn bake(&mut self, svg_path: &str, tintable: bool) -> anyhow::Result<Image> {
        let mut doc = renamite_io_svg::import(&std::fs::read(svg_path)?)?;
        if tintable {
            grayscale_paints(&mut doc);
        }
        let (aw, ah) = doc.compositions[doc.main].size;
        let scale = self.canvas as f64 / (aw.max(ah) as f64);
        let project = renamite_io_ren::RenFile::new(doc, "part");
        let mut player = renamite_player::Player::new(project)?;
        player.engine.scrub(&player.project, 0.0);
        self.gpu.sync_document_images(&player.project.document)?;
        let view = ViewTransform {
            scale,
            offset: bevy::math::DVec2::ZERO,
        };
        let prepared = self.bridge.prepare(player.engine.scene(), &view);
        let mut repose_scene = repose_core::Scene::default();
        self.bridge
            .append_repose_scene(&prepared, &mut repose_scene);
        let rgba = self.gpu.render_rgba(&repose_scene, None)?;
        trim_to_image(&rgba, self.canvas, self.canvas)
    }

    /// Rasterize the full artboard; keep empty cells so atlas UVs stay stable.
    fn bake_atlas(&mut self, svg_path: &str) -> anyhow::Result<(Image, u32, u32)> {
        let doc = renamite_io_svg::import(&std::fs::read(svg_path)?)?;
        let (aw, ah) = doc.compositions[doc.main].size;
        let scale = self.canvas as f64 / (aw.max(ah) as f64);
        let project = renamite_io_ren::RenFile::new(doc, "atlas");
        let mut player = renamite_player::Player::new(project)?;
        player.engine.scrub(&player.project, 0.0);
        self.gpu.sync_document_images(&player.project.document)?;
        let view = ViewTransform {
            scale,
            offset: bevy::math::DVec2::ZERO,
        };
        let prepared = self.bridge.prepare(player.engine.scene(), &view);
        let mut repose_scene = repose_core::Scene::default();
        self.bridge
            .append_repose_scene(&prepared, &mut repose_scene);
        let rgba = self.gpu.render_rgba(&repose_scene, None)?;

        // Artboard pixel rect inside the square canvas (top-left anchored by
        // the view transform: world (0,0) -> screen (0,0)).
        let pw = (aw as f64 * scale).round().max(1.0) as u32;
        let ph = (ah as f64 * scale).round().max(1.0) as u32;
        let pw = pw.min(self.canvas);
        let ph = ph.min(self.canvas);

        let mut data = Vec::with_capacity((pw * ph * 4) as usize);
        let w = self.canvas as usize;
        for y in 0..ph as usize {
            for x in 0..pw as usize {
                let i = (y * w + x) * 4;
                let a = rgba[i + 3] as f32 / 255.0;
                if a > 0.0 {
                    let cv = |v: u8| ((v as f32 / 255.0 / a).min(1.0) * 255.0) as u8;
                    data.extend([cv(rgba[i]), cv(rgba[i + 1]), cv(rgba[i + 2]), rgba[i + 3]]);
                } else {
                    data.extend([0, 0, 0, 0]);
                }
            }
        }

        Ok((
            Image::new(
                Extent3d {
                    width: pw,
                    height: ph,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                data,
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
            ),
            pw,
            ph,
        ))
    }
}

/// Atlas cell (col, row) -> pixel rect. The tileset grid is 8 x 4.
pub fn tileset_rect(tile_px: u32, col: u32, row: u32) -> Rect {
    let p = tile_px as f32;
    let min = Vec2::new(col as f32 * p, row as f32 * p);
    Rect::from_corners(min, min + Vec2::splat(p))
}

/// Bake every part in `assets/images/`. Tintable parts are rendered as gray
/// silhouettes so the `Sprite::color` tint system keeps working; the bow
/// (drawn white) and the full-color background keep their art colors.
pub fn bake_warrior_art(images: &mut Assets<Image>) -> anyhow::Result<WarriorArt> {
    let mut baker = SvgBaker::new()?;

    let (tileset_img, atlas_w, _atlas_h) = baker.bake_atlas("assets/images/tileset.svg")?;
    // SVG canvas is 8 columns x 4 rows of equal cells.
    let tile_px = (atlas_w / 8).max(1);

    let mut bake = |svg_path: &str, tintable: bool| -> anyhow::Result<Handle<Image>> {
        Ok(images.add(baker.bake(svg_path, tintable)?))
    };
    Ok(WarriorArt {
        body: bake("assets/images/body.svg", true)?,
        head: bake("assets/images/head.svg", true)?,
        upper_arm: bake("assets/images/upper_arm.svg", true)?,
        hand: bake("assets/images/hand.svg", true)?,
        leg: bake("assets/images/leg.svg", true)?,
        bow: bake("assets/images/bow.svg", false)?,
        bg: bake("assets/images/bg.svg", false)?,
        tileset: images.add(tileset_img),
        tile_px,
    })
}

/// Replace every fill/stroke color with its sRGB luminance so `Sprite.color`
/// tinting can color each part without distortion.
fn grayscale_paints(doc: &mut Document) {
    for node in doc.nodes.values_mut() {
        if let NodeKind::Style(style) = &mut node.kind {
            let gray = gray_paint(style.paint());
            let _ = style.swap_paint(gray);
        }
    }
}

fn gray_paint(paint: &StylePaint) -> StylePaint {
    let lum = |c: Color| {
        let l = 0.299 * c.r + 0.587 * c.g + 0.114 * c.b;
        Color {
            r: l,
            g: l,
            b: l,
            a: c.a,
        }
    };
    match paint {
        StylePaint::Solid { .. } => StylePaint::solid(lum(paint.base_color())),
        StylePaint::Gradient(g) => {
            let mut out = g.clone();
            for stop in out.stops.base.0.iter_mut() {
                stop.color = lum(stop.color);
            }
            StylePaint::Gradient(out)
        }
    }
}

/// Un-premultiply and crop the offscreen render to the non-transparent content.
fn trim_to_image(rgba: &[u8], width: u32, height: u32) -> anyhow::Result<Image> {
    let (w, h) = (width as usize, height as usize);
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
    anyhow::ensure!(max_x >= min_x && max_y >= min_y, "SVG rasterized empty");
    let (cw, ch) = (max_x - min_x + 1, max_y - min_y + 1);
    let mut data = Vec::with_capacity(cw * ch * 4);
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let i = (y * w + x) * 4;
            let a = rgba[i + 3] as f32 / 255.0;
            if a > 0.0 {
                let cv = |v: u8| ((v as f32 / 255.0 / a).min(1.0) * 255.0) as u8;
                data.extend([cv(rgba[i]), cv(rgba[i + 1]), cv(rgba[i + 2]), rgba[i + 3]]);
            } else {
                data.extend([0, 0, 0, 0]);
            }
        }
    }
    Ok(Image::new(
        Extent3d {
            width: cw as u32,
            height: ch as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    ))
}
