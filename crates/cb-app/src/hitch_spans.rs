//! Every system's time in every frame, for the `hitch:` lines (a measurement instrument, CLAUDE.md section 4). Built
//! only with the `hitch_spans` feature, which turns on Bevy's `trace` spans: a system, main world or render world,
//! runs inside a span named `system` carrying the system's name. This layer times each one and adds it to
//! `perf_hud`'s frame tally, so a frame over 16.7 ms names what it spent its time on even when no one system crossed
//! the 4 ms `slow` line. Unlike `trace_chrome` it writes nothing but the tally (about 1 us a span), so the build plays
//! at close to the game's speed; its frame rate is still not a result, only its ranking of causes.

use bevy::ecs::change_detection::DetectChanges;
use bevy::log::BoxedLayer;
use bevy::log::tracing::span::{Attributes, Id};
use bevy::log::tracing::{Subscriber, field};
use bevy::log::tracing_subscriber::Layer;
use bevy::log::tracing_subscriber::layer::Context;
use bevy::log::tracing_subscriber::registry::LookupSpan;
use bevy::prelude::{App, IntoScheduleConfigs};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

/// Counts, each frame, the entities whose `Transform` changed or that were spawned, by the first word of their
/// `Name` (an unnamed one by its parent's), into the frame's counts as `moved:<name>` and `new:<name>` (50 or more):
/// what the transform propagation of a slow frame had to walk.
pub struct HitchCountsPlugin;

impl bevy::prelude::Plugin for HitchCountsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            bevy::prelude::PostUpdate,
            count_changes.before(bevy::transform::TransformSystems::Propagate),
        )
        .add_systems(bevy::prelude::Last, count_assets);
    }
}

fn count_changes(
    q: bevy::prelude::Query<(
        bevy::prelude::Ref<bevy::prelude::Transform>,
        Option<&bevy::prelude::Name>,
        Option<&bevy::prelude::ChildOf>,
    )>,
    names: bevy::prelude::Query<&bevy::prelude::Name>,
) {
    let mut moved: HashMap<String, u32> = HashMap::new();
    let mut new: HashMap<String, u32> = HashMap::new();
    for (t, name, parent) in &q {
        if !t.is_changed() {
            continue;
        }
        let name = name.or_else(|| parent.and_then(|p| names.get(p.parent()).ok()));
        let word = name
            .map_or("unnamed", |n| n.as_str().split([' ', ':']).next().unwrap_or("unnamed"))
            .to_owned();
        let map = if t.is_added() { &mut new } else { &mut moved };
        *map.entry(word).or_default() += 1;
    }
    for (prefix, map) in [("moved:", moved), ("new:", new)] {
        for (k, n) in map {
            if n >= 50 {
                crate::perf_hud::count(intern(&format!("{prefix}{k}")), n);
            }
        }
    }
}

/// Counts the mesh and image assets added, changed and removed this frame (`mesh+`, `mesh~`, `mesh-`, `image+`,
/// ...): what the render world has to upload or free next.
fn count_assets(
    mut meshes: bevy::prelude::MessageReader<bevy::prelude::AssetEvent<bevy::prelude::Mesh>>,
    mut images: bevy::prelude::MessageReader<bevy::prelude::AssetEvent<bevy::prelude::Image>>,
) {
    use bevy::prelude::AssetEvent;
    for e in meshes.read() {
        match e {
            AssetEvent::Added { .. } => crate::perf_hud::count("mesh+", 1),
            AssetEvent::Modified { .. } => crate::perf_hud::count("mesh~", 1),
            AssetEvent::Removed { .. } => crate::perf_hud::count("mesh-", 1),
            _ => {}
        }
    }
    for e in images.read() {
        match e {
            AssetEvent::Added { .. } => crate::perf_hud::count("image+", 1),
            AssetEvent::Modified { .. } => crate::perf_hud::count("image~", 1),
            AssetEvent::Removed { .. } => crate::perf_hud::count("image-", 1),
            _ => {}
        }
    }
}

/// The layer `LogPlugin::custom_layer` adds.
pub fn layer(_app: &mut App) -> Option<BoxedLayer> {
    Some(Box::new(SystemTimes))
}

/// Times `system` spans.
struct SystemTimes;

/// A system span's name, kept once for the life of the game (there are a few hundred systems).
struct Named(&'static str);
/// When the span was last entered.
struct Entered(Instant);

static NAMES: Mutex<Option<HashMap<String, &'static str>>> = Mutex::new(None);

fn intern(s: &str) -> &'static str {
    let Ok(mut names) = NAMES.lock() else { return "?" };
    let names = names.get_or_insert_with(HashMap::new);
    if let Some(n) = names.get(s) {
        return n;
    }
    let n: &'static str = Box::leak(s.to_owned().into_boxed_str());
    names.insert(s.to_owned(), n);
    n
}

/// Reads a span's `name` field.
#[derive(Default)]
struct NameField(Option<String>);

impl field::Visit for NameField {
    fn record_str(&mut self, f: &field::Field, value: &str) {
        if f.name() == "name" {
            self.0 = Some(value.to_owned());
        }
    }
    fn record_debug(&mut self, f: &field::Field, value: &dyn std::fmt::Debug) {
        if f.name() == "name" {
            self.0 = Some(format!("{value:?}").trim_matches('"').to_owned());
        }
    }
}

impl<S: Subscriber + for<'a> LookupSpan<'a>> Layer<S> for SystemTimes {
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        if attrs.metadata().name() != "system" {
            return;
        }
        let mut v = NameField::default();
        attrs.record(&mut v);
        if let (Some(name), Some(span)) = (v.0, ctx.span(id)) {
            // The module path says little; the system's own name and its last path part are enough.
            let short = name
                .rsplit("::")
                .take(2)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join("::");
            span.extensions_mut().insert(Named(intern(&short)));
        }
    }

    fn on_enter(&self, id: &Id, ctx: Context<'_, S>) {
        if let Some(span) = ctx.span(id) {
            let mut ext = span.extensions_mut();
            if ext.get_mut::<Named>().is_some() {
                ext.replace(Entered(Instant::now()));
            }
        }
    }

    fn on_exit(&self, id: &Id, ctx: Context<'_, S>) {
        if let Some(span) = ctx.span(id) {
            let ext = span.extensions();
            if let (Some(n), Some(e)) = (ext.get::<Named>(), ext.get::<Entered>()) {
                crate::perf_hud::tally(n.0, e.0.elapsed().as_secs_f32() * 1000.0);
            }
        }
    }
}
