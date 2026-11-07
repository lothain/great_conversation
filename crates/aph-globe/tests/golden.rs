//! Golden-image tests for the renderer: render fixed views (fixed size, frozen time, no HUD)
//! and compare them with the PNGs in `tests/golden/`.
//!
//! These need a GPU and a window, so they are ignored by default:
//!
//! ```text
//! cargo test -p aph-globe --test golden -- --ignored            # compare
//! APH_BLESS=1 cargo test -p aph-globe --test golden -- --ignored # accept new images
//! ```
//!
//! A mismatch writes the actual image and a difference image next to the golden one.

use std::{
    path::{Path, PathBuf},
    process::Command,
};

use image::{ImageBuffer, Rgba, RgbaImage};

struct View {
    name: &'static str,
    year: f32,
    lat: f32,
    lon: f32,
    distance: f32,
}

const SIZE: &str = "960x600";

const VIEWS: [View; 4] = [
    // Rome at its height: one large civilization, early threads, the halo fade.
    View { name: "rome-117", year: 117.0, lat: 40.0, lon: 15.0, distance: 2.6 },
    // Europe in 1950: dense dots, many threads with arrowheads, overlapping civilizations.
    View { name: "europe-1950", year: 1950.0, lat: 48.0, lon: 10.0, distance: 1.8 },
    // Close in: border lines must stay crisp and one width when magnified.
    View { name: "close-1950", year: 1950.0, lat: 50.0, lon: 10.0, distance: 1.3 },
    // The whole globe from far away: limb glow, graticule, nothing yet said.
    View { name: "far-before", year: -1000.0, lat: 20.0, lon: 30.0, distance: 5.5 },
];

/// Mean absolute channel difference allowed (0-255 scale).
const MEAN_TOLERANCE: f64 = 1.5;
/// Fraction of pixels allowed to differ by more than `PIXEL_TOLERANCE` in any channel.
const OUTLIER_FRACTION: f64 = 0.005;
const PIXEL_TOLERANCE: u8 = 32;

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

fn render(view: &View, out: &Path) {
    let status = Command::new(env!("CARGO_BIN_EXE_aph-globe"))
        .env("APH_SNAPSHOT", out)
        .env("APH_SIZE", SIZE)
        .env("APH_YEAR", view.year.to_string())
        .env("APH_LAT", view.lat.to_string())
        .env("APH_LON", view.lon.to_string())
        .env("APH_DIST", view.distance.to_string())
        .env("RUST_LOG", "error")
        .status()
        .expect("renderer runs");
    assert!(status.success(), "renderer exited with {status}");
    assert!(out.exists(), "renderer wrote {}", out.display());
}

struct Diff {
    mean: f64,
    outliers: f64,
    image: RgbaImage,
}

fn compare(expected: &RgbaImage, actual: &RgbaImage) -> Diff {
    assert_eq!(expected.dimensions(), actual.dimensions(), "image size changed");
    let mut total = 0u64;
    let mut outliers = 0u64;
    let image = ImageBuffer::from_fn(expected.width(), expected.height(), |x, y| {
        let (a, b) = (expected.get_pixel(x, y), actual.get_pixel(x, y));
        let d = [0, 1, 2].map(|c| a[c].abs_diff(b[c]));
        total += d.iter().map(|&v| u64::from(v)).sum::<u64>();
        let worst = *d.iter().max().unwrap_or(&0);
        if worst > PIXEL_TOLERANCE {
            outliers += 1;
        }
        Rgba([worst.saturating_mul(4), 0, 0, 255])
    });
    let pixels = f64::from(expected.width() * expected.height());
    Diff { mean: total as f64 / (pixels * 3.0), outliers: outliers as f64 / pixels, image }
}

#[test]
#[ignore = "needs a GPU and a window; run with --ignored"]
fn renders_match_the_golden_images() {
    let dir = golden_dir();
    let bless = std::env::var_os("APH_BLESS").is_some();
    let mut failures = Vec::new();

    for view in &VIEWS {
        let golden = dir.join(format!("{}.png", view.name));
        let actual_path = dir.join(format!("{}.actual.png", view.name));
        render(view, &actual_path);
        let actual = image::open(&actual_path).expect("readable snapshot").to_rgba8();

        if bless || !golden.exists() {
            actual.save(&golden).expect("write golden image");
            std::fs::remove_file(&actual_path).ok();
            eprintln!("blessed {}", golden.display());
            continue;
        }
        let expected = image::open(&golden).expect("readable golden image").to_rgba8();
        let diff = compare(&expected, &actual);
        if diff.mean > MEAN_TOLERANCE || diff.outliers > OUTLIER_FRACTION {
            diff.image.save(dir.join(format!("{}.diff.png", view.name))).ok();
            failures.push(format!("{}: mean diff {:.2}, outliers {:.3}%", view.name, diff.mean, diff.outliers * 100.0));
        } else {
            std::fs::remove_file(&actual_path).ok();
        }
    }
    assert!(failures.is_empty(), "renders differ from golden images:\n{}", failures.join("\n"));
}
