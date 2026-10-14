//! What every admitted material reflects — `cargo run -p voxel-shapers
//! --example surface-colours`
//!
//! Prints the sampled reflectance of each admitted material, because "the ground is one
//! pale playa" is a claim about numbers before it is a claim about pixels. The five
//! surface covers (C-159.10) are the rows whose name is a biome's ground.
//!
//! This is the appearance layer's input, not its output: `openpbr::derive` turns these
//! channels into the shader's base colour, and that derivation is tested in
//! `crates/voxel-client/src/shading.rs`.

use voxel_shapers::materials;

fn main() {
    let registry = materials::registry().expect("the admitted registry builds");
    println!("{:>3} {:<18} {:>8}  reflectance (r, g, b)", "id", "name", "kg/m3");
    for m in registry.materials() {
        let (r, g, b) = match &m.reflectance {
            Some(sourced) => {
                let [r, g, b] = sourced.value.linear();
                (r, g, b)
            }
            None => (f64::NAN, f64::NAN, f64::NAN),
        };
        println!(
            "{:>3} {:<18} {:>8.0}  ({:.4}, {:.4}, {:.4})",
            m.id.0,
            m.name,
            m.bulk_density_kg_m3().to_f64(),
            r,
            g,
            b
        );
    }
}
