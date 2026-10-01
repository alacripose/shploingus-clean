//! The canonical sparse voxel octree.
//!
//! The tree spans one chunk: [`OCTREE_LEVELS`] = 5, because a chunk is
//! `CHUNK_SIDE` = 32 = 2⁵ voxels per edge. A node at level `L` covers `2^L`
//! voxels per axis, so level 0 covers one voxel and is a **leaf**:
//!
//! | Node | Covers | Means |
//! |---|---|---|
//! | [`Node::Empty`] | `2^L` voxels | no matter at all |
//! | [`Node::Uniform`] | `2^L` voxels | the same material in every subvoxel |
//! | [`Node::Mixed`] | always 1 voxel | a real 4×4×4 [`VoxelShape`] |
//! | [`Node::Branch`] | `2^L` voxels, `L ≥ 1` | eight children in octant order |
//!
//! # Octant order
//!
//! A child's index is the three low-order bits of its coordinates at this level,
//! **x in bit 0, y in bit 1, z in bit 2**. That is a convention and it is stated
//! once, here, because every digest, every walk and every raycast in this crate
//! depends on it and two different conventions would silently produce two
//! different worlds.
//!
//! # Canonicalisation
//!
//! Every write collapses the tree on the way back up, so a tree is a function of
//! its **contents** and not of its insertion order:
//!
//! * eight `Empty` children become one `Empty`;
//! * eight `Uniform(m)` children with the same `m` become one `Uniform(m)`;
//! * a `Mixed` leaf whose 64 cells all hold one material becomes `Uniform` (or
//!   `Empty`, if that material is empty).
//!
//! This is what makes `is_sparse` and the digest meaningful. It is also, by
//! itself, most of the compression: a chunk of uniform rock is **one** node, not
//! 32,768.

use voxel_model::{
    Digest, LocalPos, MaterialId, Refusal, RefusalKind, Refused, SubvoxelIndex, VoxelShape,
    CHUNK_SIDE, CHUNK_VOLUME, SUBVOXEL_SIDE,
};

/// Levels in the tree. A chunk is `2^5` voxels per edge, so there are five.
pub const OCTREE_LEVELS: u32 = 5;

/// One node of the octree. See the module docs for what each variant covers.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Node {
    /// No matter anywhere under this node.
    Empty,
    /// One material, filling every subvoxel under this node.
    Uniform(MaterialId),
    /// A real 4×4×4 shape. Only ever one voxel wide, so only at level 0.
    Mixed(Box<VoxelShape>),
    /// Eight children, in octant order.
    Branch(Box<[Node; 8]>),
}

impl Node {
    /// The leaf a voxel's shape becomes, already canonical.
    fn from_voxel(shape: VoxelShape) -> Self {
        if shape.is_empty() {
            return Node::Empty;
        }
        let cells = shape.cells();
        let first = cells[0];
        if cells.iter().all(|m| *m == first) {
            // A shape that is the same everywhere is not a shape — it is a
            // material, and saying so is what keeps a uniform world small.
            return if first.is_empty() { Node::Empty } else { Node::Uniform(first) };
        }
        Node::Mixed(Box::new(shape))
    }

    /// Collapse this node if its children let it. Called on every write, on the
    /// way back up.
    fn canonical(self) -> Self {
        match self {
            Node::Branch(children) => {
                if children.iter().all(|c| matches!(c, Node::Empty)) {
                    return Node::Empty;
                }
                if let Node::Uniform(m) = children[0] {
                    if children.iter().all(|c| matches!(c, Node::Uniform(mm) if *mm == m)) {
                        return Node::Uniform(m);
                    }
                }
                Node::Branch(children)
            }
            // Leaves are already canonical by construction.
            other => other,
        }
    }

    /// True when this node is a leaf rather than a branch.
    pub const fn is_leaf(&self) -> bool {
        !matches!(self, Node::Branch(_))
    }

    /// Heap bytes this node's subtree **owns**, excluding the node's own inline
    /// storage (its parent's array accounted for that).
    fn owned_bytes(&self) -> usize {
        match self {
            Node::Empty | Node::Uniform(_) => 0,
            // The box's contents, not the box: the pointer lives in the node,
            // which the caller's array already accounted for.
            Node::Mixed(_) => std::mem::size_of::<VoxelShape>() + ALLOC_OVERHEAD,
            Node::Branch(children) => {
                std::mem::size_of::<[Node; 8]>()
                    + ALLOC_OVERHEAD
                    + children.iter().map(Node::owned_bytes).sum::<usize>()
            }
        }
    }

    /// Nodes in this subtree, including itself.
    fn node_count(&self) -> u64 {
        match self {
            Node::Empty | Node::Uniform(_) | Node::Mixed(_) => 1,
            Node::Branch(children) => 1 + children.iter().map(Node::node_count).sum::<u64>(),
        }
    }
}

/// Allocator overhead charged per heap allocation, in bytes.
///
/// **A declared floor, not a measurement of the allocator.** A general-purpose
/// allocator rounds a request up and stores bookkeeping, so the true cost of an
/// allocation is at least one pointer's worth above what was asked for. Charging
/// a floor and *saying it is a floor* is the honest direction to be wrong in: a
/// memory budget that turns out to be optimistic by
/// `overhead × allocation count` is a budget, whereas one that pretends to a
/// precision nobody measured is a guess with a decimal point.
///
/// It is a `const` so it appears in [`crate::octree`]'s accounting and in the
/// budget that consults it, rather than being buried in an estimate.
pub const ALLOC_OVERHEAD: usize = 16;

/// The octant a local coordinate falls in at a level whose half-extent is
/// `half`. x is bit 0, y is bit 1, z is bit 2 (module docs).
const fn octant_index(local: [i32; 3], half: i32) -> usize {
    let bx = ((local[0] & half) != 0) as usize;
    let by = ((local[1] & half) != 0) as usize;
    let bz = ((local[2] & half) != 0) as usize;
    bx | (by << 1) | (bz << 2)
}

/// A sparse voxel octree over one chunk's `32³` voxels.
///
/// # No cached counters
///
/// `solid_count`, `leaf_count` and `is_sparse` are all **walks**, not fields.
/// A cached count is a count that can drift — one missed increment and the
/// answer is wrong forever with nothing to check it against. A walk over a
/// five-level tree of at most 32,768 voxels is cheap, and it cannot be wrong.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SparseVoxelOctree {
    root: Node,
}

impl Default for SparseVoxelOctree {
    fn default() -> Self {
        Self::new()
    }
}

impl SparseVoxelOctree {
    /// A chunk of nothing.
    pub const fn new() -> Self {
        SparseVoxelOctree { root: Node::Empty }
    }

    /// The root node, for a caller that wants to walk the tree itself.
    pub const fn root(&self) -> &Node {
        &self.root
    }

    /// Heap bytes this tree owns, excluding its own inline root.
    ///
    /// A **walk**, like `solid_count` and `leaf_count` and for the same reason:
    /// a cached byte count is a count that can drift, and the whole point of
    /// asking is to find out what actually happened. Cheap enough to call
    /// between frames, which is what a memory budget does.
    pub fn heap_bytes(&self) -> usize {
        self.root.owned_bytes()
    }

    /// Everything this tree occupies: the inline root plus what it allocated.
    pub fn bytes(&self) -> usize {
        std::mem::size_of::<Self>() + self.heap_bytes()
    }

    /// Nodes in the tree, including the root. What the bytes were spent on.
    pub fn node_count(&self) -> u64 {
        self.root.node_count()
    }

    /// Set one voxel's 4×4×4 shape.
    ///
    /// An **empty** shape removes the voxel rather than storing 64 empties,
    /// which is the same thing said more cheaply.
    pub fn set_voxel(&mut self, local: LocalPos, shape: VoxelShape) {
        let (x, y, z) = (local.x as i32, local.y as i32, local.z as i32);
        set_at(&mut self.root, OCTREE_LEVELS, [x, y, z], shape);
    }

    /// Set one subvoxel of one voxel, promoting the voxel to a `Mixed` leaf if
    /// it was uniform. This is how an authored (or dug) world differs from a
    /// generated one.
    pub fn set_subvoxel(&mut self, local: LocalPos, at: SubvoxelIndex, material: MaterialId) {
        let mut shape = self.voxel(local);
        shape.set(at, material);
        self.set_voxel(local, shape);
    }

    /// The voxel's shape. An absent voxel is an empty shape — which is a fact
    /// about the tree, not a default: the tree genuinely holds no matter there.
    pub fn voxel(&self, local: LocalPos) -> VoxelShape {
        get_at(
            &self.root,
            OCTREE_LEVELS,
            [local.x as i32, local.y as i32, local.z as i32],
        )
    }

    /// One subvoxel's material, without materialising the whole shape.
    pub fn material_at(&self, local: LocalPos, at: SubvoxelIndex) -> MaterialId {
        material_at(
            &self.root,
            OCTREE_LEVELS,
            [local.x as i32, local.y as i32, local.z as i32],
            at,
        )
    }

    /// Visit every **leaf** node with the voxel-space origin of the region it
    /// covers and its level. Branches are walked, never reported, which is the
    /// whole point of a recursive visitor over a tree this shape.
    pub fn visit_leaves(&self, f: &mut impl FnMut([i32; 3], u32, &Node)) {
        visit(&self.root, OCTREE_LEVELS, [0, 0, 0], f);
    }

    /// How many voxels hold matter.
    pub fn solid_count(&self) -> u64 {
        count_solid(&self.root, OCTREE_LEVELS)
    }

    /// How many leaf nodes the tree has.
    ///
    /// A maximally uncompressed tree has one leaf per voxel, so
    /// `leaf_count() == CHUNK_VOLUME` means nothing merged. See [`Self::is_sparse`].
    pub fn leaf_count(&self) -> u64 {
        count_leaves(&self.root)
    }

    /// True when the tree compresses at least one region larger than a single
    /// voxel — that is, when the structure is doing something a flat array of
    /// 32,768 shapes could not.
    ///
    /// Exactly equivalent to `leaf_count() < CHUNK_VOLUME`, because leaves
    /// partition the chunk's voxels and every leaf covers a whole number of
    /// them. Stated as an equality of two independent measures in the tests.
    pub fn is_sparse(&self) -> bool {
        self.leaf_count() < CHUNK_VOLUME as u64
    }

    /// True when every voxel holds the same material — including all-air.
    pub fn is_uniform(&self) -> bool {
        matches!(self.root, Node::Empty | Node::Uniform(_))
    }

    /// A content digest over the canonical tree.
    ///
    /// Because the tree is canonical, this is a function of the voxels and not
    /// of how they got there. Two builds of the same chunk must agree, and that
    /// is a test rather than a hope.
    pub fn digest(&self) -> Digest {
        let mut transcript = Vec::new();
        transcript_node(&self.root, &mut transcript);
        Digest::of_fields(
            "voxel-mesh/octree",
            &[b"v1", transcript.as_slice()],
        )
    }
}

/// Descend to `local` at `level`, replacing whatever node is there.
///
/// A node that is not a branch has to be *expanded* on the way down. Expanding a
/// `Uniform(m)` into eight `Uniform(m)` children is exactly right — it is the
/// same region described in more words. Expanding a `Mixed` leaf the same way is
/// not reachable in a canonical tree (a `Mixed` leaf is one voxel, so it is
/// already at level 0), but it is handled rather than asserted, because a
/// defensive branch that is correct costs one sentence and an `unreachable!`
/// costs a panic in a world generator.
fn set_at(node: &mut Node, level: u32, local: [i32; 3], shape: VoxelShape) {
    if level == 0 {
        *node = Node::from_voxel(shape);
        return;
    }
    let half = 1i32 << (level - 1);
    let index = octant_index(local, half);

    let taken = core::mem::replace(node, Node::Empty);
    let mut children = match taken {
        Node::Branch(c) => c,
        Node::Empty => Box::new(core::array::from_fn(|_| Node::Empty)),
        Node::Uniform(m) => Box::new(core::array::from_fn(|_| Node::Uniform(m))),
        Node::Mixed(b) => Box::new(core::array::from_fn(|_| Node::Mixed(b.clone()))),
    };

    set_at(&mut children[index], level - 1, local, shape);
    *node = Node::Branch(children).canonical();
}

/// Read the shape at `local`.
fn get_at(node: &Node, level: u32, local: [i32; 3]) -> VoxelShape {
    match node {
        Node::Empty => VoxelShape::empty(),
        Node::Uniform(m) => VoxelShape::solid(*m),
        Node::Mixed(b) => **b,
        // A branch at level 0 cannot exist in a canonical tree. Reading it as
        // "no matter" is the safe direction: it under-claims rather than
        // claiming a material the caller never stored.
        Node::Branch(_) if level == 0 => VoxelShape::empty(),
        Node::Branch(c) => {
            let half = 1i32 << (level - 1);
            get_at(&c[octant_index(local, half)], level - 1, local)
        }
    }
}

/// Read one subvoxel, without copying the whole shape.
fn material_at(node: &Node, level: u32, local: [i32; 3], at: SubvoxelIndex) -> MaterialId {
    match node {
        Node::Empty => MaterialId::EMPTY,
        Node::Uniform(m) => *m,
        Node::Mixed(b) => b.get(at),
        Node::Branch(_) if level == 0 => MaterialId::EMPTY,
        Node::Branch(c) => {
            let half = 1i32 << (level - 1);
            material_at(&c[octant_index(local, half)], level - 1, local, at)
        }
    }
}

fn visit(node: &Node, level: u32, origin: [i32; 3], f: &mut impl FnMut([i32; 3], u32, &Node)) {
    if node.is_leaf() {
        f(origin, level, node);
        return;
    }
    let half = 1i32 << (level - 1);
    if let Node::Branch(children) = node {
        for (i, child) in children.iter().enumerate() {
            // Invert `octant_index`: x is bit 0, y is bit 1, z is bit 2.
            let child_origin = [
                origin[0] + if i & 1 != 0 { half } else { 0 },
                origin[1] + if i & 2 != 0 { half } else { 0 },
                origin[2] + if i & 4 != 0 { half } else { 0 },
            ];
            visit(child, level - 1, child_origin, f);
        }
    }
}

/// Voxels covered by a node at `level`.
fn coverage(level: u32) -> u64 {
    let side = 1u64 << level;
    side * side * side
}

fn count_solid(node: &Node, level: u32) -> u64 {
    match node {
        Node::Empty => 0,
        Node::Uniform(_) => coverage(level),
        // A Mixed leaf is one voxel by construction.
        Node::Mixed(_) => 1,
        Node::Branch(children) => children.iter().map(|c| count_solid(c, level - 1)).sum(),
    }
}

fn count_leaves(node: &Node) -> u64 {
    match node {
        Node::Branch(children) => children.iter().map(count_leaves).sum(),
        _ => 1,
    }
}

fn transcript_node(node: &Node, out: &mut Vec<u8>) {
    match node {
        Node::Empty => out.push(0),
        Node::Uniform(m) => {
            out.push(1);
            out.extend_from_slice(&m.0.to_le_bytes());
        }
        Node::Mixed(b) => {
            out.push(2);
            for cell in b.cells() {
                out.extend_from_slice(&cell.0.to_le_bytes());
            }
        }
        Node::Branch(children) => {
            out.push(3);
            for child in children.iter() {
                transcript_node(child, out);
            }
        }
    }
}

/// Refuse a local coordinate outside the chunk — used by callers that take an
/// integer triple rather than a [`LocalPos`].
pub fn local_from(local: [i32; 3]) -> Refused<LocalPos> {
    let in_range = |v: i32| (0..CHUNK_SIDE).contains(&v);
    if !(in_range(local[0]) && in_range(local[1]) && in_range(local[2])) {
        return Err(Refusal::owned(
            RefusalKind::IndexOutOfRange,
            format!(
                "local voxel ({}, {}, {}) outside 0..{CHUNK_SIDE}",
                local[0], local[1], local[2]
            ),
        ));
    }
    Ok(LocalPos { x: local[0] as u32, y: local[1] as u32, z: local[2] as u32 })
}

/// The number of subvoxels along one edge of a voxel, as an `i32`, for the
/// mesher's arithmetic.
pub const SUBVOXEL_SIDE_I32: i32 = SUBVOXEL_SIDE as i32;

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(x: u32, y: u32, z: u32) -> LocalPos {
        LocalPos::new(x, y, z).unwrap()
    }

    fn solid_everywhere(m: MaterialId) -> SparseVoxelOctree {
        let mut t = SparseVoxelOctree::new();
        for z in 0..CHUNK_SIDE as u32 {
            for y in 0..CHUNK_SIDE as u32 {
                for x in 0..CHUNK_SIDE as u32 {
                    t.set_voxel(pos(x, y, z), VoxelShape::solid(m));
                }
            }
        }
        t
    }

    #[test]
    fn an_empty_tree_is_one_empty_leaf() {
        let t = SparseVoxelOctree::new();
        assert_eq!(t.leaf_count(), 1);
        assert_eq!(t.solid_count(), 0);
        assert!(t.is_uniform());
        assert!(t.is_sparse(), "an all-empty chunk is maximally compressed");
        assert_eq!(t.voxel(pos(0, 0, 0)), VoxelShape::empty());
    }

    #[test]
    fn set_and_get_round_trip_including_negative_octants() {
        let mut t = SparseVoxelOctree::new();
        // (17, 5, 29) exercises octant bits 1, 0 and 1 across five levels.
        t.set_voxel(pos(17, 5, 29), VoxelShape::solid(MaterialId(6)));
        assert_eq!(t.voxel(pos(17, 5, 29)), VoxelShape::solid(MaterialId(6)));
        assert_eq!(t.voxel(pos(17, 5, 28)), VoxelShape::empty());
        assert_eq!(t.solid_count(), 1);
    }

    #[test]
    fn a_uniform_chunk_is_one_node_and_a_uniform_shape_is_one_node() {
        // The compression claim, measured two ways: the whole chunk, and a
        // single voxel whose 64 cells are all the same material.
        let t = solid_everywhere(MaterialId(6));
        assert!(t.is_uniform(), "a solid chunk should be a single node");
        assert_eq!(t.leaf_count(), 1);
        assert_eq!(t.solid_count(), CHUNK_VOLUME as u64);

        let mut t2 = SparseVoxelOctree::new();
        t2.set_voxel(pos(4, 4, 4), VoxelShape::solid(MaterialId(6)));
        assert_eq!(t2.tree_kind_at(pos(4, 4, 4)), "uniform");
    }

    #[test]
    fn a_genuinely_mixed_voxel_stays_a_shape() {
        let mut shape = VoxelShape::empty();
        for i in SubvoxelIndex::all() {
            shape.set(i, if i.z < 2 { MaterialId(3) } else { MaterialId(4) });
        }
        let mut t = SparseVoxelOctree::new();
        t.set_voxel(pos(9, 9, 9), shape);
        assert_eq!(t.voxel(pos(9, 9, 9)), shape);
        assert_eq!(t.material_at(pos(9, 9, 9), SubvoxelIndex::new(0, 0, 0).unwrap()), MaterialId(3));
        assert_eq!(t.material_at(pos(9, 9, 9), SubvoxelIndex::new(0, 0, 3).unwrap()), MaterialId(4));
        assert_eq!(t.solid_count(), 1, "a mixed voxel is still one voxel");
    }

    #[test]
    fn setting_a_subvoxel_promotes_a_uniform_voxel_to_a_shape() {
        let mut t = SparseVoxelOctree::new();
        t.set_voxel(pos(1, 1, 1), VoxelShape::solid(MaterialId(6)));
        t.set_subvoxel(pos(1, 1, 1), SubvoxelIndex::new(0, 0, 0).unwrap(), MaterialId(1));
        assert_eq!(
            t.material_at(pos(1, 1, 1), SubvoxelIndex::new(0, 0, 0).unwrap()),
            MaterialId(1)
        );
        assert_eq!(
            t.material_at(pos(1, 1, 1), SubvoxelIndex::new(3, 3, 3).unwrap()),
            MaterialId(6)
        );
    }

    #[test]
    fn carving_every_subvoxel_back_out_returns_the_voxel_to_empty() {
        // The canonicalisation must work in reverse too, or a dug-out voxel
        // would keep costing a `Mixed` leaf forever.
        let mut t = SparseVoxelOctree::new();
        t.set_voxel(pos(2, 2, 2), VoxelShape::solid(MaterialId(6)));
        for i in SubvoxelIndex::all() {
            t.set_subvoxel(pos(2, 2, 2), i, MaterialId::EMPTY);
        }
        assert_eq!(t.voxel(pos(2, 2, 2)), VoxelShape::empty());
        assert_eq!(t.leaf_count(), 1, "the tree should be empty again");
        assert_eq!(t.solid_count(), 0);
    }

    #[test]
    fn the_digest_does_not_depend_on_insertion_order() {
        // The claim the whole canonical tree exists to support.
        let mut forward = SparseVoxelOctree::new();
        let mut backward = SparseVoxelOctree::new();
        let mut positions = Vec::new();
        for z in (0..CHUNK_SIDE as u32).step_by(3) {
            for y in (0..CHUNK_SIDE as u32).step_by(5) {
                for x in (0..CHUNK_SIDE as u32).step_by(7) {
                    positions.push((pos(x, y, z), MaterialId(3 + ((x + y + z) % 4) as u16)));
                }
            }
        }
        for (p, m) in &positions {
            forward.set_voxel(*p, VoxelShape::solid(*m));
        }
        for (p, m) in positions.iter().rev() {
            backward.set_voxel(*p, VoxelShape::solid(*m));
        }
        assert_eq!(forward.digest(), backward.digest());
    }

    #[test]
    fn the_digest_changes_when_anything_changes() {
        let a = solid_everywhere(MaterialId(6));
        let mut b = a.clone();
        assert_eq!(a.digest(), b.digest());
        b.set_subvoxel(pos(0, 0, 0), SubvoxelIndex::new(0, 0, 0).unwrap(), MaterialId(7));
        assert_ne!(a.digest(), b.digest());
    }

    #[test]
    fn sparse_and_leaf_count_agree_by_construction() {
        // Two independent measures of the same property must agree, or one of
        // them is a bug that the other one is hiding.
        let mut t = solid_everywhere(MaterialId(6));
        assert_eq!(t.is_sparse(), t.leaf_count() < CHUNK_VOLUME as u64);
        for (i, x) in (0..CHUNK_SIDE as u32).step_by(2).enumerate() {
            t.set_voxel(pos(x, 3, 3), VoxelShape::empty());
            if i % 4 == 0 {
                assert_eq!(t.is_sparse(), t.leaf_count() < CHUNK_VOLUME as u64);
            }
        }
    }

    #[test]
    fn visit_leaves_covers_the_chunk_exactly_once() {
        let mut t = SparseVoxelOctree::new();
        t.set_voxel(pos(0, 0, 0), VoxelShape::solid(MaterialId(6)));
        t.set_voxel(pos(31, 31, 31), VoxelShape::solid(MaterialId(6)));
        let mut covered = 0u64;
        let mut leaves = 0u64;
        t.visit_leaves(&mut |origin, level, node| {
            leaves += 1;
            covered += match node {
                Node::Empty => coverage(level) - coverage(level), // zero, spelled clearly
                Node::Uniform(_) => coverage(level),
                Node::Mixed(_) => {
                    assert_eq!(level, 0, "a Mixed leaf is one voxel");
                    1
                }
                Node::Branch(_) => unreachable!("visit_leaves only reports leaves"),
            };
            // The origin must be aligned to the region the node covers.
            let side = 1i32 << level;
            for c in origin {
                assert_eq!(c % side, 0, "leaf origin {origin:?} is not aligned to {side}");
            }
        });
        assert_eq!(leaves, t.leaf_count());
        assert_eq!(covered, t.solid_count());
    }

    #[test]
    fn an_out_of_range_local_coordinate_is_refused_by_name() {
        assert!(local_from([0, 0, 31]).is_ok());
        let r = local_from([32, 0, 0]).unwrap_err();
        assert_eq!(r.kind(), RefusalKind::IndexOutOfRange);
        assert!(r.detail().contains("outside 0..32"));
        assert!(local_from([-1, 0, 0]).is_err());
    }

    #[test]
    fn octant_index_and_its_inverse_agree() {
        // `visit` inverts `octant_index` by hand; if the two ever disagree,
        // every walk is silently wrong. This is the test that pins them.
        let half = 16;
        for x in 0..32 {
            for y in (0..32).step_by(7) {
                for z in (0..32).step_by(11) {
                    let i = octant_index([x, y, z], half);
                    let origin = [
                        if i & 1 != 0 { half } else { 0 },
                        if i & 2 != 0 { half } else { 0 },
                        if i & 4 != 0 { half } else { 0 },
                    ];
                    // The octant's origin differs from the coordinate by less
                    // than `half` on every axis, and the bits agree.
                    for a in 0..3 {
                        assert_eq!(origin[a] != 0, ([x, y, z][a] & half) != 0);
                    }
                    assert!([x, y, z][0] - origin[0] < half);
                }
            }
        }
    }

    // A small helper so the test above can ask which kind of node a voxel is.
    impl SparseVoxelOctree {
        fn tree_kind_at(&self, local: LocalPos) -> &'static str {
            fn descend(node: &Node, level: u32, local: [i32; 3]) -> &'static str {
                match node {
                    Node::Empty => "empty",
                    Node::Uniform(_) => "uniform",
                    Node::Mixed(_) => "mixed",
                    Node::Branch(c) => {
                        let half = 1i32 << (level - 1);
                        descend(&c[octant_index(local, half)], level - 1, local)
                    }
                }
            }
            descend(&self.root, OCTREE_LEVELS, [
                local.x as i32,
                local.y as i32,
                local.z as i32,
            ])
        }
    }
}
