//! Phase 1 LOOK SPIKE — the "Golden Beauty-Shot" hero scene.
//!
//! A frozen voxel kitchen framed to chase `docs/assets/golden-beauty-shot-ref.png`
//! (left window, golden-hour key light, warm bounce, god rays, DOF on the hero
//! bowl, stainless fridge, one teal/green accent block). The point of this module
//! is NOT terrain — it's to wire the FULL Bevy 0.19 lighting/post stack onto one
//! static frame so we can eyeball it against the reference and note what the
//! engine can/can't hit before generalising.
//!
//! Everything the shot needs to be re-framed (camera / sun / DOF / fog / exposure)
//! can be overridden by env vars parsed in `main::read_cfg`, so the scene can be
//! tuned and re-screenshotted WITHOUT another (slow) Bevy recompile.
//!
//! Those overrides are a TUNING path, not the source of truth. The CEO-approved
//! recipe is baked in `mod recipe` below, so running the shot binary with NO env at
//! all reproduces `docs/assets/wide-hero-final.png`. See `mod recipe` for why (it
//! did not, and the frame you got instead was red-clipped).

/// Set to `true` when the overlap gate fails. A static atomic so Bevy's window
/// close handler (which resets `AppExit` to `Success` during shutdown) cannot
/// overwrite it — the owning binary reads it AFTER `app.run()` returns and
/// calls `std::process::exit` with the right code.
pub static GATE_FAILED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Gate verdict resource — mutated by systems that detect FAIL, read by exit points.
/// Mirrors the static `GATE_FAILED` so systems that run before `app.run()` returns
/// can still check it without a static read. (Use `GATE_FAILED` after `app.run()`.)
#[derive(Resource, Default)]
pub struct GateVerdict {
    pub failed: bool,
}

use bevy::anti_alias::taa::TemporalAntiAliasing;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::light::{
    AmbientLight, DirectionalLightShadowMap, FogVolume, NotShadowCaster, ShadowFilteringMethod,
    VolumetricFog, VolumetricLight,
};
use bevy::pbr::{
    ContactShadows, DistanceFog, FogFalloff, ScreenSpaceAmbientOcclusion,
    ScreenSpaceAmbientOcclusionQualityLevel,
};
use bevy::post_process::bloom::Bloom;
use bevy::post_process::dof::{DepthOfField, DepthOfFieldMode};
use bevy::prelude::*;
use bevy::render::view::Msaa;
use bevy::render::view::{ColorGrading, ColorGradingGlobal, ColorGradingSection};
use bevy::camera::{
    Camera, ClearColorConfig, Exposure, PerspectiveProjection, Projection,
};

use crate::block_atlas::{self, AtlasMode, BlockAtlas};
use crate::Cfg;

/// Binds each palette material to the textured cube mesh it should be drawn with.
///
/// The scene authors ~60 `put`/`fill` calls that name a MATERIAL, never a mesh —
/// that is the vocabulary the whole room is written in, and rewriting every call
/// site to also pass a block kind would be a 600-line diff whose only content is
/// an argument. Keying off the material handle instead keeps the authoring
/// untouched: the palette declares "this handle is oak planks" once, and
/// [`VoxelGrid::flush`] looks up the right per-face cube when it spawns the cell.
///
/// Materials with no entry (steel, the window panes, dust motes) fall back to the
/// plain untextured cube, which is exactly what they want.
#[derive(Default)]
pub struct BlockSkins {
    mesh_for: std::collections::HashMap<AssetId<StandardMaterial>, Handle<Mesh>>,
    /// One mesh per KIND, not per material — two materials sharing a kind share
    /// the mesh handle, so Bevy still batches them into one draw call.
    by_kind: std::collections::HashMap<String, Handle<Mesh>>,
}

impl BlockSkins {
    /// Give `mat` the atlas texture and bind it to `kind`'s per-face cube.
    ///
    /// In `Detail` mode the material keeps its authored `base_color` and the tile
    /// (normalised to unit mean luminance) only adds variation — the signed-off
    /// grade is preserved by construction. In `Albedo` mode the tile IS the
    /// colour, so `base_color` drops to white or the art set gets tinted twice.
    fn skin(
        &mut self,
        atlas: &BlockAtlas,
        tex: &Handle<Image>,
        meshes: &mut Assets<Mesh>,
        mats: &mut Assets<StandardMaterial>,
        mat: &Handle<StandardMaterial>,
        kind: &str,
    ) {
        let Some(uv) = atlas.face_uv(kind) else {
            println!("ATLAS_SKIN missing kind {kind:?} — material left flat");
            return;
        };
        let mesh = self
            .by_kind
            .entry(kind.to_string())
            .or_insert_with(|| meshes.add(block_atlas::cube_mesh(uv)))
            .clone();
        if let Some(mut m) = mats.get_mut(mat) {
            m.base_color_texture = Some(tex.clone());
            if atlas.mode == AtlasMode::Albedo {
                m.base_color = Color::WHITE;
            }
        }
        self.mesh_for.insert(mat.id(), mesh);
    }

    fn mesh(&self, mat: &Handle<StandardMaterial>) -> Option<&Handle<Mesh>> {
        self.mesh_for.get(&mat.id())
    }
}

/// Warm golden-hour palette (linear-ish sRGB authoring; tone-map does the rest).
mod pal {
    use bevy::prelude::Color;
    // P0.3 charm (Flamingo #9): the foreground floor read as saturated plastic-RED
    // in shade, not warm wood. Lifted G/B toward honey (0.42,0.29,0.18 →
    // 0.44,0.32,0.21) — a small desaturate that pulls the red-dominance down toward
    // the ref's amber wood while staying warm (R>G>B). Pairs with the lifted WOOD_B.
    pub const WOOD_A: Color = Color::srgb(0.44, 0.32, 0.21); // walnut
    // P0.1 charm (Flamingo): espresso checker was crushing to near-black in the
    // foreground shade (reads as a black tile, not warm wood). Lifted 0.30→0.40 /
    // 0.20→0.28 / 0.12→0.17 to keep the dark pair WARM BROWN with visible grain in
    // shadow — softer checker contrast, no pure-black floor. Raises G3 margin.
    pub const WOOD_B: Color = Color::srgb(0.40, 0.28, 0.17); // espresso (checker pair)
    pub const COUNTER: Color = Color::srgb(0.52, 0.38, 0.24); // lighter top wood (honey-warmed, #9)
    pub const WALL_A: Color = Color::srgb(0.66, 0.55, 0.42); // warm beige
    pub const WALL_B: Color = Color::srgb(0.60, 0.50, 0.38); // block-grid pair
    pub const CABINET: Color = Color::srgb(0.40, 0.27, 0.16);
    pub const FRAME: Color = Color::srgb(0.24, 0.16, 0.09); // dark window frame
    pub const CERAMIC: Color = Color::srgb(0.86, 0.80, 0.68); // matte bowl
    pub const STEEL: Color = Color::srgb(0.72, 0.74, 0.78); // fridge
    pub const ACCENT: Color = Color::srgb(0.30, 0.52, 0.24); // muted moss-green accent (ref ≈88,87,27)
    pub const BOOK: Color = Color::srgb(0.88, 0.85, 0.78);
}

/// The CEO-approved beauty-shot recipe, BAKED (Flamingo / pixel lane, 2026-08-16).
///
/// SHIP-BLOCKER #3 (`docs/VERDICT-flamingo-beauty-2026-08-16.md` §4③): until this
/// block existed, the approved look lived ONLY in the 13 env vars that
/// `scripts/render_wide_hero.sh` exported. Anyone who built the tree and ran
/// `voxelforge_shot` with no env got a different frame — measured 2026-08-16 at
/// **mid-clip 44.36 %** (ceiling 35), mean RGB 147.9/39.6/9.4, and **0 px** of cool
/// accent: a red-clipped plate where the moss/teal accent had been crushed out of
/// gamut entirely. Worse, that clip made G3/G5/G6 read GREEN — "is it warm / is
/// R>G>B" is trivially true for a frame clipped to red — so the machine gates
/// could not see the failure. Two sources of truth, and the wrong one was the
/// default.
///
/// Every value below is copied verbatim from `scripts/render_wide_hero.sh` (the
/// recipe that produced the locked `docs/assets/wide-hero-final.png`). The env
/// vars still override for no-recompile sweeps — they are now a TUNING path, not
/// the only path to the shipped look.
mod recipe {
    /// `VOXELFORGE_CAM` — TILT-DOWN wide-B establishing cam: eye → target, fov.
    pub const CAM: [f32; 7] = [7.6, 6.4, -6.0, 7.6, 2.7, 8.0, 60.0];
    /// `VOXELFORGE_DOF` — focus 8 m, f/10: deep focus, so the establishing floor
    /// stays crisp voxel geometry (G1). The old f/1.4 default melted it.
    pub const DOF: [f32; 2] = [8.0, 10.0];
    /// `VOXELFORGE_SUN` — elevation°, azimuth°, illuminance. Key-light-dominant.
    pub const SUN: [f32; 3] = [19.0, 196.0, 26000.0];
    /// `VOXELFORGE_AMBIENT` — the old 4200 default was the ambient-DOMINATED
    /// balance that rendered flat fire-orange; 2800 hands the frame to the sun.
    pub const AMBIENT: f32 = 2800.0;
    /// `VOXELFORGE_BLUESCALE` — 0.85. The old 0.50 default halved the blue leg of
    /// sun + fog on top of an already-red grade; that is most of the 44 % clip.
    pub const BLUESCALE: f32 = 0.85;
    /// `VOXELFORGE_EXPOSURE` — ev100 9.0.
    pub const EXPOSURE: f32 = 9.0;
    /// `VOXELFORGE_GRADE` — temperature, post_saturation, contrast. Temperature
    /// drops 0.10 → 0.02: the old default pushed an already-warm frame redder.
    pub const GRADE: [f32; 3] = [0.02, 1.00, 1.30];
    /// `VOXELFORGE_SHOULDER` — filmic highlight roll-off. 0.64 seats window p95
    /// back inside the 150..185 band (measured 177.36; without it, 230).
    pub const SHOULDER: f32 = 0.64;
    /// `VOXELFORGE_DUST` — mote density in the god-ray corridor.
    pub const DUST: f32 = 3.0;
    /// GATE G4a — directional shadow-map resolution, and the ONLY lever on this
    /// scene that actually moves the penumbra.
    ///
    /// The measured penumbra on the wide plate was **4 px** against a `>=5` machine
    /// line and **8 px** on the golden ref. It is not a plate-resolution artefact:
    /// down-scaling the ref left its penumbra at 8→8→9 px, so the wide frame really
    /// is twice as hard-edged.
    ///
    /// PCSS is NOT the lever here. `soft_shadow_size` measured FLAT from 0.02 to 400
    /// because in a room this small the blocker→receiver depth gap is tiny and Bevy
    /// clamps `blur_size` to its 0.5 floor. What the visible penumbra is actually made
    /// of is `ShadowFilteringMethod::Temporal` + TAA sampling the shadow map — and
    /// that filter's kernel is denominated in shadow-map TEXELS. So the penumbra is
    /// proportional to texel world-size, i.e. inversely proportional to this number:
    /// 4096 → 2048 doubles the texel and should take 4 px to ~8 px.
    ///
    /// `voxel_shot`/`main` insert 4096 at app-build time; this is inserted from
    /// `setup_hero`, which runs after, so the look recipe wins — the shadow map is a
    /// LOOK knob and belongs with the rest of the baked recipe rather than in a bin's
    /// boilerplate. Sweep with `VOXELFORGE_SHADOWMAP=<n>` before moving it.
    pub const SHADOW_MAP: u32 = 2048;
    /// `VOXELFORGE_BOUNCE` / `_BOUNCE2` — floor-bounce and dark-lifter cards.
    pub const BOUNCE: f32 = 1.0;
    pub const BOUNCE2: f32 = 1.7;
    /// `VOXELFORGE_AMBCOLOR` — AMBER fill. The old default derived B from
    /// `0.180 * bluescale`, i.e. a fill so red the wide frame collapsed to
    /// fire-red; the recipe passes all three legs explicitly, so this does too.
    pub const AMBCOLOR: [f32; 3] = [0.70, 0.60, 0.44];
    /// The approved shot is the WIDE establishing scene, so the wide room
    /// dressing (parquet planks, honey plaster, teal-glass tumbler) is the
    /// default rather than an opt-in. `VOXELFORGE_WIDE=1` is kept working — it is
    /// now a no-op, so every lane script that already sets it renders unchanged.
    ///
    /// The `if !wide` branches below are the narrow calibration scene. They are
    /// deliberately left in place and NOT deleted: flipping this one const back
    /// to `false` restores them, which is the cheapest possible revert. They are
    /// unreachable at the moment because `Cfg.wide` is a presence-only bool
    /// (`env::var(..).is_ok()`), so it cannot express "explicitly off", and the
    /// struct is mirrored in `main.rs` — another lane's file.
    pub const WIDE: bool = true;
}

/// Build the whole hero scene: geometry + materials + sun + fog + camera stack.
pub fn setup_hero(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    cfg: Res<Cfg>,
) {
    // Single unit-cube mesh, instanced across the whole room (same mesh + same
    // material auto-batches, so a few thousand blocks stay cheap on a 1060).
    // Still the fallback for anything the atlas has no kind for.
    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));

    // ---- block texture atlas -------------------------------------------
    // The room was built out of flat `base_color` cubes, which is why the
    // signed-off beauty frame reads as untextured colour blocks: every "texture"
    // in it is really an adjacent-cell tone pair, i.e. the block-grid checker.
    // A failure to load is NOT fatal — the scene falls back to exactly the
    // frame that was signed off, and says why on stdout.
    let atlas = match block_atlas::load(None) {
        Ok(a) => a,
        Err(e) => {
            println!("ATLAS load failed ({e}) — flat materials kept");
            None
        }
    };
    let mut skins = BlockSkins::default();

    // ---- materials ------------------------------------------------------
    let matte = |c: Color, rough: f32| StandardMaterial {
        base_color: c,
        perceptual_roughness: rough,
        metallic: 0.0,
        reflectance: 0.1,
        ..default()
    };
    let wood_a = mats.add(matte(pal::WOOD_A, 0.9));
    let wood_b = mats.add(matte(pal::WOOD_B, 0.9));
    let counter = mats.add(matte(pal::COUNTER, 0.75));
    // P1 foreground-texture (Flamingo): the hero tabletop read as ONE flat matte
    // plate, so the grader's fg zone (island top below the bowl) measured almost no
    // hi-freq (fg/bg ≈ 0.34; golden ≈ 3.5, driven by its planked wood). A darker
    // walnut pair checkered against `counter` lays a plank/grain seam grid on the
    // tabletop — same warm hue (R>G>B) but big edge contrast → fg hi-freq. Kept
    // strictly darker-than-counter so it can NOT raise p95, and the dark tiles fall
    // below the midtone p35 band so warmth/blue/sat stay exactly where they passed.
    let counter_dk = mats.add(matte(Color::srgb(0.30, 0.21, 0.13), 0.82));
    let wall_a = mats.add(matte(pal::WALL_A, 0.97));
    let wall_b = mats.add(matte(pal::WALL_B, 0.97));
    let cabinet = mats.add(matte(pal::CABINET, 0.85));
    let frame = mats.add(matte(pal::FRAME, 0.8));
    let ceramic = mats.add(matte(pal::CERAMIC, 0.55));
    // P1 foreground-texture: a slightly shaded cream so the HERO bowl rim alternates
    // tile-to-tile — reads as bevelled voxel edges (like the golden ref's crisp bowl)
    // instead of one flat cream block. Only the hero bowl gets it; the far depth-cue
    // bowl stays plain so it does NOT add hi-freq to the blurred background zone.
    let ceramic_sh = mats.add(matte(Color::srgb(0.74, 0.68, 0.56), 0.55));
    let accent = mats.add(matte(pal::ACCENT, 0.7));
    let book = mats.add(matte(pal::BOOK, 0.6));

    // ---- WIDE geometry-pass materials (CEO sign-off 2026-07-26) ---------
    // Only used when VOXELFORGE_WIDE is set, so the locked narrow hero stays
    // byte-identical. Closes the ref look-gap the env-only pass parked:
    //   • floor: the 2-tone (x+z) CHECKER read as an orange/black chessboard.
    //     A PLANK palette (4 warm walnut/honey tones, LOW albedo spread, all
    //     R>G>B) laid as 1-wide boards running in Z (no cross-axis alternation)
    //     reads as continuous parquet like the ref — not a checkerboard.
    //   • tabletop: the light `counter` honey went flat-YELLOW filling frame
    //     centre; a slightly deeper planked wood pair reads as a real table so
    //     the cream bowl pops off it.
    //   • accent: the ref's cool note is a TEAL GLASS tumbler, not the muted
    //     moss block. Saturated teal, low roughness + a faint emissive glow so
    //     it reads as lit glass beside the bowl.
    // GEOMETRY PASS (Flamingo, 2026-07-27) — the de-checkered parquet was reading
    // as a FLAT red-orange sheet (albedo too red-dominant + tones too close under
    // the amber grade), not the ref's warm honey-walnut boards. Lifted the G/B legs
    // toward honey so the floor reads amber WOOD not plastic-red, and widened the
    // board-to-board spread a touch so individual planks separate at distance. Seam
    // lifted off near-black so joints read as grooves, not black gaps. All stay
    // R>G>B (G3 warm) and the lifted darks raise the shadow-floor p05-L margin.
    // Round 2: pushed the G leg up decisively (R-G gap ~0.13 → ~0.10) so the floor
    // reads honey-AMBER wood, not plastic-RED — the red-dominant albedo was fighting
    // the amber ambient into a flat orange sheet. Board-to-board tone spread widened
    // so individual planks separate at establishing distance.
    let plank_h = mats.add(matte(Color::srgb(0.55, 0.44, 0.29), 0.9)); // honey oak
    let plank_m = mats.add(matte(Color::srgb(0.48, 0.37, 0.24), 0.9)); // walnut
    let plank_d = mats.add(matte(Color::srgb(0.42, 0.32, 0.21), 0.9)); // dark walnut
    let plank_seam = mats.add(matte(Color::srgb(0.36, 0.28, 0.18), 0.92)); // board joint
    // GEOMETRY PASS (Flamingo, 2026-07-27): deepened the wide tabletop boards a
    // touch (was 0.50/0.44 R) so the bright flat-yellow slab becomes a warm mid
    // wood — the cream hero bowl now separates off the table instead of melting
    // into one blobby yellow mass. Still R>G>B, still below the cream bowl albedo.
    // Board pair kept a clear 2-tone (~0.11 spread): the foreground table fills the
    // near-camera, in-focus lower frame, so its plank-to-plank tone step is where the
    // ref carries most of its foreground voxel-grain. A crisp table seam grid restores
    // the micro-contrast the smooth walls gave up WITHOUT crushing any shadow (both
    // tones stay warm mid-wood, well above the G3 floor).
    let table_h = mats.add(matte(Color::srgb(0.49, 0.37, 0.24), 0.72)); // lit table board
    let table_d = mats.add(matte(Color::srgb(0.38, 0.28, 0.18), 0.72)); // table board pair
    // GEOMETRY PASS (Flamingo, 2026-07-27) — de-checker the walls for the WIDE
    // establishing frame. The shipped wall_a/wall_b pair (0.06 albedo spread)
    // read as a LOUD orange/yellow chessboard under the saturated key-lit grade —
    // the #1 "Minecraft-with-no-shader" tell vs the ref's smooth honey plaster.
    // A near-tone honey pair (~0.02 spread) keeps a whisper of voxel block-grid
    // (still reads as blocks up close) but resolves to a calm continuous wall at
    // establishing distance, like the ref. WIDE-only: narrow hero uses the
    // original pair unchanged (byte-identical lock preserved).
    // Pair kept ALMOST identical (~0.006 spread): a 0.02 spread still resolved to a
    // visible chessboard once the grazing key light + post contrast/saturation
    // amplified it on the brightly-lit walls. This close, blocks still read via
    // per-face lighting + AO (the voxel silhouette survives) while the wall reads
    // as smooth honey plaster at establishing distance, like the ref.
    let wall_a_w = mats.add(matte(Color::srgb(0.620, 0.520, 0.410), 0.97)); // honey plaster
    let wall_b_w = mats.add(matte(Color::srgb(0.614, 0.514, 0.404), 0.97)); // whisper-grid pair
    let glass_teal = mats.add(StandardMaterial {
        base_color: Color::srgb(0.10, 0.44, 0.42),
        emissive: LinearRgba::rgb(0.03, 0.24, 0.22),
        perceptual_roughness: 0.32,
        metallic: 0.0,
        reflectance: 0.5,
        ..default()
    });
    // ---- bind palette materials to atlas kinds --------------------------
    // The mapping is here, next to the palette it describes, and nowhere else.
    // A material named in this block gains real 16 px albedo; one left out keeps
    // the flat look on purpose — `steel`, `pane_lo`/`pane_hi` and the dust motes
    // are a mirror, a light source and sub-pixel specks, none of which want a
    // wood grain on them.
    //
    // `frame` is deliberately `log`: it is the one surface in the kitchen with a
    // real top/side split (cut end vs bark), so it is also the per-face proof
    // that the atlas addresses three windows and not one.
    if let Some(atlas) = atlas.as_ref() {
        let tex = images.add(atlas.image.clone());
        let mut bind = |mat: &Handle<StandardMaterial>, kind: &str| {
            skins.skin(atlas, &tex, &mut meshes, &mut mats, mat, kind);
        };
        // wood: cabinetry, counters, tabletops
        for m in [&wood_a, &wood_b, &counter, &counter_dk, &cabinet] {
            bind(m, "plank");
        }
        for m in [&table_h, &table_d] {
            bind(m, "plank");
        }
        // floor parquet — weathered board tile, distinct from interior planks
        for m in [&plank_h, &plank_m, &plank_d, &plank_seam] {
            bind(m, "floorboard");
        }
        // walls: lime plaster, the one cool-cast wall material in the set
        for m in [&wall_a, &wall_b, &wall_a_w, &wall_b_w] {
            bind(m, "plaster");
        }
        for m in [&ceramic, &ceramic_sh] {
            bind(m, "plaster");
        }
        bind(&frame, "log");
        bind(&accent, "leaves");
        bind(&book, "roof_tile");
        bind(&glass_teal, "glass");
    }

    // Parquet plank picker: tone varies per-BOARD (constant x) via a cheap
    // deterministic hash so neighbours differ without an A/B/A/B checker, with a
    // staggered board-joint seam every ~6 tiles down each plank (real flooring
    // offsets its joints per row). Returns the material for floor tile (x,z).
    let plank_at = |x: i32, z: i32| -> &Handle<StandardMaterial> {
        // staggered joint: seam every 6 down the board, offset by the board index
        if (z + x * 2).rem_euclid(6) == 0 {
            return &plank_seam;
        }
        // Interleave tones so EACH board differs from its neighbour (was 0|1→h,
        // which clustered honey boards into flat runs). Even spread h/m/d/m gives a
        // board-edge tone step at almost every plank seam → the floor reads as
        // separated planks at distance AND the extra edges restore the voxel-grain
        // (micro-contrast) the de-checkered walls gave up.
        match ((x.wrapping_mul(1103515245).wrapping_add(12345) >> 4) & 3) {
            0 => &plank_h,
            1 => &plank_m,
            2 => &plank_d,
            _ => &plank_m,
        }
    };
    // Table plank picker: boards run in Z too, 2-tone low-contrast wood.
    let table_at = |x: i32| -> &Handle<StandardMaterial> {
        if ((x.wrapping_mul(2654435761u32 as i32).wrapping_add(7) >> 3) & 1) == 0 {
            &table_h
        } else {
            &table_d
        }
    };
    // Every look knob below comes from `Cfg` — never `std::env::var` directly.
    // env::var returns Err on wasm32, so a direct read silently rendered the web
    // build with all-defaults while native honoured the recipe. Cfg is filled
    // from env on native and from the query string on web (see main.rs).
    // `recipe::WIDE` is the bake; `cfg.wide` (VOXELFORGE_WIDE) stays honoured so
    // callers that already export it are unaffected. See `mod recipe`.
    let wide = cfg.wide || recipe::WIDE;
    // Wall pair selector: the WIDE establishing frame gets the de-checkered honey
    // plaster (smooth at distance); the locked narrow hero keeps the shipped pair
    // exactly, so its signed frame stays byte-identical.
    let (wa, wb) = if wide {
        (&wall_a_w, &wall_b_w)
    } else {
        (&wall_a, &wall_b)
    };
    // Fridge: dielectric but smooth + reflective => a real specular streak from
    // the sun (true metallic would render black without an env-map).
    let steel = mats.add(StandardMaterial {
        base_color: pal::STEEL,
        // P2.5 charm (Flamingo): fridge read matte. Tighter roughness 0.18→0.12
        // narrows the specular lobe into a brighter VERTICAL streak down the
        // stainless face when the sun grazes it — "brushed metal", not flat.
        perceptual_roughness: 0.12,
        metallic: 0.0,
        reflectance: 0.7,
        ..default()
    });
    // Window pane: warm emissive => the in-frame light source + bloom seed.
    // Split into a low/high band with a vertical gradient (brighter near the
    // horizon, dimmer up top) so the window reads as a golden-hour SKY, not one
    // flat 255 plate — that gradient is exactly what G5/G6a grade. Emissive is
    // pulled well down from the old (6,4.2,1.8) blow-out so G,B roll off instead
    // of clipping; `VOXELFORGE_EMISSIVE` scales both bands for env-side tuning.
    let em = cfg.emissive.unwrap_or(1.0);
    let pane_lo = mats.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.86, 0.58),
        emissive: LinearRgba::rgb(3.1 * em, 2.35 * em, 1.35 * em),
        perceptual_roughness: 1.0,
        ..default()
    });
    let pane_hi = mats.add(StandardMaterial {
        base_color: Color::srgb(0.98, 0.84, 0.60),
        emissive: LinearRgba::rgb(2.3 * em, 1.85 * em, 1.25 * em),
        perceptual_roughness: 1.0,
        ..default()
    });

    // ---- geometry helpers ----------------------------------------------
    // Every grid-aligned cube in this scene is written into ONE cell map and
    // spawned once, at the end, by `grid.flush()`. Nothing below calls
    // `commands.spawn` for a unit cube directly — see `VoxelGrid` for why that
    // rule exists (short version: two cubes in one cell z-fight, the winner is
    // draw order, and draw order silently changed the golden image).
    let mut grid = VoxelGrid::default();

    // ---- room shell -----------------------------------------------------
    // Layout is authored for a camera looking toward +Z (screen-LEFT = +X):
    //   window on the +X wall (screen-left, the key light), fridge on the -X
    //   wall (screen-right), hero island front-centre, cabinets on the far +Z wall.
    // Floor: checkerboard wood (this is what reads as "voxel" in the ref).
    // Narrow hero only — the WIDE path lays a de-checkered parquet plank floor
    // across the full deepened footprint (z -14..16) in the VOXELFORGE_WIDE block.
    if !wide {
        for x in 0..16 {
            for z in 0..16 {
                let m = if (x + z) % 2 == 0 { &wood_a } else { &wood_b };
                grid.put(x, 0, z, m);
            }
        }
    }
    // Far wall (+Z) — carries the upper cabinets. Faint 2-tone block grid so
    // flat walls still read blocky.
    for x in 0..16 {
        for y in 1..9 {
            let m = if (x + y) % 2 == 0 { wa } else { wb };
            grid.put(x, y, 15, m);
        }
    }
    // Right-screen wall (-X, x=0) — solid, behind the fridge.
    for z in 0..16 {
        for y in 1..9 {
            let m = if (z + y) % 2 == 0 { wa } else { wb };
            grid.put(0, y, z, m);
        }
    }
    // Left-screen wall (+X, x=15) — holds the window hole (z 4..10, y 3..8).
    for z in 0..16 {
        for y in 1..9 {
            let is_window = (4..10).contains(&z) && (3..8).contains(&y);
            if is_window {
                continue;
            }
            let m = if (z + y) % 2 == 0 { wa } else { wb };
            grid.put(15, y, z, m);
        }
    }

    // ---- window: mullions + emissive pane ------------------------------
    // Bright pane sits OUTSIDE the wall (x = 16.5); the mullion bars on the wall
    // plane (x=15.5) occlude the volumetric light => banded god rays inside.
    // Lower band (y 3..5) brighter than upper band (y 5..8) => sky gradient.
    //
    // GATE G2 (2026-08-16): the pane is spawned NO-CAST. It used to be an ordinary
    // opaque cube wall sealing the opening from outside, which occluded the KEY
    // light before it reached the mullions — see `VoxelGrid::fill_nocast`. It banded
    // the VOLUMETRIC light (the fog pass reads the light, not the shadow map, so god
    // rays are unaffected) but never let a single bar of sun onto the floor.
    //
    // `VOXELFORGE_G2_PANE=block` restores the old occluding pane. That is the BEFORE
    // plate lever: one binary, one scene, one changed bit — so a before/after pair
    // cannot be two different builds wearing the same caption.
    let pane_blocks = std::env::var("VOXELFORGE_G2_PANE").as_deref() == Ok("block");
    // Annotated as a fn POINTER on purpose: the two arms are distinct fn *items* with
    // distinct types, and while rustc will coerce them here, spelling the pointer out
    // means the day one of the two signatures drifts the error lands on this line
    // instead of somewhere inside the if/else inference.
    type PaneFill = fn(&mut VoxelGrid, &Handle<StandardMaterial>, i32, i32, i32, i32, i32, i32);
    let pane_fill: PaneFill = if pane_blocks {
        VoxelGrid::fill
    } else {
        VoxelGrid::fill_nocast
    };
    pane_fill(&mut grid, &pane_lo, 16, 17, 3, 5, 4, 10);
    pane_fill(&mut grid, &pane_hi, 16, 17, 5, 8, 4, 10);
    println!(
        "G2_PANE={}",
        if pane_blocks { "block" } else { "pass" }
    );
    // Mullions across the opening (x=15 layer).
    grid.fill(&frame, 15, 16, 3, 8, 6, 7); // vertical mullion
    grid.fill(&frame, 15, 16, 5, 6, 4, 10); // horizontal mullion

    // ---- counters / cabinets -------------------------------------------
    // Counter run under the window (+X side).
    grid.fill(&cabinet, 11, 15, 0, 3, 1, 15);
    grid.fill(&counter, 11, 15, 3, 4, 1, 15);
    // Back counter run along the far wall.
    grid.fill(&cabinet, 4, 11, 0, 3, 12, 15);
    grid.fill(&counter, 4, 11, 3, 4, 12, 15);
    // Upper cabinets on the far wall.
    grid.fill(&cabinet, 4, 7, 6, 9, 13, 15);
    grid.fill(&cabinet, 8, 11, 6, 9, 13, 15);
    // A little open shelf with a book stack (the ref has one). The niche between
    // the two upper-cabinet runs is ONE column wide (they take x 4..7 and x 8..11),
    // so the stack has to be x 7..8. It used to be x 7..9, which put a book cube
    // inside the cabinet cube at (8,6,13) — see `report_voxel_overlaps` below: that
    // block flipped between cream book and dark cabinet from run to run.
    grid.fill(&book, 7, 8, 6, 7, 13, 14);

    // ---- fridge (screen-right, -X wall) --------------------------------
    grid.fill(&steel, 1, 4, 0, 8, 8, 12);

    // ---- hero island (front-centre) + the hero bowl --------------------
    // P0.2 charm (Flamingo): the bowl rim (5 wide, x5..9) used to span the WHOLE
    // island top (x5..9) edge-to-edge, so it read as "counter block", not a bowl
    // sitting ON a counter. Widened the island to x4..10 (7 wide) so there's a wood
    // margin on each side of the bowl — the silhouette now reads as a ceramic vessel
    // resting on the surface, like the ref.
    grid.fill(&cabinet, 4, 11, 0, 2, 2, 6);
    // Tabletop top surface (y=2..3): planked wood instead of one flat plate. Per-block
    // checker of counter/counter_dk lays a seam grid on the exact tiles the grader's
    // fg zone samples — the one in-scope lever for the fg/bg hi-freq axis.
    // Narrow only — the WIDE path re-lays the tabletop (z -10..6) in warm table-wood
    // boards so it reads as a table, not a flat-yellow slab, in the establishing frame.
    if !wide {
        for x in 4..11 {
            for z in 2..6 {
                let m = if (x + z) % 2 == 0 { &counter } else { &counter_dk };
                grid.put(x, 2, z, m);
            }
        }
    }
    // PROPOSAL PROBE (Poppy, env-gated — default OFF, signed frame untouched):
    // The grader's DOF fg zone (bottom-centre, rows 72-95%) lands on the SHADOWED
    // near-FLOOR (walnut/espresso checker, ~4-pt albedo gap crushed dark => fg-hf ~1.0),
    // while golden's fg zone is a LIT high-contrast planked TABLETOP (fg-hf ~8). 17
    // env-only frames (reframe/aperture/focus/sun) proved fg-hf is content-bound and
    // can't clear the axis. This extends the lit planked tabletop FORWARD (z -1..2) so
    // the bright counter/counter_dk checker (22-pt albedo gap) fills the fg zone
    // instead of the dark floor. VOXELFORGE_FGAPRON=1 to enable; bake only on sign-off.
    if cfg.fg_apron {
        grid.fill(&cabinet, 4, 11, 0, 2, -1, 2); // support under the apron
        for x in 4..11 {
            for z in -1..2 {
                let m = if (x + z) % 2 == 0 { &counter } else { &counter_dk };
                grid.put(x, 2, z, m);
            }
        }
    }
    // Hero bowl on the island top (y=3), centred ~ x7,z4 — the DOF focus point.
    // Rim alternates cream/shaded cream => bevelled edges (fg hi-freq on the subject).
    bowl(&mut grid, &ceramic, Some(&ceramic_sh), 7, 3, 4);
    // A second bowl far off on the back counter (depth cue, blurs out in DOF) — plain
    // rim: must NOT add hi-freq to the blurred background zone.
    bowl(&mut grid, &ceramic, None, 6, 4, 13);

    // ---- green accent block on the island (≤15% of frame) --------------
    // One small moss-green block beside the bowl — the single cool note that
    // makes the warm room sing (bible §4). Kept SMALL (~1% of frame like the
    // ref); a big saturated block walls off the cozy mood.
    // Narrow only — the WIDE path swaps this for a proper TEAL GLASS tumbler
    // beside the hero bowl (matches the ref's teal accent), placed in the block below.
    if !wide {
        grid.fill(&accent, 9, 10, 3, 5, 4, 6); // 1×2×2 standing accent
    }

    // ---- WIDE establishing dressing (env-gated · default OFF) ----------
    // The shipped/locked converged frame renders WITHOUT this flag, so it stays
    // byte-identical (VOXELFORGE_WIDE unset => this whole block is skipped).
    // With VOXELFORGE_WIDE=1 the room is dressed for a pulled-back, TILT-DOWN
    // establishing shot (golden-ref framing) that never reveals void and reads
    // the bowl as a vessel on a long warm tabletop.
    //
    // Redesign notes (Pixel, 2026-07-26) — supersedes the first wide attempt
    // (pp-w1/pp-w2), which dyed the top of frame deep RED and read muddy:
    //   • ROOT CAUSE of the red: a bright warm-WOOD ceiling (y=13, `counter`
    //     material) sat directly over the emissive window and trapped the
    //     god-ray fill, so a slightly-up tilt caught a glowing orange slab
    //     across the whole top. FIX: no ceiling at all. On a tilt-DOWN
    //     establishing angle the top of frame lands on the far WALL/cabinets,
    //     so the room is capped by taller MATTE walls (wall_a/wall_b, low
    //     reflectance) instead — they hold warm without blooming red.
    //   • Walls grow to y=14 across the full (now deeper) footprint so the
    //     downward tilt + wide FOV never sees the dark void above the shipped
    //     8-tall shell.
    //   • The checker floor + the planked island top RUN FORWARD to z=-14/-10
    //     so the near foreground is one continuous warm tabletop/floor.
    // Bake as the default framing ONLY on Flamingo + CEO sign-off.
    if wide {
        // 1) DE-CHECKERED parquet floor across the FULL deepened footprint
        //    (z -14..16 — the base checker floor is skipped when wide). Boards
        //    run in Z (recede toward the window like the ref); tone varies per
        //    board via `plank_at`, staggered joints break the board lengths.
        //    Reads as continuous warm parquet, not an orange/black chessboard.
        for x in 0..16 {
            for z in -14..16 {
                let m = plank_at(x, z);
                grid.put(x, 0, z, m);
            }
        }
        // 2) Taller room — raise the far wall (+Z) to y=14 (blocky grid). NO
        //    bright ceiling: the top of a tilt-down frame lands here, on wall.
        for x in 0..16 {
            for y in 9..14 {
                let m = if (x + y) % 2 == 0 { wa } else { wb };
                grid.put(x, y, 15, m);
            }
        }
        // Side walls: raise to y=14 across the full deepened footprint (z -14..16)…
        for z in -14..16 {
            for y in 9..14 {
                let m = if (z + y) % 2 == 0 { wa } else { wb };
                grid.put(0, y, z, m);
                grid.put(15, y, z, m);
            }
        }
        // …and close the FOREGROUND side stretch (z -14..0) full height so a wide
        //    FOV can't see past where the shipped walls stop at z=0. Both sides
        //    are solid here — the window opening stays back at its shipped z 4..10.
        for z in -14..0 {
            for y in 1..9 {
                let m = if (z + y) % 2 == 0 { wa } else { wb };
                grid.put(0, y, z, m);
                grid.put(15, y, z, m);
            }
        }
        // 3) Run the hero island body + its WOOD-PLANK top across the full length
        //    (z −10..6 — base tabletop skipped when wide) so the bowl rests on a
        //    long warm TABLE that fills the foreground like the ref, reading as
        //    wood boards instead of a flat-yellow slab.
        grid.fill(&cabinet, 4, 11, 0, 2, -10, 2);
        for x in 4..11 {
            for z in -10..6 {
                let m = table_at(x);
                grid.put(x, 2, z, m);
            }
        }
        // 4) TEAL GLASS tumbler beside the hero bowl (bowl sits at x7,z4). Placed
        //    screen-right + forward of the bowl (lower x, nearer z) to mirror the
        //    ref's bowl-left / teal-right foreground pairing. A 2×2 vessel, 2 tall,
        //    with a hollowed top and a 1-block handle nub so it reads as a mug/glass,
        //    not a plain cube. Sits on the tabletop top (y=3).
        //
        //    THE HOLE (Flamingo, 2026-07-31 — root cause of the missing teal voxel
        //    in `wide-hero-final.png`; full write-up in docs/voxel-hole-findings.md).
        //    The tumbler's top course reaches INTO the hero bowl's rim ring: `bowl()`
        //    lays a hollow 5×5 ring at y = base_y+1 = 4 spanning x 5..9 / z 2..6, so
        //    its near-left corner block sits on (5,4,2) — and the top course below
        //    claims that same cell. Nothing here moved; the placement is what the
        //    ref-matching web frame (`docs/assets/wasm-hero-v3.png`) shows. What was
        //    broken is that the OLD code spawned both cubes and let the renderer pick:
        //    two opaque unit cubes at one transform z-fight, the winner is draw order,
        //    and draw order is not a property of this file. So the block was teal on
        //    web and gone on native, and the locked golden lost a voxel without a
        //    single line here changing.
        //
        //    THE FIX is `VoxelGrid` (top of this fn): every cube goes into ONE cell
        //    map, last write wins, and the map is spawned sorted. The tumbler is
        //    written after `bowl()`, so (5,4,2) resolves to teal — deterministically,
        //    and matching web. That is the whole change: no re-stage, no camera move,
        //    the CEO-approved composition is untouched.
        //
        //    (5,4,2) is therefore a DECLARED overwrite, not an accident, and
        //    `scripts/voxel_hole_proof.py` carries it in EXPECTED_OVERWRITES with
        //    this reason. Any OTHER shared cell fails that script. Keep it that way:
        //    an overwrite nobody wrote down is the exact bug this comment is about.
        {
            let (gx, gz) = (4, 2); // screen-right of the bowl, one row forward
            // solid 2×2 base course (y=3)
            grid.fill(&glass_teal, gx, gx + 2, 3, 4, gz, gz + 2);
            // upper course (y=4): open one back-inner corner so the top reads hollow
            for x in gx..gx + 2 {
                for z in gz..gz + 2 {
                    if x == gx + 1 && z == gz + 1 {
                        continue; // hollow notch
                    }
                    grid.put(x, 4, z, &glass_teal);
                }
            }
            // handle nub on the screen-left face (+x side), mid height
            grid.put(gx + 2, 3, gz, &glass_teal);
        }

        // 5) Pin 3 (LOOK): DUST MOTES in the god-ray. The volumetric shaft reads
        //    as a clean gradient with nothing IN it; real golden-hour light is
        //    full of lit dust. A field of tiny warm-emissive specks scattered
        //    through the window-beam corridor catches the key light → floating
        //    motes that give the shaft texture + a little hi-frequency sparkle
        //    (micro-contrast) the flat air lacks. Deterministic positions (frozen
        //    frame, so TAA doesn't smear them). env VOXELFORGE_DUST scales density
        //    (0 = off). Specks are sub-pixel-small so they can't move p95, but the
        //    hi-pass grain they add nudges micro-contrast up.
        let dust: f32 = cfg.dust.unwrap_or(recipe::DUST);
        if dust > 0.0 {
            // hash(i, salt) -> [0,1): a cheap integer mix so motes scatter in 3D
            // instead of falling on a lattice (three different salts per mote).
            let hash = |i: i32, s: u32| -> f32 {
                let mut v = (i as u32).wrapping_mul(0x9E3779B1).wrapping_add(s);
                v ^= v >> 15;
                v = v.wrapping_mul(0x85EBCA77);
                v ^= v >> 13;
                (v & 0xFFFF) as f32 / 65535.0
            };
            let mote_mat = mats.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.92, 0.72),
                emissive: LinearRgba::rgb(2.4, 1.85, 1.0),
                perceptual_roughness: 1.0,
                ..default()
            });
            let mote_mesh = meshes.add(Cuboid::new(0.06, 0.06, 0.06));
            let n = (70.0 * dust) as i32;
            for i in 0..n {
                // Corridor between the +X window (x≈15) and the room centre, at
                // beam height, spanning the window's z-opening — where the shaft is.
                let x = 4.0 + hash(i, 0x1111) * 11.0; // 4..15
                let y = 2.2 + hash(i, 0x2222) * 5.0; // 2.2..7.2
                let z = 3.0 + hash(i, 0x3333) * 8.0; // 3..11
                commands.spawn((
                    Mesh3d(mote_mesh.clone()),
                    MeshMaterial3d(mote_mat.clone()),
                    Transform::from_xyz(x, y, z),
                ));
            }
        }
    }

    // ---- commit the voxel grid ------------------------------------------
    // Every cube above was a WRITE into one cell map; this is the only place a
    // grid cube is actually spawned, exactly once per occupied cell. Later
    // writes overwrote earlier ones, which is what the authoring above already
    // assumed ("the fridge stands ON the floor tile", "the counter caps the
    // cabinet") — it just used to express that as two coincident cubes and let
    // the GPU pick. `report_voxel_overlaps` is the standing proof it stays that
    // way; the dust motes below are the only cubes that bypass the grid, and
    // they're a different mesh at fractional positions, so they can't collide.
    let placed = grid.flush(&mut commands, &cube, &skins);
    println!("VOXEL_CELLS={placed}");

    // ---- sun (golden key, streaming through the +X window) -------------
    // P0-BLUE (grade-vs-golden): midtone B was ~34 (target <=10). The dominant
    // source is the KEY light's own blue leg multiplying every lit surface. A
    // single scalar pulls the B leg of sun + ambient fill + fog DOWN together
    // (env `VOXELFORGE_BLUESCALE`; 1.0 = prior look). Lowering blue (not adding
    // red) raises R-B and saturation at the same time — the brief's
    // "ลดฟ้า ไม่ใช่ดันแดงกลบ". Now `recipe::BLUESCALE` = 0.85: the old 0.50 default
    // was tuned for the narrow hero, and on the wide frame it cut so much blue on
    // top of the red grade that the midtones clipped (44 % — ship-blocker #3).
    let bscale: f32 = cfg.bluescale.unwrap_or(recipe::BLUESCALE);
    let (elev, azim, illum) = cfg.sun.unwrap_or(recipe::SUN).into_tuple3();
    let dir = sun_dir(elev, azim);
    // GATE G4a penumbra — see `recipe::SHADOW_MAP` for why this, and not PCSS, is the
    // lever. Read straight from env rather than through `Cfg`: `Cfg` is declared in
    // `main.rs`, another lane's file, and this knob must not need an edit there.
    let shadow_map: u32 = std::env::var("VOXELFORGE_SHADOWMAP")
        .ok()
        .and_then(|s| s.trim().parse().ok())
        // Bevy: "must be a power of two to avoid unstable cascade positioning" — a
        // swept 3000 would silently jitter the cascades and poison the very number
        // the sweep is trying to read, so reject it rather than honour it.
        .filter(|n: &u32| *n >= 256 && n.is_power_of_two())
        .unwrap_or(recipe::SHADOW_MAP);
    println!("SHADOW_MAP={shadow_map}");
    commands.insert_resource(DirectionalLightShadowMap {
        size: shadow_map as usize,
    });
    // Position the light off the room and aim it in; direction is what matters.
    let sun_pos = Vec3::new(8.0, 6.0, 6.0) - dir * 40.0;
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(1.0, 0.80, 0.52 * bscale),
            illuminance: illum,
            shadow_maps_enabled: true,
            // GATE G4 soft edge. The visible penumbra (~6px measured, up from the
            // old ~1px hard PCF) actually comes from `ShadowFilteringMethod::Temporal`
            // + TAA + the 4K shadow map on the camera below — NOT from PCSS.
            // `experimental_pbr_pcss` is enabled and `soft_shadow_size` is set, but in
            // a room this small the directional blocker→receiver depth gap is tiny, so
            // Bevy clamps `blur_size` to its 0.5 floor and the penumbra does NOT scale
            // with occluder distance (measured flat from soft=0.02 to 400). So bible
            // pass-4c (distance-proportional penumbra) is NOT achieved here; the gate's
            // uniform-soft-edge requirement IS. env `VOXELFORGE_SOFT` kept for tuning.
            // The field itself only EXISTS behind `experimental_pbr_pcss`, and the web
            // build drops it (index.html passes --no-default-features --features webgpu
            // because the PCSS shader does not compile under Tint) — so it has to be
            // cfg'd out or wasm32 fails with E0560 on this line. Native/hero-shot builds
            // keep the cargo default and still get PCSS.
            #[cfg(feature = "experimental_pbr_pcss")]
            soft_shadow_size: Some(cfg.soft.unwrap_or(3.0)),
            // Wider penumbra needs more depth-bias headroom or the soft edge
            // self-shadows into acne on the flat wood; normal-bias keeps block
            // faces off each other.
            shadow_depth_bias: 0.10,
            shadow_normal_bias: 2.2,
            ..default()
        },
        Transform::from_translation(sun_pos).looking_at(Vec3::new(8.0, 4.0, 7.0), Vec3::Y),
        VolumetricLight, // <- makes this light visible as god rays in the fog
    ));

    // ---- Pin 1 (LOOK): one-bounce directional GI fill (WIDE-A) ----------
    // The uniform AmbientLight below is a FLAT hemisphere wash — every face of
    // every block gets the same fill, so the establishing frame collapses into
    // one monochrome orange sheet (the #1 AAA-killer + what pins micro-contrast
    // at the 4.9 near-miss). Real GI here is faked the way it is before you have
    // lightmaps/DDGI: two SHADOWLESS directional "bounce cards" stand in for the
    // dominant indirect paths, so the shade carries a DIRECTION (window-side warm
    // & bright → far side deep amber) instead of being flat. WIDE-only so the
    // separately-locked narrow hero stays byte-identical; `VOXELFORGE_BOUNCE`
    // scales both cards (0 = old flat look) and pairs with a cut to AMBIENT so
    // total exposure is held while the fill gains directionality.
    if wide {
        // Two bounce cards, independently scaled so the tuning can push the
        // dark-lifter (card 2) hard for G3's p05 WITHOUT the broad floor bounce
        // (card 1) inflating the p95 highlight band — they pull opposite axes.
        let b1: f32 = cfg.bounce.unwrap_or(recipe::BOUNCE);
        let b2: f32 = cfg.bounce2.unwrap_or(recipe::BOUNCE2);
        // 1) FLOOR BOUNCE — the sunlit honey parquet throws warm light UP and
        //    across toward the shaded -X wall. Lights undersides (counter lip,
        //    bowl foot, table edge) + the far shade wall with indirect amber that
        //    the top-down sun never reaches. Travels up + toward -X/+Z. Broad
        //    coverage → the main p95 contributor of the two, so kept modest.
        commands.spawn((
            DirectionalLight {
                color: Color::srgb(1.0, 0.63, 0.28),
                illuminance: 3040.0 * b1,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_translation(Vec3::new(8.0, 0.5, 8.0))
                .looking_at(Vec3::new(3.0, 6.5, 12.0), Vec3::Y),
        ));
        // 2) BACK-WALL RAKE — the sunlit near floor also bounces DEEP into the
        //    room toward the far wall, lighting the camera-facing (-Z) cabinet
        //    fronts the top-down sun + window key never reach. Those deep-shade
        //    cabinet faces are the darkest 5% of the frame (they pin G3's p05),
        //    so a warm fill travelling +Z lifts THEM specifically instead of
        //    washing the whole frame flat — p05 up, micro-contrast preserved.
        commands.spawn((
            DirectionalLight {
                color: Color::srgb(1.0, 0.75, 0.42),
                illuminance: 1200.0 * b2,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_translation(Vec3::new(8.0, 6.5, -9.0))
                .looking_at(Vec3::new(7.0, 5.0, 15.0), Vec3::Y),
        ));
    }

    // (warm bounce / GI fill is attached to the camera below — AmbientLight is a
    // per-view Component in 0.19, not a resource; honey tint keeps shadows off
    // pure black, bible pass #2. On web a baked lightmap/irradiance volume takes
    // this role.)

    // ---- volumetric fog volume (the god-ray medium) --------------------
    // Baked 0.032 (was 0.06): the approved converged frame (cv-amb2900 ==
    // hero-converged-final.png) rendered with FOG=0.032 — MEDIUM haze that keeps
    // the god-ray beams readable per spec §3 ("ไม่ fog ทึบ"). 0.06 hazed the room
    // and violated the spec, so the env value is now the ONLY default.
    let fog_density = cfg.fog.unwrap_or(0.032);
    commands.spawn((
        FogVolume {
            // God-ray medium: still golden (it IS sunlit air) but B lifted a touch
            // so the beams don't paint the whole room pure orange.
            fog_color: Color::srgb(1.0, 0.88, 0.70 * bscale),
            density_factor: fog_density,
            scattering: 0.55,
            ..default()
        },
        Transform::from_translation(Vec3::new(8.0, 5.0, 8.0)).with_scale(Vec3::new(20.0, 12.0, 20.0)),
    ));

    // ---- camera + full post stack --------------------------------------
    // Baked to `recipe::CAM` — the CEO-approved TILT-DOWN wide-B establishing cam:
    // eye 7.6,6.4,-6.0 → target 7.6,2.7,8.0, FOV 60°, the framing behind the locked
    // docs/assets/wide-hero-final.png. It replaces the earlier converged NARROW
    // framing (7.6,5.9,-5.2 → 7.6,3.2,6.0, FOV 52°), which was itself signed off but
    // is no longer what ships — and which, left as the default, only ever rendered
    // correctly WITH env crutches. Env still overrides for re-framing sweeps.
    let cam = cfg
        .cam
        .unwrap_or(recipe::CAM);
    let eye = Vec3::new(cam[0], cam[1], cam[2]);
    let target = Vec3::new(cam[3], cam[4], cam[5]);
    let fov = cam[6].to_radians();
    // Baked DOF = `recipe::DOF`: focus 8 m at aperture f/10. A wide ESTABLISHING
    // frame wants deep focus — the whole point of the pulled-back shot is that the
    // floor stays crisp voxel geometry (G1), so the shallow f/1.4 the narrow hero
    // used would melt the subject. Consequence, documented not hidden: the grader's
    // DOF fg:bg axis wants shallow-DOF-with-textured-foreground and so reads 0.17
    // against a target of 3 — an INTRINSIC miss for this shot class, unchanged by
    // this bake (the approved wide-hero-final.png measures the same 0.17/0.18).
    // Env still overrides for re-framing.
    let (focus, aperture) = cfg.dof.unwrap_or(recipe::DOF).into_tuple2();
    // Exposure = `recipe::EXPOSURE` (ev100 9.0). Reins in the blown right-side wall /
    // window highlights, keeping filmic roll-off instead of clipping to paper-white.
    // It runs a half-stop HOTTER than the old 9.5 narrow-hero default because the
    // wide recipe pairs it with a real highlight shoulder (0.64) that the narrow path
    // never applied; measured together they land p95 177.36 inside the 150..185 band.
    // Exposure alone is not the highlight tool here — see the shoulder note below.
    // Env `VOXELFORGE_EXPOSURE` overrides.
    let exposure_ev = cfg.exposure.unwrap_or(recipe::EXPOSURE);
    // P0 post color-grade (grade-vs-golden, applied AFTER AcesFitted tonemap):
    //   temperature +  -> shifts chromaticity redder == pulls the residual midtone
    //     BLUE down (the honey ambient re-tint alone left mid-B ~24 vs golden 4; the
    //     wash lived in the tone-mapped output, so it's killed HERE not in the fill).
    //   post_saturation > 1 -> the voxel materials tonemap flat (~80% sat vs golden
    //     96%); this re-saturates the whole frame toward the golden's punch.
    //   contrast > 1 -> spreads values off mid-grey == raises local high-pass energy
    //     (micro-contrast / voxel grain) back toward the golden's crisp read.
    // Env `VOXELFORGE_GRADE=temp,sat,contrast` overrides for no-recompile sweeps.
    let (g_temp, g_sat, g_contrast) = cfg.grade.unwrap_or(recipe::GRADE).into_tuple3();

    // Pin 3 (LOOK): filmic highlight SHOULDER for the WIDE frame — the "LUT" half
    // of the grade. Pin 1's bounce fill lifts the shade (G3 p05) but, because the
    // baseline p95 already sat near the top of the 150..185 band, added fill pushes
    // the highlight band over too — and lighting alone can't lift shadows AND pull
    // highlights (both are just "more light", they move together). A per-section
    // highlight roll-off (gain < 1) is the right tool: it compresses ONLY the
    // brightest surfaces (sunlit wedge + window) back into band while leaving the
    // bounce-lit shade + midtones untouched — the range compression a flat AcesFitted
    // curve can't do. Still branch-gated on `wide` (the narrow calibration grade is
    // preserved verbatim for the revert), but since `recipe::WIDE` is the default the
    // shoulder is now ALWAYS applied out of the box. That gate is exactly what made
    // the old bare run blow out: with the shoulder skipped, p95 measured 230 against
    // a 150..185 band. env VOXELFORGE_SHOULDER (1.0 = no shoulder).
    let shoulder: f32 = cfg.shoulder.unwrap_or(recipe::SHOULDER);
    let (hi_contrast, hi_gain) = if wide { (1.0, shoulder) } else { (g_contrast, 1.0) };

    // Warm-bounce fill COLOUR = `recipe::AMBCOLOR` (0.70,0.60,0.44) — AMBER, matching
    // the ref's higher G leg. The old default was the narrow hero's honey tint with a
    // bscale-derived blue leg (0.784, 0.541, 0.180 × bscale ≈ 0.09): under it the WIDE
    // frame — which is ambient-DOMINATED over huge floor/wall areas — collapsed to
    // FIRE-RED and clipped. All three legs are now explicit, so the fill colour no
    // longer moves when someone sweeps VOXELFORGE_BLUESCALE. `VOXELFORGE_AMBCOLOR`
    // still overrides.
    let amb_col = cfg.ambcolor.unwrap_or(recipe::AMBCOLOR);

    let mut cam = commands.spawn((
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(0.05, 0.03, 0.02)),
            ..default()
        },
        Projection::Perspective(PerspectiveProjection {
            fov,
            near: 0.05,
            ..default()
        }),
        Transform::from_translation(eye).looking_at(target, Vec3::Y),
        Msaa::Off, // SSAO requires MSAA off; edges stay crisp anyway (voxel look)
        // Warm-bounce fill (bible pass #2). DESATURATED honey — old (0.78,0.55,0.30)
        // was so orange it dyed every surface one hue (monochrome collapse); lifting
        // B 0.30→0.50 keeps it warm (R>G>B) while letting materials hold their own
        // colour (cream ceramic / grey steel / green accent survive). Brighter floor
        // (env `VOXELFORGE_AMBIENT`) pulls open-shadow luminance off pure black so the
        // wood grain stays readable in shade (G3).
        AmbientLight {
            // P0-BLUEWASH (grade-vs-golden): the fill was tinted (0.90,0.66,0.42) — its
            // B/G leg was washing the midtones cool (measured midtone B high, saturation
            // low, R-B under target). Re-tinted to honey #C88A4A (0.784,0.541,0.290): a
            // deeper, more saturated golden with the blue leg pulled well down, so shade
            // + midtone surfaces read amber (warm R-B up, midtone blue down, saturation
            // up toward the golden). Still R>G>B so G3 stays "warm & not-blue".
            // P0 blue-wash (round 2): B leg 0.290 -> 0.180. The post grade kills most
            // of the midtone blue, but pulling the fill's own blue leg down at source
            // stops shade pixels being dyed cool before the grade even runs (mid-B
            // was still ~24 vs golden 4 on tint alone). Still R>G>B so G3 stays warm.
            color: Color::srgb(amb_col[0], amb_col[1], amb_col[2]),
            // Ambient POWER = `recipe::AMBIENT` (2800). The narrow hero climbed this to
            // 4200 because there, ambient power was what drove warmth (R-B) and cutting
            // it collapsed the axis. The WIDE recipe reaches warmth a different way —
            // KEY-LIGHT-DOMINANT: sun illuminance more than doubles (12000 → 26000) and
            // ambient drops to 2800, plus two directional bounce cards below carry the
            // fill. An ambient-DOMINATED wide frame is precisely what rendered flat
            // fire-orange. Measured on the baked default: warmth 125.90, G3 interior
            // p05-L 14.3 % (gate >=8). Env `VOXELFORGE_AMBIENT` overrides.
            brightness: cfg.ambient.unwrap_or(recipe::AMBIENT),
            affects_lightmapped_meshes: false,
        },
        Exposure { ev100: exposure_ev },
        Tonemapping::AcesFitted,
        // P0 filmic grade — kills the residual blue-wash + restores saturation/grain
        // that AcesFitted flattens. Contrast lands on MIDTONES + HIGHLIGHTS only —
        // the micro-contrast axis is carried by lit wood-grain / edge detail there.
        // The SHADOWS section is held neutral: applying contrast to shadows crushed
        // the open-shade floor to pure black (G3 interior p05-L 13.7% -> 3.3%, a
        // regression), so shadows keep their warm bounce fill instead of clipping.
        ColorGrading {
            global: ColorGradingGlobal {
                temperature: g_temp,
                post_saturation: g_sat,
                ..default()
            },
            shadows: ColorGradingSection { contrast: 1.0, ..default() },
            midtones: ColorGradingSection { contrast: g_contrast, ..default() },
            // WIDE highlights roll off via `hi_gain` (Pin 3 shoulder); the narrow
            // hero keeps the shipped `g_contrast` highlight (hi_gain = 1.0).
            highlights: ColorGradingSection { contrast: hi_contrast, gain: hi_gain, ..default() },
        },
        // Temporal shadow filter + TAA: PCSS soft shadows and Ultra SSAO both use
        // stochastic samples that are noisy for a single frame; TAA accumulates
        // them across the frames rendered before the 3.2s screenshot into clean
        // penumbra + contact AO. Voxel edges stay 90° (sub-pixel jitter only).
        ShadowFilteringMethod::Temporal,
        TemporalAntiAliasing::default(),
        Bloom {
            // CHARM edge-softening (Flamingo): voxel highlight edges read hard/CG.
            // Nudged 0.30→0.34 so the window + lit bowl-rim + sunlit wood carry a
            // slightly wider golden bleed that rounds the 90° highlight edges the way
            // the ref does — NATURAL's high threshold still keeps it off the shadows,
            // so G3/G5 don't move.
            // P0 micro-contrast: bloom was bleeding highlight energy across the voxel
            // edges and softening the grain (micro 3.8 vs golden 5.2). Trimmed
            // 0.34 -> 0.26 so highlight edges stay crisper; NATURAL threshold still
            // keeps it off the shadows so G3/G5 don't move.
            intensity: 0.26,
            ..Bloom::NATURAL
        },
        DepthOfField {
            mode: DepthOfFieldMode::Bokeh,
            focal_distance: focus,
            aperture_f_stops: aperture,
            // Bevy's circle-of-confusion scales as focal_length²/(sensor·N), and with
            // the physical ~18.6mm default sensor the CoC is near-zero at this room
            // scale — even f/0.1 barely blurred (the far cabinets stayed razor sharp,
            // which is exactly what the reviewer caught). Use a large-format sensor so
            // a cinematic f/1.4–2.0 melts the background into real bokeh like the ref.
            sensor_height: 0.35,
            ..default()
        },
        // GATE G4 (contact AO): Ultra sampling + a voxel-scale thickness so the
        // crevices under the bowl rim / block feet / counter joins read as dark
        // contact shadows instead of floating. Default (Medium-ish, 0.25 thick)
        // was too faint to see against the warm bounce fill.
        ScreenSpaceAmbientOcclusion {
            quality_level: ScreenSpaceAmbientOcclusionQualityLevel::Ultra,
            // CHARM contact-shadow (Flamingo): the bowl foot / accent block / counter
            // joins read a touch floaty vs the ref's firm grounding. Thickened
            // 1.25→1.45 so the contact crease under each object darkens a little more
            // and the pieces sit ON the wood instead of hovering. Warm bounce still
            // keeps the crease off pure-black (G3), so it grounds without going murky.
            constant_object_thickness: 1.45,
        },
        // The two fog components are grouped in a nested tuple: the camera bundle
        // otherwise hits 16 top-level items and Bevy's Bundle tuple impls stop at 15.
        // A nested tuple is itself a Bundle, so this is one slot (no behaviour change).
        (VolumetricFog {
            // P1.4 charm (Flamingo): god-ray was a hard triangular wedge. More
            // jitter (0.4→0.6) dithers the ray-march step boundary so TAA resolves
            // it into a SOFT-edged shaft instead of a crisp cut, closer to the ref's
            // diffuse mullion beams. step_count held at 96 so the beam stays smooth.
            ambient_intensity: 0.08,
            step_count: 96,
            jitter: 0.6,
            ..default()
        },
        DistanceFog {
            // Desaturated + thinner than the old (0.55,0.40,0.26)@0.012: that orange
            // haze over depth was flattening the far wall/fridge into the same hue as
            // the foreground. Lifted B + lower density lets distance keep material tint.
            color: Color::srgb(0.50, 0.42, 0.28),
            falloff: FogFalloff::Exponential {
                density: cfg.dfog.unwrap_or(0.008),
            },
            ..default()
        },
        // GATE G4b (contact AO) — "บล็อกมิ้นต์/ชามไม่มีเงาสัมผัส นั่งลอย".
        //
        // SSAO above cannot close this and raising `constant_object_thickness`
        // further will not either: SSAO attenuates the AMBIENT term, and at the
        // foot of a sunlit block the ambient is the small half of the light, so the
        // crease it can draw is capped by the fill's share. `ContactShadows`
        // ray-marches the depth buffer toward the light and attenuates the DIRECT
        // term instead — at a block's foot that is the whole key light to bite into,
        // which is what puts the seam back that a shadow cascade covering the room
        // is far too coarse to resolve. Same argument look.rs made for the outdoor
        // lane; these are its proven v3 numbers, at the same 1-unit-per-block scale.
        //
        // Inserted AFTER the spawn rather than as a bundle item, for two reasons:
        // the camera bundle is already at Bevy's 15-item tuple ceiling (see the
        // comment above), and `Option<C>` is NOT a `Bundle` in Bevy 0.19 — checked,
        // not assumed — so `None` cannot be the off-switch inside a tuple.
        // `VOXELFORGE_G4_CONTACT=off` disables it: the BEFORE lever for this half of
        // G4, same one-binary rule as `VOXELFORGE_G2_PANE`.
        ),
    ));
    if let Some(cs) = contact_shadows() {
        cam.insert(cs);
    }
}

/// GATE G4b — the camera's contact-shadow march, or `None` when
/// `VOXELFORGE_G4_CONTACT=off` (the BEFORE lever).
///
/// Returning `Option<ContactShadows>` rather than the component itself is what makes
/// the off-switch free: Bevy implements `Bundle` for `Option<C>`, so `None` inserts
/// nothing at all and the BEFORE plate is the genuine no-contact-shadow scene, not a
/// zero-length march that still costs a depth read and still nudges the pixels.
///
/// The three numbers are `look.rs`'s shipped v3 values (`CONTACT_SHADOW_LENGTH_V3`
/// / `_THICKNESS_V3` / `_STEPS_V3`), copied rather than imported: `look.rs` is
/// another lane's file and this bin does not compile it. They transfer because both
/// scenes are authored at ONE WORLD UNIT PER BLOCK, which is the only assumption the
/// numbers make — Bevy's own 0.3 default is metre-scale-character tuning and draws a
/// seam thinner than the voxel it is meant to be grounding.
fn contact_shadows() -> Option<ContactShadows> {
    if std::env::var("VOXELFORGE_G4_CONTACT").as_deref() == Ok("off") {
        println!("G4_CONTACT=off");
        return None;
    }
    println!("G4_CONTACT=on");
    Some(ContactShadows {
        // 0.85 blocks: long enough that the groove under a block survives the plate
        // downsample, short enough to stay inside one voxel so it reads as CONTACT
        // and not as a second cast shadow.
        length: 0.85,
        // Thin: the depth buffer is 2.5-D, so `thickness` is a guess at how solid a
        // fragment is. Too fat and the march self-occludes across the flat parquet
        // and greys the whole floor.
        thickness: 0.14,
        linear_steps: 24,
    })
}

/// Marks a cube that came out of [`VoxelGrid::flush`] — i.e. one that went through
/// the dedup. Anything drawn without it bypassed the grid, which is exactly the
/// thing `report_voxel_overlaps` is watching for.
#[derive(Component)]
pub struct VoxelCell;

/// The scene's single source of truth for grid-aligned cubes: one material per
/// integer cell, last write wins.
///
/// WHY THIS EXISTS. The hero kitchen is authored as overlapping boxes — the fridge
/// is filled from y=0 so it sits *on* the floor, the counter caps the cabinet run,
/// the two walls share a corner column. Written as direct `commands.spawn` calls
/// that meant 233 cells each held two opaque unit cubes at the same transform.
/// Coincident faces z-fight, and the winner is decided by draw order, which is not
/// a stable property: it shifts whenever anything else in the scene changes. That
/// is not theoretical — the teal tumbler's top block and the hero bowl's rim corner
/// both claimed (5,4,2), and merely adding the dust-mote material+mesh flipped the
/// order, so a whole teal voxel (~5.6k px) vanished from the locked golden while
/// the source still "clearly" spawned it. Reading the code never showed it.
///
/// Routing every cube through `put`/`fill` makes the winner the LAST write, which
/// is what the authoring order already meant, and makes it deterministic. It also
/// drops ~233 redundant draw calls.
#[derive(Default)]
pub struct VoxelGrid {
    cells: std::collections::HashMap<(i32, i32, i32), Handle<StandardMaterial>>,
    /// Cells that render but do NOT occlude the sun (see [`VoxelGrid::fill_nocast`]).
    nocast: std::collections::HashSet<(i32, i32, i32)>,
}

impl VoxelGrid {
    /// Claim one cell. A later `put` on the same cell replaces the earlier one.
    fn put(&mut self, x: i32, y: i32, z: i32, mat: &Handle<StandardMaterial>) {
        self.cells.insert((x, y, z), mat.clone());
        // Last write wins for the SHADOW flag too, not just the material: a plain
        // `fill` over a cell previously claimed by `fill_nocast` must get an ordinary
        // shadow-casting cube back, or the no-cast hole would outlive the pane that
        // asked for it. (`fill_nocast` re-inserts after its own `fill`, so it is
        // unaffected by this.)
        self.nocast.remove(&(x, y, z));
    }

    /// Claim the box region [x0..x1)×[y0..y1)×[z0..z1) for one material.
    fn fill(
        &mut self,
        mat: &Handle<StandardMaterial>,
        x0: i32,
        x1: i32,
        y0: i32,
        y1: i32,
        z0: i32,
        z1: i32,
    ) {
        for x in x0..x1 {
            for y in y0..y1 {
                for z in z0..z1 {
                    self.put(x, y, z, mat);
                }
            }
        }
    }

    /// Claim a box region whose cubes are VISIBLE but cast NO shadow.
    ///
    /// GATE G2 (`docs/VERDICT-flamingo-beauty-2026-08-16.md`): the beauty frame had
    /// "ไม่มีแถบ mullion ทาบพื้น/ผนังเลยสักเส้น" — not one bar of window light on the
    /// floor. The cause was not the sun and not the mullions: it was the emissive
    /// window PANE. The pane is authored as solid cubes at x 16..17 covering the whole
    /// opening (y 3..8, z 4..10) — i.e. a lid bolted over the window from OUTSIDE.
    /// Being an ordinary opaque mesh it also went into the shadow map, so the key
    /// light was stopped one block before it ever reached the mullion cross behind it.
    /// The room was lit entirely by ambient + the two bounce cards, which is exactly
    /// the flat, directionless read G2 scores.
    ///
    /// A pane is a light SOURCE, not an occluder — physically it is the sky seen
    /// through glass. Dropping it out of the shadow pass (rather than deleting it or
    /// making it transparent) keeps every pixel of the frame identical where the pane
    /// is directly visible — same emissive, same bloom seed, same G5 highlight — and
    /// changes only what the sun is allowed to reach.
    fn fill_nocast(
        &mut self,
        mat: &Handle<StandardMaterial>,
        x0: i32,
        x1: i32,
        y0: i32,
        y1: i32,
        z0: i32,
        z1: i32,
    ) {
        self.fill(mat, x0, x1, y0, y1, z0, z1);
        for x in x0..x1 {
            for y in y0..y1 {
                for z in z0..z1 {
                    self.nocast.insert((x, y, z));
                }
            }
        }
    }

    /// Spawn one cube per occupied cell and return how many. Sorted, because a
    /// `HashMap`'s iteration order is deliberately randomised per run and this
    /// whole type exists to stop render output depending on spawn order.
    fn flush(
        self,
        commands: &mut Commands,
        cube: &Handle<Mesh>,
        skins: &BlockSkins,
    ) -> usize {
        let mut cells: Vec<((i32, i32, i32), Handle<StandardMaterial>)> =
            self.cells.into_iter().collect();
        cells.sort_by_key(|(k, _)| *k);
        let mut nocast_placed = 0usize;
        let mut textured = 0usize;
        for ((x, y, z), mat) in &cells {
            // The per-face cube for this material's block kind, or the plain one.
            // Cells sharing a kind share the mesh handle, so this does not cost a
            // draw call per cell — it costs one per (kind, material) pair, the
            // same batching the single-cube version had.
            let mesh = match skins.mesh(mat) {
                Some(m) => {
                    textured += 1;
                    m
                }
                None => cube,
            };
            let mut e = commands.spawn((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(mat.clone()),
                Transform::from_xyz(*x as f32 + 0.5, *y as f32 + 0.5, *z as f32 + 0.5),
                VoxelCell,
            ));
            if self.nocast.contains(&(*x, *y, *z)) {
                e.insert(NotShadowCaster);
                nocast_placed += 1;
            }
        }
        // Printed, not silent: G2's whole fix is "N cells stopped occluding the sun",
        // and a render driver that greps 0 here knows the bake did not take without
        // having to eyeball a frame for it.
        println!("VOXEL_NOCAST={nocast_placed}");
        // Printed for the same reason as VOXEL_NOCAST: "the atlas is wired" is a
        // claim a runlog should be able to settle without eyeballing a frame. 0
        // here with a successful ATLAS line above means the palette binding, not
        // the loader, is what broke.
        println!("VOXEL_TEXTURED={textured}/{}", cells.len());
        cells.len()
    }
}

/// PROOF GATE (Flamingo, 2026-07-31) — no cell may hold two cubes of DIFFERENT
/// materials.
///
/// [`VoxelGrid`] makes that true by construction, so this gate is not the fix —
/// it is the guard that the fix stays in force. It reads the live World, not the
/// grid, so it also catches a cube spawned *around* the grid: the failure mode
/// that produced the missing teal voxel was code that looked correct in isolation.
///
/// It keys on the material handle rather than raw cube count on purpose. Two cubes
/// of the same material in one cell are wasteful but invisible; two of *different*
/// materials are the live landmine, because which one you see is draw order. Grading
/// only the second keeps the gate at "this can change the picture".
///
/// Runs once on the first Update (Startup's commands are applied by then) and prints
/// a line the render driver greps. `VOXEL_OVERLAPS=0` is the pass.
/// `VOXELFORGE_DUPPROBE=1` (see `shot_main.rs`) deliberately plants one conflicting
/// cube so this gate can be watched to go red — a gate nobody has seen fail is not
/// a gate.
pub fn report_voxel_overlaps(
    mut done: Local<bool>,
    q: Query<(&Transform, &MeshMaterial3d<StandardMaterial>)>,
    mut verdict: ResMut<GateVerdict>,
) {
    if *done {
        return;
    }
    *done = true;
    // Quantise to millimetres so float noise can't split one cell into two keys.
    let mut cells: std::collections::HashMap<(i64, i64, i64), Vec<AssetId<StandardMaterial>>> =
        std::collections::HashMap::new();
    for (t, mat) in &q {
        let p = t.translation;
        let key = (
            (p.x * 1000.0).round() as i64,
            (p.y * 1000.0).round() as i64,
            (p.z * 1000.0).round() as i64,
        );
        cells.entry(key).or_default().push(mat.id());
    }
    let mut conflicts: Vec<((i64, i64, i64), usize)> = cells
        .into_iter()
        .filter_map(|(k, mut ids)| {
            ids.sort();
            ids.dedup();
            (ids.len() > 1).then_some((k, ids.len()))
        })
        .collect();
    conflicts.sort();
    let n = conflicts.len();
    if n > 0 {
        verdict.failed = true;
        GATE_FAILED.store(true, std::sync::atomic::Ordering::Release);
    }
    println!("VOXEL_OVERLAPS={n}");
    for ((x, y, z), n) in conflicts.iter().take(32) {
        println!(
            "  OVERLAP at ({:.3}, {:.3}, {:.3}) {} distinct materials",
            *x as f64 / 1000.0,
            *y as f64 / 1000.0,
            *z as f64 / 1000.0,
            n
        );
    }
}

/// A hollow stepped bowl (voxel frustum) whose base sits at (cx, base_y, cz).
fn bowl(
    grid: &mut VoxelGrid,
    mat: &Handle<StandardMaterial>,
    rim_alt: Option<&Handle<StandardMaterial>>,
    cx: i32,
    base_y: i32,
    cz: i32,
) {
    // base 3×3 solid
    for x in cx - 1..=cx + 1 {
        for z in cz - 1..=cz + 1 {
            grid.put(x, base_y, z, mat);
        }
    }
    // rim ring 5×5, hollow centre (walls, height 1) => bowl silhouette. When rim_alt
    // is given, checker the ring tiles so the rim reads as bevelled voxel edges.
    let y = base_y + 1;
    for x in cx - 2..=cx + 2 {
        for z in cz - 2..=cz + 2 {
            let edge = x == cx - 2 || x == cx + 2 || z == cz - 2 || z == cz + 2;
            if edge {
                let m = match rim_alt {
                    Some(alt) if (x + z) % 2 != 0 => alt,
                    _ => mat,
                };
                grid.put(x, y, z, m);
            }
        }
    }
}

/// Sun direction (points FROM sky TO scene) from elevation/azimuth degrees.
fn sun_dir(elev_deg: f32, azim_deg: f32) -> Vec3 {
    let e = elev_deg.to_radians();
    let a = azim_deg.to_radians();
    // -Y downward; azimuth swings around Y. Normalised.
    Vec3::new(a.cos() * e.cos(), -e.sin(), a.sin() * e.cos()).normalize()
}

// Tiny tuple destructuring helpers so env arrays read cleanly above.
trait Tup3 {
    fn into_tuple3(self) -> (f32, f32, f32);
}
impl Tup3 for [f32; 3] {
    fn into_tuple3(self) -> (f32, f32, f32) {
        (self[0], self[1], self[2])
    }
}
trait Tup2 {
    fn into_tuple2(self) -> (f32, f32);
}
impl Tup2 for [f32; 2] {
    fn into_tuple2(self) -> (f32, f32) {
        (self[0], self[1])
    }
}
