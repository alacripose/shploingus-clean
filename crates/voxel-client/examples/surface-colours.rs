//! What every admitted material looks like — `cargo run -p voxel-client
//! --example surface-colours`
//!
//! Prints the OpenPBR base colour of each admitted material, because "the ground is one
//! pale playa" is a claim about numbers before it is a claim about pixels. The five
//! surface covers (C-159.10) are the rows whose name is a biome's ground.

use voxel_appearance::openpbr::Colour;
use voxel_client::shading;
use voxel_shapers::materials;

fn main() {
    let registry = materials::registry().expect("the admitted registry builds");
    let field = materials::shading_field().expect("the world's shading field declares");
    let _ = shading::table(&registry, &field).expect("every admitted material derives");
    println!("{:>3} {:<18} {:>8}  base colour (linear rgb)", "id", "name", "kg/m3");
    for m in registry.materials() {
        let inputs = shading::inputs_for(&registry, m.id).expect("derives");
        let (r, g, b) = match inputs.base_color {
            Colour::Admitted(c) => (c.r, c.g, c.b),
            Colour::Unadmitted => (0.0, 0.0, 0.0),
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
