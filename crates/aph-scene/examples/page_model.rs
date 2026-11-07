//! Print the page model as JSON: `cargo run -p aph-scene --example page_model > model.json`
use std::path::Path;

fn main() -> anyhow::Result<()> {
    let data = aph_data::store::Dataset::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data"))?;
    let atlas = aph_scene::atlas::Atlas::build(&data);
    println!("{}", serde_json::to_string(&aph_scene::export::PageModel::build(&data, &atlas))?);
    Ok(())
}
