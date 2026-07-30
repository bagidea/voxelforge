# Bevy 0.19 / bevy_egui 0.41 API Cheatsheet for Voxelforge

> Verified against the actual versions in `Cargo.lock`:
> - `bevy` **0.19.0**
> - `bevy_egui` **0.41.1**
> - `egui` **0.35.0**
> - `epaint` **0.35.0**
>
> Do not copy-paste old examples from Bevy 0.14/0.15 or older `bevy_egui` versions.

---

## 1. Events / Messages

In Bevy 0.19 the old queued event API was **renamed to Messages**. The name `Event` now means observer/trigger events.

| What you used to write | What you must write in Bevy 0.19 |
| ---------------------- | -------------------------------- |
| `#[derive(Event)]` for queued events | `#[derive(Message)]` |
| `EventReader<T>` | `MessageReader<T>` or `PopulatedMessageReader<T>` |
| `EventWriter<T>` | `MessageWriter<T>` |
| `app.add_event::<T>()` | `app.add_message::<T>()` |
| `writer.send(T)` | `writer.write(T)` |
| `writer.send_default()` | `writer.write_default()` |
| `reader.read()` | `reader.read()` (same name) |
| Run-if helper for queued events | `on_message::<T>()` |

### 1a. Message API (queued, schedule-based)

```rust
use bevy::prelude::*;

#[derive(Message)]
struct DamageDealt {
    target: Entity,
    amount: u32,
}

fn plugin(app: &mut App) {
    app.add_message::<DamageDealt>();
}

fn deal_damage(mut writer: MessageWriter<DamageDealt>) {
    writer.write(DamageDealt {
        target: Entity::PLACEHOLDER,
        amount: 10,
    });
}

// Runs every frame; does nothing if the queue is empty.
fn read_damage(mut reader: MessageReader<DamageDealt>) {
    for msg in reader.read() {
        // msg: &DamageDealt
    }
}

// Skips the system entirely when the queue is empty.
fn read_damage_skip(mut reader: PopulatedMessageReader<DamageDealt>) {
    for msg in reader.read() {
        // ...
    }
}
```

### 1b. Observer / Trigger API (immediate, push-based)

Use this when you want reactions to happen **right now**, not at the next schedule point.

```rust
use bevy::prelude::*;

#[derive(Event)]
struct Explode;

fn plugin(app: &mut App) {
    // Global observer.
    app.add_observer(|_event: On<Explode>, mut commands: Commands| {
        // react immediately
    });
}

fn trigger_explosion(mut commands: Commands) {
    commands.trigger(Explode);
}

// Entity-targeted observer.
fn arm_bomb(mut commands: Commands, bombs: Query<Entity, With<Bomb>>) {
    for bomb in &bombs {
        commands.entity(bomb).observe(|_event: On<Explode>| {
            // only runs when this specific entity is triggered
        });
    }
}
```

References:
- [`bevy::ecs::message`](https://docs.rs/bevy/0.19.0/bevy/ecs/message/index.html)
- [`bevy::ecs::event`](https://docs.rs/bevy/0.19.0/bevy/ecs/event/index.html)
- [`bevy::ecs::observer`](https://docs.rs/bevy/0.19.0/bevy/ecs/observer/index.html)
- Bevy example: [`ecs/message.rs`](https://github.com/bevyengine/bevy/blob/v0.19.0/examples/ecs/message.rs)

---

## 2. Egui Context (`bevy_egui` 0.41)

`EguiContexts` getters now return `Result`. UI systems should return `Result` (the `bevy_ecs::error::Result` alias) and propagate with `?`, or match the error explicitly.

| Old (<= 0.29 style) | Bevy 0.19 / bevy_egui 0.41 |
| ------------------- | -------------------------- |
| `fn ui(mut contexts: EguiContexts)` | `fn ui(mut contexts: EguiContexts) -> Result` |
| `contexts.ctx_mut()` returns `&mut Context` | returns `Result<&mut Context, QuerySingleError>` |
| `contexts.ctx()` returns `&Context` | returns `Result<&Context, QuerySingleError>` |
| `contexts.ctx_for_window_mut(window)` | `contexts.ctx_for_entity_mut(entity)` |
| UI in `Update` | UI in `EguiPrimaryContextPass` (multi-pass default) |

### Correct minimal example

```rust
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiPrimaryContextPass};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin::default())
        .add_systems(Startup, setup_camera)
        .add_systems(EguiPrimaryContextPass, debug_ui)
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn debug_ui(mut contexts: EguiContexts) -> Result {
    egui::Window::new("Debug").show(contexts.ctx_mut()?, |ui| {
        ui.label("Hello Voxelforge");
    });
    Ok(())
}
```

### Explicit match style (also valid)

```rust
fn debug_ui(mut contexts: EguiContexts) {
    let Ok(ctx) = contexts.ctx_mut() else { return };
    egui::Window::new("Debug").show(ctx, |ui| {
        ui.label("Hello Voxelforge");
    });
}
```

### Per-entity context

```rust
fn per_camera_ui(
    mut contexts: EguiContexts,
    cameras: Query<Entity, With<CustomEguiCamera>>,
) -> Result {
    for camera in &cameras {
        let ctx = contexts.ctx_for_entity_mut(camera)?;
        egui::Window::new("Per-camera").show(ctx, |ui| {
            ui.label("...");
        });
    }
    Ok(())
}
```

References:
- [`bevy_egui::EguiContexts`](https://docs.rs/bevy_egui/0.41.1/bevy_egui/struct.EguiContexts.html)
- [`bevy_egui` simple example](https://github.com/vladbat00/bevy_egui/blob/v0.41.1/examples/simple.rs)

---

## 3. Egui `Margin` / `Frame`

`egui::Margin` (from `epaint`) stores **public `i8` fields**. Direct struct literals must use integers, but builder methods that accept `impl Into<Margin>` (such as `Frame::inner_margin`) also accept `f32` thanks to `From<f32> for Margin`.

| Old assumption | Reality in egui 0.35 |
| -------------- | -------------------- |
| `Margin { left: 8.0, .. }` | Fields are `i8`; use `8` |
| `Margin::symmetric(8.0, 4.0)` | `Margin::symmetric(8, 4)` (or `8.0, 4.0`; rounds to `i8`) |
| `Margin::same(8.0)` | `Margin::same(8)` (or `8.0`; rounds to `i8`) |
| Float margins everywhere | Use `MarginF32` if you really need fractional values |

### Correct snippets

```rust
use bevy_egui::egui;

// All of these are valid because Frame::inner_margin accepts impl Into<Margin>.
let frame = egui::Frame::default()
    .inner_margin(8)                         // From<i8>
    .inner_margin(8.0)                       // From<f32>: rounds to i8 via v.round()
    .inner_margin(egui::Margin::same(8))
    .inner_margin(egui::Margin::symmetric(8, 4))
    .outer_margin(egui::Margin { left: 4, right: 4, top: 2, bottom: 2 });
```

`Frame` fields are public, but the recommended style is the builder methods above. Float literals compile, but `From<f32>` rounds and then stores an `i8`, so fractional margins are lost. If you need true fractional values, use `MarginF32` instead of `Margin`.

References:
- [`epaint::Margin`](https://docs.rs/epaint/0.35.0/epaint/struct.Margin.html)
- [`egui::Frame`](https://docs.rs/egui/0.35.0/egui/containers/frame/struct.Frame.html)

---

## 4. System Ordering & Run Conditions

`App::add_system(system)` is gone. You always add systems to a schedule label.

| Old | Bevy 0.19 |
| --- | --------- |
| `app.add_system(system)` | `app.add_systems(Update, system)` |
| `app.add_systems(system).in_set(Set)` | `app.add_systems(Update, system.in_set(Set))` |
| `.before(Set)` / `.after(Set)` | same names, still work on `IntoScheduleConfigs` |
| `.run_if(condition)` | same name; condition is `impl SystemCondition<M>` |
| `.with_run_criteria(...)` | removed; use `.run_if` or `.distributive_run_if` |

### Correct examples

```rust
use bevy::prelude::*;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
enum CombatSystems {
    Input,
    Damage,
    Cleanup,
}

fn plugin(app: &mut App) {
    app.add_systems(Update, (
        read_input.in_set(CombatSystems::Input),
        apply_damage
            .in_set(CombatSystems::Damage)
            .after(CombatSystems::Input),
        cleanup.after(CombatSystems::Damage),
    ));

    // Or chain the whole set.
    app.configure_sets(Update, (
        CombatSystems::Input,
        CombatSystems::Damage,
        CombatSystems::Cleanup,
    ).chain());

    // Run condition: only run while a resource exists.
    app.add_systems(Update, some_system.run_if(resource_exists::<GameState>));
}
```

### Run conditions for messages

```rust
app.add_systems(Update, on_damage_taken.run_if(on_message::<DamageDealt>));
```

`on_message::<M>()` consumes the messages it sees, so it is meant for systems that do not also read the queue with `MessageReader<M>`.

References:
- [`IntoScheduleConfigs`](https://docs.rs/bevy/0.19.0/bevy/ecs/schedule/trait.IntoScheduleConfigs.html)
- [`SystemSet`](https://docs.rs/bevy/0.19.0/bevy/ecs/schedule/trait.SystemSet.html)
- [`common_conditions`](https://docs.rs/bevy/0.19.0/bevy/ecs/schedule/common_conditions/index.html)

---

## 5. Query Conflict `B0001`

Error text (from source):

```text
error[B0001]: ... accesses component(s) ... in a way that conflicts with a previous system parameter.
Consider using `Without<T>` to create disjoint Queries or merging conflicting Queries into a `ParamSet`.
```

### Bad

```rust
fn broken(
    mut players: Query<&mut Transform, With<Player>>,
    mut enemies: Query<&mut Transform>, // overlaps with players
) {
    // ...
}
```

### Fix A: disjoint filters

```rust
fn fixed(
    mut players: Query<&mut Transform, With<Player>>,
    mut enemies: Query<&mut Transform, Without<Player>>,
) {
    // ...
}
```

### Fix B: `ParamSet`

Use this when you need to iterate the same component in two different ways.

```rust
fn fixed(mut params: ParamSet<(
    Query<&mut Transform, With<Player>>,
    Query<&mut Transform, Without<Player>>,
)>) {
    for mut t in params.p0().iter_mut() {
        // ...
    }
    for mut t in params.p1().iter_mut() {
        // ...
    }
}
```

References:
- [Bevy error B0001](https://bevy.org/learn/errors/b0001)
- [`ParamSet`](https://docs.rs/bevy/0.19.0/bevy/ecs/system/struct.ParamSet.html)

---

## Top 5 Mistakes This Team Keeps Hitting

1. **Using the old event names.** `EventReader`, `EventWriter`, `add_event`, and `#[derive(Event)]` for queued events are gone. Use `MessageReader`, `MessageWriter`, `add_message`, and `#[derive(Message)]`.
2. **Calling `EguiContexts` getters as if they return the context directly.** They return `Result`. Return `Result` from the system and use `?`, or match/`let Ok(...)`.
3. **Putting egui systems in `Update`.** `bevy_egui` 0.41 defaults to multi-pass; add UI systems to `EguiPrimaryContextPass`.
4. **Assuming floats are rejected by `egui::Margin`.** `Margin` stores `i8`, but `Frame::inner_margin` accepts `impl Into<Margin>` and `From<f32> for Margin` exists, so `inner_margin(8.0)` compiles. Fractional values are rounded to `i8`; use `MarginF32` when you need real fractional margins.
5. **Two mutable queries over the same component without disjoint filters or `ParamSet`.** Bevy panics with `B0001`. Add `With<T>`/`Without<T>` filters, or wrap the queries in a `ParamSet`.

---

## Quick Reference Links

| Topic | URL |
| ----- | --- |
| Bevy 0.19 docs | https://docs.rs/bevy/0.19.0/bevy/ |
| `bevy_egui` 0.41 docs | https://docs.rs/bevy_egui/0.41.1/bevy_egui/ |
| `egui` 0.35 docs | https://docs.rs/egui/0.35.0/egui/ |
| `epaint::Margin` | https://docs.rs/epaint/0.35.0/epaint/struct.Margin.html |
| Bevy B0001 error | https://bevy.org/learn/errors/b0001 |
| Bevy migration guides | https://bevyengine.org/learn/migration-guides/ |
