//! Keyboard controls and an on-screen year for running the globe on its own.
//! In the web build the HTML time bar drives `ShownTime` instead.

use aph_scene::time::Clock;
use bevy::prelude::*;

use crate::{
    clock::{FIRST_YEAR, LAST_YEAR, ShownTime},
    scene::Scene,
};

const WINDOWS: [f32; 4] = [0.0, 50.0, 150.0, 400.0];

#[derive(Resource, Debug)]
struct Playback {
    playing: bool,
    /// Years per second.
    speed: f32,
}

pub struct DevControlsPlugin;

impl Plugin for DevControlsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Playback { playing: false, speed: 40.0 })
            .add_systems(Startup, spawn_hud)
            .add_systems(Update, (keys, play, hud).chain());
        if let Some(shot) = Snapshot::from_env() {
            app.insert_resource(shot).add_systems(PostStartup, apply_snapshot).add_systems(Update, take_snapshot);
        }
    }
}

/// `APH_SNAPSHOT=out.png [APH_YEAR=117] [APH_LAT=41 APH_LON=12 APH_DIST=2.4]`:
/// render one view, save it, and exit. For checking the renderer without a person at the screen.
#[derive(Resource, Debug)]
struct Snapshot {
    path: std::path::PathBuf,
    year: Option<f32>,
    orbit: Option<crate::camera::Orbit>,
    frames: u32,
}

impl Snapshot {
    fn from_env() -> Option<Self> {
        let var = |k: &str| std::env::var(k).ok();
        let num = |k: &str| var(k).and_then(|v| v.parse::<f32>().ok());
        let orbit = match (num("APH_LAT"), num("APH_LON")) {
            (Some(lat), Some(lon)) => Some(crate::camera::Orbit {
                over: aph_scene::sphere::LatLon::new(lat, lon),
                distance: num("APH_DIST").unwrap_or(2.6),
            }),
            _ => None,
        };
        Some(Self { path: var("APH_SNAPSHOT")?.into(), year: num("APH_YEAR"), orbit, frames: 0 })
    }
}

fn apply_snapshot(
    shot: Res<Snapshot>,
    mut shown: ResMut<ShownTime>,
    mut orbit: Query<&mut crate::camera::Orbit>,
    mut time: ResMut<Time<Virtual>>,
    mut hud: Query<&mut Visibility, With<Hud>>,
) {
    for mut v in &mut hud {
        *v = Visibility::Hidden;
    }
    // Freeze animation (pulses, glows) so the same view renders the same pixels.
    time.pause();
    if let Some(year) = shot.year {
        shown.0.year = year;
    }
    if let (Some(o), Ok(mut current)) = (shot.orbit, orbit.single_mut()) {
        *current = o;
    }
}

fn take_snapshot(mut commands: Commands, mut shot: ResMut<Snapshot>, mut exit: MessageWriter<AppExit>) {
    use bevy::render::view::screenshot::{Screenshot, save_to_disk};
    shot.frames += 1;
    // Give pipelines time to compile and the overlay to upload.
    match shot.frames {
        90 => {
            commands.spawn(Screenshot::primary_window()).observe(save_to_disk(shot.path.clone()));
        }
        120 => {
            exit.write(AppExit::Success);
        }
        _ => {}
    }
}

#[derive(Component)]
struct Hud;

fn spawn_hud(mut commands: Commands) {
    commands.spawn((
        Hud,
        Text::new(""),
        TextFont { font_size: bevy::text::FontSize::Px(18.0), ..default() },
        TextColor(Color::srgb_u8(0xef, 0xe4, 0xc8)),
        Node { position_type: PositionType::Absolute, left: px(16), top: px(12), ..default() },
    ));
}

fn keys(input: Res<ButtonInput<KeyCode>>, time: Res<Time>, mut shown: ResMut<ShownTime>, mut playback: ResMut<Playback>) {
    let clock = &mut shown.0;
    if input.just_pressed(KeyCode::Space) {
        if clock.year >= LAST_YEAR {
            clock.year = FIRST_YEAR;
        }
        playback.playing = !playback.playing;
    }
    if input.just_pressed(KeyCode::KeyW) {
        let i = WINDOWS.iter().position(|w| *w == clock.window).unwrap_or(0);
        clock.window = WINDOWS[(i + 1) % WINDOWS.len()];
    }
    if input.just_pressed(KeyCode::ArrowUp) {
        playback.speed = (playback.speed * 2.0).min(640.0);
    }
    if input.just_pressed(KeyCode::ArrowDown) {
        playback.speed = (playback.speed / 2.0).max(5.0);
    }
    let direction = f32::from(i8::from(input.pressed(KeyCode::ArrowRight)) - i8::from(input.pressed(KeyCode::ArrowLeft)));
    if direction != 0.0 {
        let rate = if input.pressed(KeyCode::ShiftLeft) { 800.0 } else { 150.0 };
        clock.year = (clock.year + direction * rate * time.delta_secs()).clamp(FIRST_YEAR, LAST_YEAR);
    }
}

fn play(time: Res<Time>, mut shown: ResMut<ShownTime>, mut playback: ResMut<Playback>) {
    if playback.playing {
        shown.0.year = (shown.0.year + playback.speed * time.delta_secs()).min(LAST_YEAR);
        playback.playing = shown.0.year < LAST_YEAR;
    }
}

fn era(year: f32) -> String {
    let y = year.round() as i32;
    if y < 1 { format!("{} BCE", 1 - y) } else { format!("{y} CE") }
}

fn hud(shown: Res<ShownTime>, playback: Res<Playback>, scene: Res<Scene>, mut text: Single<&mut Text, With<Hud>>) {
    let clock: Clock = shown.0;
    let nodes = scene.atlas.nodes.iter().filter(|n| clock.shows(n.from as f32, n.until as f32)).count();
    let threads = scene.atlas.threads.iter().filter(|t| clock.drawn(clock.age(t.year as f32)).is_some()).count();
    let window = if clock.window > 0.0 { format!("last {} years", clock.window) } else { "everything so far".into() };
    text.0 = format!(
        "{}   {}\n{nodes} aphorisms | {threads} threads | {window}\n\
         space play | left/right scrub (shift: fast) | up/down speed {} y/s | W window | drag turn | scroll zoom",
        era(clock.year),
        if playback.playing { ">" } else { "||" },
        playback.speed,
    );
}
