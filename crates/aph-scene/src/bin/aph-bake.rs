//! Precompute what would otherwise be built while the user waits:
//!
//! - `page-model.json`: everything the page's widgets need, so the page renders without
//!   waiting for the globe's WASM module;
//! - `coasts.png`: the coastline distance field (8-bit, equirectangular), so the globe
//!   doesn't compute it at startup.
//!
//! `cargo run -p aph-scene --features bake --bin aph-bake [-- data-dir [out-dir]]`
//! (defaults: `data/` and `data/baked/`). Trunk runs it before every web build.

use std::{fs, path::PathBuf, time::Instant};

use anyhow::{Context, Result};
use aph_data::store::Dataset;
use aph_scene::{atlas::Atlas, export::PageModel, overlay};

/// Must match the globe's overlay texture size.
const COAST_SIZE: (u32, u32) = (2048, 1024);

fn main() -> Result<()> {
    let mut args = std::env::args_os().skip(1).map(PathBuf::from);
    let data = args.next().unwrap_or_else(|| crate_dir().join("../../data"));
    let out = args.next().unwrap_or_else(|| data.join("baked"));
    fs::create_dir_all(&out).with_context(|| format!("creating {}", out.display()))?;

    let started = Instant::now();
    let dataset = Dataset::load(&data)?;
    let atlas = Atlas::build(&dataset);

    let page = serde_json::to_string(&PageModel::build(&dataset, &atlas))?;
    fs::write(out.join("page-model.json"), &page)?;

    let (w, h) = COAST_SIZE;
    if atlas.land.is_empty() {
        eprintln!("no land data; skipping coasts.png");
    } else {
        let field = overlay::coasts(&atlas.land, w, h);
        image::GrayImage::from_raw(w, h, field)
            .context("field matches its size")?
            .save(out.join("coasts.png"))
            .context("writing coasts.png")?;
    }
    for missing in &atlas.unplaced {
        eprintln!("unplaced thread: {missing}");
    }
    eprintln!("baked {} ({} KB page model) in {:?}", out.display(), page.len() / 1024, started.elapsed());
    Ok(())
}

/// This crate's folder: from Cargo when run through it (so a moved checkout keeps working
/// without a rebuild), else where it was built.
fn crate_dir() -> PathBuf {
    std::env::var_os("CARGO_MANIFEST_DIR").map_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")), PathBuf::from)
}
