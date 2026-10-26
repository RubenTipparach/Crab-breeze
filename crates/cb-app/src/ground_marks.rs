//! Ground marks and the impact of a mech's feet (openspec change `ground-marks`; owner,
//! 2026-10-24: "making mech steps really impact. Stamp mech footprint decals like we did with mining
//! mike. Kick up dirt, skates should leave dirt trails. Concrete would get concrete footprints").
//!
//! The mech's step reports each foot strike and each skating wheel as a `GroundContact`. Here each
//! one is turned into what it leaves: a footprint or a trail segment in the mark ring (the rules are
//! `cb_core::ground_marks`), the dirt or the grit thrown up (an `Fx` recipe chosen by the ground
//! under the foot), and, for the piloted mech, a jolt of the pilot's eye. Every mark is drawn by one
//! entity with one mesh and one material (`assets/shaders/ground_marks.wesl`) from one storage
//! buffer: one draw call. Craters dug by blasts get a scorch mark too. Soil prints and skate
//! grooves clear the grass under them for their life (`PrintCrush`, read by `grass`).
//!
//! A port of Mining Mike's GroundDents (`godot-sandbox`, branch `origin/claude/long-haul-playtest`,
//! commit 74e1412c: `scripts/fx/ground_dents.gd`, `scenes/fx/ground_dents.tscn`,
//! `shaders/ground_dent.gdshader`): one batch, a multiply over the ground, drawn before anything
//! see-through. `CB_NO_GROUND_MARKS=1` turns the marks off (the effects and the jolt stay), so the
//! benchmark can compare.

use crate::fx::Fx;
use crate::pilot::{PilotCamera, PilotSim};
use crate::sim::{Data, PresentSet};
use crate::state::GameApp;
use crate::world::{Live, Plan};
use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, MeshVertexBufferLayoutRef};
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, PrimitiveTopology, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError};
use bevy::render::storage::ShaderBuffer;
use bevy::shader::ShaderRef;
use cb_core::ground_marks::{GroundMarksConfig, Jolt, MarkRing, ROW_FLOATS, TrailLayer, grid_mesh, mark_surface};
use cb_core::mech::forward;
use cb_core::pilot::PilotMode;
use cb_core::world::{Scene, Surface};

/// The marks shader, loaded from `assets/`.
const SHADER: &str = "shaders/ground_marks.wesl";

/// A wheel not reported for this long has left the ground: its next contact starts a new line.
const WHEEL_FORGET_S: f32 = 0.3;

/// A mech's foot or wheel touching dry ground this fixed step (written by `mech::step_mech`).
#[derive(Message, Clone, Copy, Debug)]
pub struct GroundContact {
    /// The mech's id (0 is the player's).
    pub mech: u32,
    /// The mech the pilot drives.
    pub piloted: bool,
    /// What touched.
    pub contact: Contact,
}

/// What touched the ground.
#[derive(Clone, Copy, Debug)]
pub enum Contact {
    /// A foot starts its stance (`cb_core::mech::foot_strikes`).
    Step {
        /// 0 left, 1 right.
        side: usize,
        /// The ground under the foot's rest ankle (world metres).
        foot: Vec3,
        /// The mech's heading, radians (0 faces -z).
        heading_rad: f32,
        /// How far ahead of its rest ankle the foot comes down (`stance_lead_m`), metres.
        lead_m: f32,
        /// The mech's ground speed, m/s.
        speed_m_s: f32,
        /// Both feet coming down from the air.
        landed: bool,
    },
    /// A skate wheel rolling on the ground.
    Wheel {
        /// 0 left, 1 right.
        wheel: u8,
        /// The wheel (world metres).
        at: Vec3,
        /// The mech's velocity (x, z), m/s.
        vel: Vec2,
    },
}

/// The pilot's eye offset this frame from the piloted mech's foot strikes: a position, never a
/// direction (`pilot::place_camera` adds it to the seated eye).
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct ViewJolt(pub Vec3);

/// Where the live soil prints and skate grooves crush the grass, and a version that moves when the
/// set changes (the grass repacks the tiles they touch only then).
#[derive(Resource, Default, Clone, Debug)]
pub struct PrintCrush {
    /// Moves on when a crushing mark is laid or fades out.
    pub version: u64,
    /// Every live crush rectangle.
    pub rects: Vec<cb_core::grass::CrushRect>,
}

/// `MarksLook` in `ground_marks.wesl`, field for field.
#[derive(Clone, Copy, Debug, Default, ShaderType)]
pub struct MarksUniform {
    reach: Vec4,
    soil: Vec4,
    paving: Vec4,
    crack: Vec4,
}

impl MarksUniform {
    /// The uniform for `cfg`.
    pub fn new(cfg: &GroundMarksConfig) -> Self {
        let l = &cfg.look;
        Self {
            reach: Vec4::new(cfg.draw_within_m, cfg.lift_m, l.pull_per_m, l.ambient),
            soil: Vec3::from_array(l.soil_tint).extend(l.pit_dark),
            paving: Vec3::from_array(l.crack_colour).extend(l.dust_brighten),
            crack: Vec4::new(l.crack_width_m, l.crack_reach, l.scuff, l.slab_m),
        }
    }
}

/// The one marks material (CLAUDE.md 6.1): a multiply over the ground, drawn first among the
/// see-through things.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct GroundMarksMaterial {
    /// The look.
    #[uniform(0)]
    pub look: MarksUniform,
    /// Every slot of the ring, `ROW_FLOATS / 4` rows each.
    #[storage(1, read_only)]
    pub marks: Handle<ShaderBuffer>,
}

impl Material for GroundMarksMaterial {
    fn vertex_shader() -> ShaderRef {
        SHADER.into()
    }

    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }

    // Multiplied over what is drawn: 1 leaves the ground as it is (Mining Mike's `blend_mul`).
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Multiply
    }

    // Sorted before every other see-through thing (smoke, dust, water rings), so they draw over it
    // (Mining Mike's `render_priority = -20`). The bias only orders the transparent pass.
    fn depth_bias(&self) -> f32 {
        -1.0e9
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

/// The ring, the trails in progress, and the buffer the GPU reads.
#[derive(Resource)]
struct Marks {
    ring: MarkRing,
    trails: TrailLayer,
    /// The ring as rows, rewritten while any mark lives.
    rows: Vec<[f32; 4]>,
    buffer: Option<Handle<ShaderBuffer>>,
    /// False under `CB_NO_GROUND_MARKS=1`: no mark is laid or drawn.
    enabled: bool,
    /// Blasts already scorched.
    blasts_seen: usize,
    /// A mark lived last frame (the buffer then needs one more write to clear it).
    any_last: bool,
    /// The piloted mech's jolts.
    jolt: Jolt,
    /// Footprints laid so far (logged every tenth, for checking a take laid them).
    laid: u64,
    /// The marks' material: touched after each upload so its bind group is rebuilt on the new buffer.
    material: Option<Handle<GroundMarksMaterial>>,
}

/// Lays and draws the marks; plays the dirt and grit; jolts the pilot.
pub struct GroundMarksPlugin {
    /// Draw and lay marks (false under `CB_NO_GROUND_MARKS=1`).
    pub enabled: bool,
}

impl Plugin for GroundMarksPlugin {
    fn build(&self, app: &mut App) {
        let cap = app.world().resource::<Data>().0.ground_marks.max_marks as usize;
        app.add_message::<GroundContact>()
            .init_resource::<ViewJolt>()
            .init_resource::<PrintCrush>()
            .insert_resource(Marks {
                ring: MarkRing::new(cap),
                trails: TrailLayer::default(),
                rows: vec![[0.0; 4]; cap * ROW_FLOATS / 4],
                buffer: None,
                enabled: self.enabled,
                blasts_seen: 0,
                any_last: false,
                jolt: Jolt::default(),
                laid: 0,
                material: None,
            })
            .add_game_systems(
                Update,
                (take_contacts, scorch_craters, age_and_upload).chain().in_set(PresentSet::Pose),
            );
        if self.enabled {
            app.add_plugins(MaterialPlugin::<GroundMarksMaterial>::default())
                .add_world_spawn(setup);
        }
    }
}

fn setup(
    mut commands: Commands,
    data: Res<Data>,
    mut marks: ResMut<Marks>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
    mut materials: ResMut<Assets<GroundMarksMaterial>>,
) {
    let cfg = &data.0.ground_marks;
    let cap = marks.ring.capacity();
    let (vertices, indices) = grid_mesh(cap);
    let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
        .with_inserted_indices(Indices::U32(indices));
    let buffer = buffers.add(ShaderBuffer::new(marks.rows.clone(), RenderAssetUsages::RENDER_WORLD));
    let material = materials.add(GroundMarksMaterial {
        look: MarksUniform::new(cfg),
        marks: buffer.clone(),
    });
    info!(
        "ground marks: {cap} slots, {} KiB buffer, {} vertices in one draw",
        cap * ROW_FLOATS * 4 / 1024,
        cap * cb_core::ground_marks::GRID * cb_core::ground_marks::GRID
    );
    let material_handle = material.clone();
    commands.spawn((
        Name::new("ground marks"),
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(material),
        Transform::IDENTITY,
        // The marks spread over the whole island; the shader drops the far ones.
        NoFrustumCulling,
        NotShadowCaster,
    ));
    marks.buffer = Some(buffer);
    marks.material = Some(material_handle);
}

/// The fx recipe a foot strike plays on `surface`.
fn strike_recipe(surface: Surface, landed: bool) -> &'static str {
    match (surface, landed) {
        (Surface::Dirt, false) => "foot_strike_dirt",
        (Surface::Concrete, false) => "foot_strike_concrete",
        (Surface::Dirt, true) => "landing_dirt",
        (Surface::Concrete, true) => "landing_concrete",
    }
}

/// The fx recipe a fast wheel plays on `surface`.
fn wheel_recipe(surface: Surface) -> &'static str {
    match surface {
        Surface::Dirt => "skate_dirt",
        Surface::Concrete => "skate_grit",
    }
}

/// The unit vector toward foot `side` (0 left, 1 right) of a mech heading `heading_rad`.
fn toward_foot(heading_rad: f32, side: usize) -> Vec3 {
    let left = forward(heading_rad + std::f32::consts::FRAC_PI_2);
    if side == 0 { left } else { -left }
}

/// Turns this step's contacts into marks, effects and the pilot's jolt.
#[allow(clippy::too_many_arguments)]
fn take_contacts(
    mut contacts: MessageReader<GroundContact>,
    mut marks: ResMut<Marks>,
    mut fx: MessageWriter<Fx>,
    data: Res<Data>,
    plan: Res<Plan>,
    live: Res<Live>,
    pilot: Res<PilotSim>,
    camera: Single<&Transform, With<PilotCamera>>,
    time: Res<Time>,
) {
    let cfg = &data.0.ground_marks;
    let eye = camera.translation;
    let now = time.elapsed_secs();
    let seated = pilot.curr.mode == PilotMode::Seated;
    let scene = Scene {
        plan: &plan,
        dug: &live.dug,
        trees: None,
        structures: None,
        platforms: &[],
    };
    let marks = &mut *marks;
    for c in contacts.read() {
        match c.contact {
            Contact::Step {
                side,
                foot,
                heading_rad,
                lead_m,
                speed_m_s,
                landed,
            } => {
                let sole = foot + forward(heading_rad) * (lead_m + cfg.sole_ahead_m);
                let Some(surface) = mark_surface(&plan, &scene, Vec2::new(sole.x, sole.z)) else {
                    continue;
                };
                // Faster strides throw more.
                let (density, scale) = if landed {
                    (1.5, 1.2)
                } else {
                    ((1.0 + speed_m_s / 15.0).clamp(1.0, 2.5), 1.0)
                };
                let at = Vec3::new(sole.x, scene_height(&scene, sole), sole.z);
                fx.write(Fx::new(strike_recipe(surface, landed), at).dense(density).scaled(scale));
                if c.piloted && seated {
                    marks
                        .jolt
                        .kick(cfg.jolt.amplitude_m(speed_m_s, landed), toward_foot(heading_rad, side));
                }
                if marks.enabled && sole.distance(eye) < cfg.lay_within_m {
                    let m = cfg.footprint(surface, sole, heading_rad, landed).on_ground(&scene);
                    marks.ring.add(m);
                    marks.laid += 1;
                    if marks.laid % 10 == 1 {
                        info!(
                            "ground marks: {} laid, the last a {surface:?} print at ({:.0}, {:.0})",
                            marks.laid, sole.x, sole.z
                        );
                    }
                }
            }
            Contact::Wheel { wheel, at, vel } => {
                let xz = Vec2::new(at.x, at.z);
                let Some(surface) = mark_surface(&plan, &scene, xz) else {
                    continue;
                };
                let speed = vel.length();
                if speed > cfg.skate_fx_min_m_s {
                    let back = -Vec3::new(vel.x, 0.0, vel.y).normalize_or(Vec3::Z);
                    fx.write(Fx::new(wheel_recipe(surface), at).toward(back + Vec3::Y * 0.3));
                }
                if !marks.enabled || speed < cfg.trail_min_speed_m_s || at.distance(eye) > cfg.lay_within_m {
                    continue;
                }
                let Some((a, b)) = marks.trails.feed((c.mech, wheel), xz, now, cfg.trail_segment_m, cfg.trail_break_m) else {
                    continue;
                };
                let a3 = Vec3::new(a.x, 0.0, a.y);
                let b3 = Vec3::new(b.x, 0.0, b.y);
                let m = cfg.trail(surface, a3, b3).on_ground(&scene);
                marks.ring.add(m);
            }
        }
    }
    marks.trails.forget(now, WHEEL_FORGET_S);
}

fn scene_height(scene: &Scene, p: Vec3) -> f32 {
    use cb_core::world::Ground;
    scene.height_at(p.x, p.z)
}

/// A scorch mark in every crater dug since the last frame.
fn scorch_craters(mut marks: ResMut<Marks>, data: Res<Data>, plan: Res<Plan>, live: Res<Live>) {
    let blasts = &live.dug.blasts;
    let fresh = blasts.len().saturating_sub(marks.blasts_seen);
    marks.blasts_seen = blasts.len();
    if !marks.enabled || fresh == 0 {
        return;
    }
    let cfg = &data.0.ground_marks;
    let craters = &data.0.world.craters;
    let scene = Scene {
        plan: &plan,
        dug: &live.dug,
        trees: None,
        structures: None,
        platforms: &[],
    };
    for b in blasts.iter().skip(blasts.len() - fresh) {
        let r = craters.radius_m_per_cbrt_mj * b.energy_mj.max(0.0).cbrt();
        let Some(surface) = mark_surface(&plan, &scene, b.centre) else {
            continue;
        };
        // On soil the Material Maker scorch decal covers the crater (openspec change `blast-scorch`).
        if surface == cb_core::world::Surface::Dirt {
            continue;
        }
        // A turn from the blast's place, so neighbouring craters differ.
        let yaw = (b.centre.x * 0.37 + b.centre.y * 0.61).sin() * std::f32::consts::PI;
        let m = cfg
            .crater(surface, Vec3::new(b.centre.x, 0.0, b.centre.y), r, yaw)
            .on_ground(&scene);
        marks.ring.add(m);
    }
}

/// Ages the marks and the jolt, writes the GPU rows while any mark lives, and publishes the jolt
/// and the grass the soil marks crush.
#[allow(clippy::too_many_arguments)]
fn age_and_upload(
    mut marks: ResMut<Marks>,
    data: Res<Data>,
    time: Res<Time>,
    pilot: Res<PilotSim>,
    mut jolt: ResMut<ViewJolt>,
    mut crush: ResMut<PrintCrush>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
    mut materials: ResMut<Assets<GroundMarksMaterial>>,
) {
    let cfg = &data.0.ground_marks;
    let dt = time.delta_secs();
    let marks = &mut *marks;
    marks.jolt.advance(dt, &cfg.jolt);
    // Only seated: on foot the pilot's own walk moves the eye.
    jolt.0 = if pilot.curr.mode == PilotMode::Seated {
        marks.jolt.offset(&cfg.jolt)
    } else {
        Vec3::ZERO
    };
    if !marks.enabled {
        return;
    }
    let any = marks.ring.advance(dt);
    if marks.ring.crush_version() != crush.version {
        crush.version = marks.ring.crush_version();
        crush.rects = marks.ring.crushes(cfg.crush_margin_m);
    }
    if !(any || marks.any_last) {
        return;
    }
    marks.any_last = any;
    marks.ring.pack_into(&mut marks.rows);
    if let Some(mut b) = marks.buffer.as_ref().and_then(|h| buffers.get_mut(h)) {
        // The same size every time, so the GPU buffer is written in place and the bind group stays.
        *b = ShaderBuffer::new(marks.rows.clone(), RenderAssetUsages::RENDER_WORLD);
    }
    // A replaced buffer is a new GPU buffer: touch the material so its bind group points at it.
    if let Some(h) = marks.material.as_ref() {
        materials.get_mut(h);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shader_check::{compile, struct_size, with_bevy};

    #[test]
    fn the_marks_shader_compiles_against_bevy_pbr_and_its_uniform_and_rows_match() {
        let Some(resolver) = with_bevy() else {
            eprintln!("skipped: Bevy 0.20's sources are not in the Cargo registry");
            return;
        };
        let base = [("VERTEX_POSITIONS", true), ("CLUSTERED_FORWARD_STORAGE_BUFFER", true)];
        for (msaa, prepass, fog) in [(false, true, true), (true, false, true), (false, false, false)] {
            let mut flags = base.to_vec();
            flags.extend([("MULTISAMPLED", msaa), ("DEPTH_PREPASS", prepass), ("DISTANCE_FOG", fog)]);
            let module = compile(&resolver, "ground_marks", &flags);
            assert_eq!(
                u64::from(struct_size(&module, "MarksLook")),
                MarksUniform::min_size().get(),
                "the Rust uniform must be the WESL struct's size"
            );
            assert_eq!(
                struct_size(&module, "Mark") as usize,
                ROW_FLOATS * 4,
                "a mark's rows must be the WESL struct's size"
            );
        }
    }

    #[test]
    fn the_ground_picks_the_effect() {
        assert_eq!(strike_recipe(Surface::Dirt, false), "foot_strike_dirt");
        assert_eq!(strike_recipe(Surface::Concrete, true), "landing_concrete");
        assert_eq!(wheel_recipe(Surface::Concrete), "skate_grit");
        let fx = cb_core::fx::RECIPES;
        for r in [
            strike_recipe(Surface::Dirt, false),
            strike_recipe(Surface::Dirt, true),
            strike_recipe(Surface::Concrete, false),
            strike_recipe(Surface::Concrete, true),
            wheel_recipe(Surface::Dirt),
            wheel_recipe(Surface::Concrete),
        ] {
            assert!(fx.contains(&r), "{r} is a recipe the data must carry");
        }
    }

    #[test]
    fn the_left_foot_is_on_the_mechs_left() {
        // Heading 0 faces -z; the mech's left is then -x.
        assert!(toward_foot(0.0, 0).distance(Vec3::NEG_X) < 1e-6);
        assert!(toward_foot(0.0, 1).distance(Vec3::X) < 1e-6);
    }
}
