//! **What this machine's adapter will actually do.**
//!
//! Both `request_device` calls in this workspace pass `..Default::default()`, which
//! means `Features::empty()` — the most conservative device `wgpu` can make. Before any
//! decision about *which* shader pipeline to use, the question is what the hardware in
//! front of you is willing to enable, and that is not something a source file can
//! answer.
//!
//! This is a measurement, not a capability list: it asks the adapter on **this**
//! machine, and a machine that reports differently is a different answer.
//!
//! ```text
//! cargo run --offline -p voxel-client --example adapter-features
//! ```

/// The features a more capable shader pipeline would reach for, with why.
///
/// Only the ones this project has a named use for — a dump of every bitflag would be
/// a list rather than an answer.
const WANTED: &[(wgpu::Features, &str)] = &[
    (
        wgpu::Features::EXPERIMENTAL_RAY_QUERY,
        "ray queries — volumetric multiple scattering, real sky occlusion, GI",
    ),
    (
        wgpu::Features::POLYGON_MODE_LINE,
        "`PolygonMode::Line` — the F4 wireframe the flag already promises and cannot draw",
    ),
    (
        wgpu::Features::SUBGROUP,
        "subgroup ops — the band integration and the pattern ladder across a wavefront",
    ),
    (
        wgpu::Features::SUBGROUP_BARRIER,
        "subgroup barriers — only meaningful with SUBGROUP above",
    ),
    (
        wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING,
        "bindless arrays — measured spectra/CMF tables indexed by MaterialId",
    ),
    (
        wgpu::Features::STORAGE_TEXTURE_ARRAY_NON_UNIFORM_INDEXING,
        "bindless storage textures — the same, for the lattice as an image",
    ),
    (
        wgpu::Features::VERTEX_WRITABLE_STORAGE,
        "writable storage in the vertex stage — GPU-driven geometry, no CPU assembly",
    ),
    (
        wgpu::Features::MULTI_DRAW_INDIRECT_COUNT,
        "multi-draw indirect — chunk draws without a per-chunk call",
    ),
    (
        wgpu::Features::SHADER_F16,
        "f16 arithmetic — half the bytes per band on the spectral path",
    ),
    (
        // `push constants` under their `wgpu 30` name: WGSL spells it `var<immediate>`,
        // and the limit is `Limits::max_immediate_size`.
        wgpu::Features::IMMEDIATES,
        "immediates — the per-frame scalars now carried in uniforms",
    ),
];

fn main() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
        ..Default::default()
    }))
    .expect("an adapter");

    let info = adapter.get_info();
    println!("adapter   {} ({:?})", info.name, info.backend);
    println!("driver    {} {}", info.driver, info.driver_info);
    println!("device    {:?}", info.device_type);

    // **What the workspace asks for today**, so the gap is printed rather than asserted.
    let requested = wgpu::Features::empty();
    let offered = adapter.features();
    println!();
    println!("requested  {requested:?}  (both `request_device` calls pass `..Default::default()`)");
    println!("offered    {} bits set", offered.iter_names().count());
    println!();

    println!("{:<58} this adapter", "feature");
    for (feature, why) in WANTED {
        let held = offered.contains(*feature);
        println!(
            "{:<58} {}",
            feature.to_string(),
            if held { "YES" } else { "no" }
        );
        println!("    -> {why}");
    }

    let worth_it: Vec<&str> = WANTED
        .iter()
        .filter(|(f, _)| offered.contains(*f))
        .map(|(_, why)| *why)
        .collect();
    println!();
    println!(
        "{} of {} requested features are available on this adapter:",
        worth_it.len(),
        WANTED.len()
    );
    for w in &worth_it {
        println!("  * {w}");
    }

    let limits = adapter.limits();
    println!();
    println!("limits");
    println!(
        "  max_storage_buffer_binding_size {} B",
        limits.max_storage_buffer_binding_size
    );
    println!(
        "  max_buffer_size                 {} B",
        limits.max_buffer_size
    );
    println!(
        "  max_compute_workgroup_size_x    {}",
        limits.max_compute_workgroup_size_x
    );
    println!(
        "  max_storage_buffers_per_shader_stage {}",
        limits.max_storage_buffers_per_shader_stage
    );
}
