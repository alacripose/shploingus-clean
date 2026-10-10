//! The **quad format**: one rectangle, 24 bytes, per mesh quad, recorded once
//! per world chunk and expanded by the GPU's `vs_quad`.
//!
//! Why a separate record instead of the four-corner vertex expansion the
//! `Scene::build` path used to emit: at ±2,000,000 m a world-space `f32`
//! metre loses subvoxel resolution, so four corners owned by the renderer would
//! have to be rebuilt every time the eye moves. A chunk-local rectangle owned by
//! the world does not — the pipeline subtracts the eye in `f64` at upload time
//! (`origin_bytes`), and the rectangle's 24 bytes are independent of the camera.
//!
//! This is the culling path's dual to the vertex path: where
//! [`crate::scene::Scene::build`] emits four per-quad corners with the quad's
//! face normal, this module emits one rectangle that `shading.wgsl`'s `vs_quad`
//! expands into two triangles in the face's own `(u, v)` basis. The winding the
//! vertex path picks — `(0,1,2) (0,2,3)` — is the same winding `vs_quad`
//! produces, so both paths end at the same two triangles and the pipeline does
//! not care which one produced them.

use voxel_model::{Face, MaterialId};
use voxel_mesh::Quad;

/// Which face each record names. 6 entries — one per axis direction.
pub const FACES_BY_INDEX: [Face; 6] = [
    Face::NegX,
    Face::PosX,
    Face::NegY,
    Face::PosY,
    Face::NegZ,
    Face::PosZ,
];

/// Encode the 6 face enum values as the compact index the quad format stores.
pub fn face_index(face: Face) -> u32 {
    match face {
        Face::NegX => 0,
        Face::PosX => 1,
        Face::NegY => 2,
        Face::PosY => 3,
        Face::NegZ => 4,
        Face::PosZ => 5,
    }
}

/// One hex-octant algebra constant: sqrt(0.5).
pub const COVER_ONE_FACE: f64 = 0.7071067811865476;

/// A face normal as a chunk-local `(u,v,n)` basis, for normals that live in
/// face space rather than world space.
pub fn basis_for(face: Face) -> [f64; 3] {
    match face {
        Face::NegX => [-1.0, 0.0, 0.0],
        Face::PosX => [1.0, 0.0, 0.0],
        Face::NegY => [0.0, -1.0, 0.0],
        Face::PosY => [0.0, 1.0, 0.0],
        Face::NegZ => [0.0, 0.0, -1.0],
        Face::PosZ => [0.0, 0.0, 1.0],
    }
}

/// The origin storage buffer's element stride in bytes (`vec4<f32>` on GPU).
pub const ORIGIN_SIZE: u64 = 16;

/// Encode chunk origins relative to eye into little-endian bytes for the GPU origin buffer.
///
/// The `w` channel is the **veil** (C-179): a cloud cell's own transmittance, from
/// `cloud_cubes`, or `0.0` for a chunk of the world — which is every origin that existed before
/// clouds did, so nothing drawn earlier can change by adding this. Position is three floats and
/// the fourth was written as zero and read by nothing, which is what makes it the veil's home
/// rather than a new field in the record format or a new binding.
pub fn origin_bytes(origins: &[[f64; 4]], eye_m: [f64; 3]) -> Vec<u8> {
    let mut out = Vec::with_capacity(origins.len() * ORIGIN_SIZE as usize);
    for origin in origins {
        let rel_x = (origin[0] - eye_m[0]) as f32;
        let rel_y = (origin[1] - eye_m[1]) as f32;
        let rel_z = (origin[2] - eye_m[2]) as f32;
        let w = origin[3] as f32;
        out.extend_from_slice(&rel_x.to_le_bytes());
        out.extend_from_slice(&rel_y.to_le_bytes());
        out.extend_from_slice(&rel_z.to_le_bytes());
        out.extend_from_slice(&w.to_le_bytes());
    }
    out
}

/// One rectangle in the quad format: the face, the quad's own material, the
/// slice the rectangle lives on, and the rectangle's `(uv)` low corner and
/// `(extent)` size in that face's own `(u, v)` basis.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct QuadRecord {
    /// Which face this rectangle lives on, as one of the six `FACES_BY_INDEX`
    /// values above. The GPU's `vs_quad` reconstructs the face basis from this.
    pub face: u32,
    /// The rectangle's material, packed as `[composition.0, composition.1]` —
    /// two `MaterialId`s in one `u32`, low then high. For the quad format the
    /// composition is the face blend of the quad's own selection, so a
    /// single-material quad carries its own id in slot 0 and empty in slot 1.
    pub material: u32,
    /// Which slice of the chiseled surface the rectangle lives on. For a plain
    /// voxel mesh this is the slice the mesh's shaper put the quad on;
    /// `vs_quad` uses it to pick the correct `(u,v)` origin.
    pub slice: u32,
    /// The rectangle's low corner in the face's `(u,v)` basis, packed as
    /// `uv.x | uv.y << 16`.
    pub uv: u32,
    /// The rectangle's size in the face's `(u,v)` basis, packed as
    /// `extent.x | extent.y << 16`.
    pub extent: u32,
    /// Which entry of the scene's origin array this rectangle sits on.
    pub chunk: u32,
}

impl QuadRecord {
    /// The record's fields, in the order they are written.
    pub const FIELDS: usize = 6;

    /// The record's stride in bytes — what the GPU's `array<QuadRecord>` uses.
    pub const SIZE: u64 = Self::FIELDS as u64 * 4;

    /// The record for one mesh quad, on the chunk at `chunk`.
    ///
    /// The quad's rectangle is the quad's own selection — the only thing a
    /// quad carries — so a quad's recorded rectangle is itself, and the record
    /// for a single-material quad is the same rectangle as before (the mesh
    /// never merges across materials). For the quad format, that single entry
    /// is the correct shade because the drawn quad is one material by
    /// construction.
    pub fn of(quad: &Quad, chunk: u32) -> Self {
        QuadRecord {
            face: quad.face as u32,
            material: quad.material.0 as u32,
            slice: quad.slice as u32,
            uv: quad.uv[0] as u32 | ((quad.uv[1] as u32) << 16),
            extent: quad.extent[0] as u32 | ((quad.extent[1] as u32) << 16),
            chunk,
        }
    }

    /// The record for a quad's **rectangle selection carrying a face composition**.
    ///
    /// The quad's rectangle (`uv`, `extent`, `slice`, `face`) is the selection.
    /// The composition is that selection's face blend, so the shader can shade
    /// every pixel inside the rectangle from its own blend rather than from one
    /// material id. A single-material composition writes the majority id, which
    /// is the old behaviour — the rectangle is the same rectangle, and a
    /// undiscriminated quad continues to shade as one material.
    pub fn of_composition(quad: &Quad, chunk: u32, composition: [MaterialId; 2]) -> Self {
        let packed = (composition[0].0 as u32) | ((composition[1].0 as u32) << 16);
        QuadRecord {
            face: quad.face as u32,
            material: packed,
            slice: quad.slice as u32,
            uv: quad.uv[0] as u32 | ((quad.uv[1] as u32) << 16),
            extent: quad.extent[0] as u32 | ((quad.extent[1] as u32) << 16),
            chunk,
        }
    }

    /// The rectangle this record names, rebuilt exactly.
    pub fn to_quad(self) -> Quad {
        Quad {
            face: Face::ALL[self.face as usize],
            slice: self.slice as u16,
            uv: [self.uv as u16, (self.uv >> 16) as u16],
            extent: [self.extent as u16, (self.extent >> 16) as u16],
            material: MaterialId(self.material as u16),
        }
    }

    /// The rectangle's low corner in the face's own `(u, v)` basis.
    pub fn low(self) -> [u16; 2] {
        [self.uv as u16, (self.uv >> 16) as u16]
    }

    /// The rectangle's size in the face's own `(u, v)` basis.
    pub fn size(self) -> [u16; 2] {
        [self.extent as u16, (self.extent >> 16) as u16]
    }

    fn write(self, out: &mut Vec<u8>) {
        for field in [self.face, self.material, self.slice, self.uv, self.extent, self.chunk] {
            out.extend_from_slice(&field.to_le_bytes());
        }
    }

    /// The records as the bytes a storage buffer takes, `SIZE` per record.
    pub fn bytes(records: &[QuadRecord]) -> Vec<u8> {
        let mut out = Vec::with_capacity(records.len() * Self::SIZE as usize);
        for r in records {
            r.write(&mut out);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_model::MaterialId;

    #[test]
    fn an_origin_carries_the_veil_in_its_own_w() {
        // **The veil's channel** (C-179): position is three floats and `w` was written as zero
        // and read by nothing, which is what makes it a cloud cell's transmittance rather than
        // a new field in the record or a new binding. A chunk's zero is the value it already
        // had, so nothing drawn before clouds can change by this.
        let bytes = origin_bytes(&[[1.0, 2.0, -3.0, 0.72]], [0.0, 0.0, 0.0]);
        assert_eq!(bytes.len(), ORIGIN_SIZE as usize);
        let f = |at: usize| f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        assert_eq!((f(0), f(4), f(8)), (1.0, 2.0, -3.0));
        assert_eq!(f(12), 0.72, "the veil did not reach the origin's own w");
        // And the eye is still subtracted, in the world's own `f64`.
        let moved = origin_bytes(&[[1.0, 2.0, -3.0, 0.0]], [0.5, 0.25, -0.25]);
        let g = |at: usize| f32::from_le_bytes(moved[at..at + 4].try_into().unwrap());
        assert_eq!((g(0), g(4), g(8)), (0.5, 1.75, -2.75));
    }

    #[test]
    fn of_yields_a_record_whose_rectangle_is_the_quad() {
        let q = Quad {
            face: Face::PosY,
            slice: 3,
            uv: [7, 11],
            extent: [4, 6],
            material: MaterialId(42),
        };
        let r = QuadRecord::of(&q, 1);
        assert_eq!(r.face, 3);
        assert_eq!(r.slice, 3);
        assert_eq!(r.uv, 7 | (11 << 16));
        assert_eq!(r.extent, 4 | (6 << 16));
        assert_eq!(r.chunk, 1);
    }

    #[test]
    fn of_composition_packs_two_ids() {
        let q = Quad {
            face: Face::NegZ,
            slice: 0,
            uv: [0, 0],
            extent: [1, 1],
            material: MaterialId(0),
        };
        let r = QuadRecord::of_composition(&q, 2, [MaterialId(7), MaterialId(12)]);
        assert_eq!(r.face, 4);
        assert_eq!(r.material, 7 | (12 << 16));
    }

    #[test]
    fn round_trip_keeps_the_rectangle() {
        let q = Quad {
            face: Face::PosX,
            slice: 5,
            uv: [3, 8],
            extent: [2, 2],
            material: MaterialId(99),
        };
        let r = QuadRecord::of(&q, 0);
        let back = r.to_quad();
        assert_eq!(back.face, q.face);
        assert_eq!(back.slice, q.slice);
        assert_eq!(back.uv, q.uv);
        assert_eq!(back.extent, q.extent);
        assert_eq!(back.material, q.material);
    }

    #[test]
    fn bytes_are_the_fields_in_order() {
        let r = QuadRecord {
            face: 2,
            material: 0x0001_0002,
            slice: 3,
            uv: 0x0004_0005,
            extent: 0x0006_0007,
            chunk: 8,
        };
        let b = QuadRecord::bytes(&[r]);
        assert_eq!(b.len(), 24);
        assert_eq!(&b[0..4], &2u32.to_le_bytes());
        assert_eq!(&b[4..8], &0x0001_0002u32.to_le_bytes());
        assert_eq!(&b[8..12], &3u32.to_le_bytes());
        assert_eq!(&b[12..16], &0x0004_0005u32.to_le_bytes());
        assert_eq!(&b[16..20], &0x0006_0007u32.to_le_bytes());
        assert_eq!(&b[20..24], &8u32.to_le_bytes());
    }
}
