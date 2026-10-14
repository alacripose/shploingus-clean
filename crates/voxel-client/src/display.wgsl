// ---- the display stage ------------------------------------------------------
//
// **This is the one copy**, and it is a file rather than three blocks because of what
// the three blocks used to say about themselves. `srgb_encode`, `srgb_decode`, `bayer8`,
// `dithered` and `tone_map` existed three times over — in `shading.wgsl`, in `sky.wgsl`
// and in `clouds.wgsl` (since removed: the clouds are geometry drawn through
// `shading.wgsl`, not a pass of their own) — and each of the three admitted it in a
// comment: *WGSL has no `include`, and a shader cannot call another shader.*
//
// That is true of WGSL. It was never true of the build: the host has `include_str!`, so
// [`world::pass_source`] concatenates this file in front of every pass that presents a
// frame and the pass is compiled with it already in scope. **A copy is a transcription
// nobody checks**, and the failure is not a compile error: it is a curve changed in one
// pass and not in the others, which is a frame whose sky and whose clouds disagree about
// what white is — visible only as a seam where they meet, and only to somebody already
// looking for it. One file cannot disagree with itself.
//
// Every pass that presents takes it; the merge compute pass and the wireframe do not,
// because neither writes a colour into the target.

// ---- dithering the target's own quantisation (C-166) ------------------------
//
// The window presents an **8-bit `sRGB` surface**, so a shaded value with more than 256
// levels is rounded on the way out, and a smooth gradient — the sky is the one everybody
// sees, and a lit cloud edge is the second — arrives as **bands**. The repair is to add
// **less than one step of the target's own encoding**, placed by an ordered threshold,
// so neighbouring pixels round different ways and the band edge dissolves into a fine
// texture instead of drawing a contour.
//
// **Ordered (Bayer), not hashed, and the reason is the whole of it.** A hashed offset is
// white noise per pixel, and per pixel means per *frame* as the camera moves: it would
// trade static banding for crawling shimmer, which is the artifact this exists to
// remove. Bayer is a fixed function of the framebuffer coordinate, so it is stable in
// place while the camera obeys it.
//
// The offset is applied **in the target's encoded space** and then decoded back, because
// that is where the rounding happens: one step of `sRGB` *is* one step of the 8-bit
// channel. The same offset applied in linear space would be roughly four times too large
// in the darks, where the encoding's slope is steepest, and about half a step too small
// at the top — noise in the shadows and banding left at the bright end, which is the
// worst of both.
//
// The amount is a flag, not a constant: `0.0` returns the colour untouched, and every
// caller passes its own — `camera.screen.y` in the world pass, `view.dither` in the sky,
// `view.screen.x` in the clouds. It is `0` for a target that quantises nothing (the
// offscreen `Rgba32Float` verification target), so a measurement frame is the number the
// model computed rather than that number with an offset spent on it.

/// One step of an 8-bit channel, in the channel's own encoded units.
const DITHER_STEP: f32 = 1.0 / 255.0;

/// `sRGB`'s transfer function, so the offset can be placed where the rounding is.
fn srgb_encode(c: vec3<f32>) -> vec3<f32> {
    let l = clamp(c, vec3<f32>(0.0), vec3<f32>(1.0));
    let linear = l * 12.92;
    let curve = 1.055 * pow(l, vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(curve, linear, l <= vec3<f32>(0.0031308));
}

fn srgb_decode(e: vec3<f32>) -> vec3<f32> {
    let e_clamped = clamp(e, vec3<f32>(0.0), vec3<f32>(1.0));
    let curve = pow((e_clamped + 0.055) / 1.055, vec3<f32>(2.4));
    let linear = e_clamped / 12.92;
    return select(curve, linear, e_clamped <= vec3<f32>(0.04045));
}

/// An **8×8 ordered (Bayer) threshold**, in `[-0.5, 0.5)`.
///
/// The matrix is the standard recursive construction written out; it is a function of
/// the framebuffer coordinate alone, which is what makes the dither stable under camera
/// motion.
fn bayer8(p: vec2<f32>) -> f32 {
    var matrix = array<u32, 64>(
         0u, 32u,  8u, 40u,  2u, 34u, 10u, 42u,
        48u, 16u, 56u, 24u, 50u, 18u, 58u, 26u,
        12u, 44u,  4u, 36u, 14u, 46u,  6u, 38u,
        60u, 28u, 52u, 20u, 62u, 30u, 54u, 22u,
         3u, 35u, 11u, 43u,  1u, 33u,  9u, 41u,
        51u, 19u, 59u, 27u, 49u, 17u, 57u, 25u,
        15u, 47u,  7u, 39u, 13u, 45u,  5u, 37u,
        63u, 31u, 55u, 23u, 61u, 29u, 53u, 21u,
    );
    let x = u32(p.x) & 7u;
    let y = u32(p.y) & 7u;
    return (f32(matrix[y * 8u + x]) + 0.5) / 64.0 - 0.5;
}

/// The colour with the target's rounding spread across the pixel.
///
/// `amount` is the presenting pass's own flag, as above: **`0.0` returns the colour
/// untouched**, which is what keeps a float target an exact measurement.
fn dithered(colour: vec3<f32>, p: vec2<f32>, amount: f32) -> vec3<f32> {
    if (amount <= 0.0) {
        return colour;
    }
    let encoded = srgb_encode(colour);
    let moved = encoded + vec3<f32>(bayer8(p) * DITHER_STEP * amount);
    return srgb_decode(moved);
}

// ---- the display transform (C-167) -----------------------------------------
//
// A transcription of `voxel_appearance::tonemap`, held to it by
// `renderer::tests::the_gpu_presents_a_radiance_the_crate_says_it_does`.
//
// Everything a pass produces before this point is **scene-referred**: the sky, the
// world's surfaces and a cloud all produce physical radiance, and it is allowed to exceed
// 1. A sunlit sky and a shadowed rock differ by orders of magnitude, and a renderer that
// writes that straight at an 8-bit target is not rendering HDR — it is clipping, at
// whatever the arithmetic happened to produce. The measured state of this project is
// exactly that: the world's brightest pixel reads **1.886** where the frame's contract
// says `[0, 1]`. A cloud makes it worse rather than better, because a cloud's whole
// brightness is scattering *toward* the eye and a lit edge is brighter than the sky it
// stands in front of.
//
// Reinhard's `L -> L / (1 + L)` (Reinhard et al. 2002), on the colour's own luminance.
// **It has no free parameter**, and that is why it is here rather than a film curve: a
// shoulder, a toe and a linear section would each be a number nobody chose, spent on
// every pixel, in a project that counts its unsourced constants in a test. It adds no
// entry to `tonemap::CONSTANTS`, which is the criterion it was selected on rather than a
// happy accident.
//
// **The gain is `1 / (1 + L)`, not `(L / (1 + L)) / L`.** They are the same expression,
// and the first cannot divide by a luminance so small that the two disagree — the Rust
// uses the first form too, so the pair computes one expression rather than two a reader
// has to prove equal.
//
// **One stated divergence from the Rust:** a `NaN` luminance presents as black here and
// stays `NaN` there. The model refuses a bad value rather than replacing it with one
// somebody could believe; the shader has to write *something* to a surface, and black is
// the only thing it can write that cannot be read as a measurement.

// The luminance weights of Rec. 709 (ITU-R BT.709), the same cited triple
// `voxel_appearance::openpbr::LinearRgb::luminance` uses. Using a different one would
// tone-map a different quantity than the bar counts bands of.
const LUMINANCE_WEIGHTS: vec3<f32> = vec3<f32>(0.2126, 0.7152, 0.0722);

/// **The display transform**: an unbounded scene-referred colour, made displayable.
///
/// Luminance is compressed and every channel is scaled by that same factor, so the
/// picture's **chromaticity is untouched** — a grey stays grey, a warm wall stays warm, a
/// blue sky stays blue while ceasing to be 1.886.
/// **And it is not a bound**, which is the defect this function had: Rec. 709's weights are
/// dominated by green, so a **red-dominant** colour takes a gain computed mostly from a channel it
/// barely has and leaves the domain — the horizon at sunset is `(2.36, 0.78, 0.01)` after
/// `EXPOSURE`, luminance `1.06`, gain `0.485`, red **`1.145`**. The 8-bit target clips it, and the
/// redness is what is lost: `(1.0, 0.378)` presents a washed yellow where the sky computed orange.
///
/// The repair is one scalar gain, the smaller of the curve's own and the brightest channel's own
/// bound, so chromaticity is still untouched and no channel can leave `[0, 1]`. The Rust form is
/// `voxel_appearance::tonemap::tone_map` and the two are held together by
/// `renderer::tests::the_gpu_presents_a_radiance_the_crate_says_it_does`.
fn tone_map(colour: vec3<f32>) -> vec3<f32> {
    let l = dot(colour, LUMINANCE_WEIGHTS);
    // `!(l > 0.0)` rather than `l <= 0.0`: it is the same test for every finite value and
    // it also catches a `NaN`, which `l <= 0.0` answers `false` for and would let through
    // to multiply the whole colour by a `NaN`.
    if (!(l > 0.0)) {
        return vec3<f32>(0.0);
    }
    let brightest = max(colour.r, max(colour.g, colour.b));
    return colour * min(1.0 / (1.0 + l), 1.0 / brightest);
}
