//! The canonical sparse voxel octree.
//!
//! A tree spans `2^levels` voxels per edge, and **the span is the tree's own field**
//! ([`SparseVoxelOctree::with_levels`]), not a constant baked into the arithmetic. A chunk is
//! `CHUNK_SIDE` = 32 = 2⁵ voxels per edge, so [`OCTREE_LEVELS`] = 5 is the *world's* tree and
//! [`SparseVoxelOctree::new`] still makes exactly that — byte for byte the tree it always was.
//! A world-scale system on a coarser lattice (a cloud cell, a terrain block at a different
//! quantum) asks for its own level count and gets the same nodes, the same canonicalisation, the
//! same walk and the same merge rule over a different domain.
//!
//! **The span is not the feature size and must not be confused with it.** `levels` fixes how many
//! voxels the root covers; how many *metres* a voxel is belongs to the caller, which is what lets
//! one tree serve a 32 m chunk of terrain and a 16 km cube of atmosphere without either of them
//! carrying a constant the other has to know about.
//!
//! A node at level `L` covers `2^L` voxels per axis, so level 0 covers one voxel — the tree's own
//! finest unit — and is a **leaf**:
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
    CHUNK_SIDE, SUBVOXEL_SIDE,
};

/// Levels in a **chunk's** tree — the world's own span. A chunk is `2^5` voxels per edge, so
/// there are five.
///
/// It is the *default* [`SparseVoxelOctree::new`] uses, not a limit on the type: see
/// [`SparseVoxelOctree::with_levels`] for a tree over any `2^levels` span.
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
    /// How many levels the tree has: the root covers `2^levels` voxels per edge. A field rather
    /// than [`OCTREE_LEVELS`], because the tree's *span* is the caller's business (see the module
    /// docs) — the same nodes serve a 32 m chunk and a 16 km cube of sky.
    levels: u32,
}

impl Default for SparseVoxelOctree {
    fn default() -> Self {
        Self::new()
    }
}

impl SparseVoxelOctree {
    /// A chunk of nothing — [`OCTREE_LEVELS`] levels, which is the world's own span.
    pub const fn new() -> Self {
        SparseVoxelOctree { root: Node::Empty, levels: OCTREE_LEVELS }
    }

    /// **A tree of nothing over `2^levels` voxels per edge** — the scale-generic constructor.
    ///
    /// The level count is the tree's span and nothing else: a caller drawing a system whose
    /// lattice is not the world's 32 m chunk (a cloud cell at a kilometre's scale, a terrain
    /// block at twice the voxel) asks for the levels its own domain needs and gets the same
    /// canonical tree, the same walks and the same merge rule. What a voxel *is* — a metre, a
    /// rung of a ladder — is the caller's, and stays the caller's.
    pub const fn with_levels(levels: u32) -> Self {
        SparseVoxelOctree { root: Node::Empty, levels }
    }

    /// A tree that already holds `root`, over `2^levels` voxels per edge.
    ///
    /// The caller states the contents: a root that is not canonical (eight identical children
    /// that should have collapsed) is stored as given, so this is for a builder that constructs
    /// canonical nodes itself — see [`Self::set_material_region`] for the writing path that keeps
    /// the guarantee.
    pub const fn from_root(root: Node, levels: u32) -> Self {
        SparseVoxelOctree { root, levels }
    }

    /// How many levels the tree has: the root covers `2^levels` voxels per edge.
    pub const fn levels(&self) -> u32 {
        self.levels
    }

    /// How many voxels the whole tree covers: `(2^levels)³`. For a chunk's tree this is
    /// `voxel_model::CHUNK_VOLUME`.
    pub fn voxels(&self) -> u64 {
        1u64 << (3 * self.levels)
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
        set_at(&mut self.root, self.levels, [x, y, z], shape);
    }

    /// **Fill a whole region with one material**, in one write.
    ///
    /// `origin` is the region's low corner in voxels and `level` says how many voxels a side it
    /// covers (`2^level`). Filling a region cell by cell through [`Self::set_voxel`] costs `8^level`
    /// writes, which is why a tree whose *features* are large has to be able to say "this whole box
    /// is one material" in one go: a 16 km cube of sky asked for at 11 levels would otherwise be
    /// eight million `set_voxel` calls to say "empty".
    ///
    /// A region carries a **material, not a shape**, because a 4×4×4 [`VoxelShape`] is one voxel by
    /// construction and a shape written over a coarser region would be claiming detail that region
    /// cannot hold. [`MaterialId::EMPTY`] clears the region, the same way an empty shape clears a
    /// voxel. Nothing is canonicalised differently by this path: eight identical children collapse
    /// on the way back up exactly as they do for a single voxel.
    pub fn set_material_region(&mut self, origin: [i32; 3], level: u32, material: MaterialId) {
        if level > self.levels {
            return;
        }
        let replacement = if material.is_empty() {
            Node::Empty
        } else {
            Node::Uniform(material)
        };
        set_node_at(&mut self.root, self.levels, origin, level, replacement);
    }

    /// **The node covering a region** of `2^level` voxels at `origin`.
    ///
    /// The counterpart [`Self::set_material_region`] needs: a caller meshing a coarse tree asks
    /// *"what is on the other side of this face?"* and gets that region's own node. A **leaf**
    /// answer covers the whole region uniformly (its material, or nothing), which is exactly what a
    /// face test wants; a [`Node::Branch`] answer means the region is subdivided, and therefore
    /// that its contents are known only a level down.
    ///
    /// Outside the tree — a level above the root, or a region off its edge — is [`Node::Empty`],
    /// which is the honest answer for a tree that only holds what it holds.
    pub fn region(&self, origin: [i32; 3], level: u32) -> &Node {
        if level > self.levels {
            return &Node::Empty;
        }
        let mut node = &self.root;
        let mut half = if self.levels == 0 { 0 } else { 1i32 << (self.levels - 1) };
        for _ in 0..(self.levels - level) {
            match node {
                Node::Branch(children) => {
                    node = &children[octant_index(origin, half)];
                    half >>= 1;
                }
                // A leaf covers every region below it — including this one, whatever it is.
                leaf => return leaf,
            }
        }
        node
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
            self.levels,
            [local.x as i32, local.y as i32, local.z as i32],
        )
    }

    /// One subvoxel's material, without materialising the whole shape.
    pub fn material_at(&self, local: LocalPos, at: SubvoxelIndex) -> MaterialId {
        material_at(
            &self.root,
            self.levels,
            [local.x as i32, local.y as i32, local.z as i32],
            at,
        )
    }

    /// Visit every **leaf** node with the voxel-space origin of the region it
    /// covers and its level. Branches are walked, never reported, which is the
    /// whole point of a recursive visitor over a tree this shape.
    pub fn visit_leaves(&self, f: &mut impl FnMut([i32; 3], u32, &Node)) {
        visit(&self.root, self.levels, [0, 0, 0], f);
    }

    /// Visit every region of **exactly `level`**, whether or not the tree
    /// compresses at that level.
    ///
    /// [`Self::visit_leaves`] reports what the tree *happened* to merge, so its
    /// output depends on the contents' compressibility: a chunk of solid rock
    /// yields one leaf and a chunk of rubble yields thousands. That is the right
    /// answer to "how is this stored" and the **wrong** answer to "what are the
    /// units of work", because a scheduler that planned work from it would give
    /// two chunks of the same world different tick plans — which is F12's exact
    /// failure, arrived at from the other direction.
    ///
    /// So this walks to `level` and stops there, descending through branches and
    /// **splitting coarse leaves back down**. A `Uniform` node at level 4 asked
    /// for at level 2 is reported four times, once per level-2 cell it covers, and
    /// the node is the same one every time. The region count is therefore a
    /// function of `level` and the chunk's size alone.
    ///
    /// A `level` above the tree's own [`Self::levels`] visits **nothing** rather
    /// than refusing: there is no such region, and an empty answer is not a wrong
    /// one. Callers that need a refusal ask [`Self::leaf_count_at`], which reports
    /// zero for the same reason and says so.
    pub fn visit_at_level(&self, level: u32, f: &mut impl FnMut([i32; 3], &Node)) {
        if level > self.levels {
            return;
        }
        visit_at(&self.root, self.levels, [0, 0, 0], level, f);
    }

    /// How many regions [`Self::visit_at_level`] would report at `level`.
    ///
    /// Equal to `(2^(levels - level))³` — `(CHUNK_SIDE >> level)³` for a chunk's
    /// own five-level tree — and zero above the tree's root. Stated as its own
    /// function because "the walk reports the right number" is a claim worth
    /// holding rather than assuming.
    pub fn leaf_count_at(&self, level: u32) -> u64 {
        if level > self.levels {
            return 0;
        }
        let per_axis = 1u64 << (self.levels - level);
        per_axis * per_axis * per_axis
    }

    /// How many voxels hold matter.
    pub fn solid_count(&self) -> u64 {
        count_solid(&self.root, self.levels)
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
    /// Exactly equivalent to `leaf_count() < voxels()`, because leaves partition
    /// the tree's voxels and every leaf covers a whole number of them. Stated as
    /// an equality of two independent measures in the tests. ([`Self::voxels`] is
    /// `CHUNK_VOLUME` for a chunk's own five-level tree.)
    pub fn is_sparse(&self) -> bool {
        self.leaf_count() < self.voxels()
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
        // **The span is part of the contents.** The same nodes at one more level cover twice the
        // ground, so a digest that ignored the span would call two different trees equal. The
        // chunk's own five levels write **nothing**: every digest ever taken over a chunk is
        // byte-for-byte what it was before the span became a field, so nothing stored needs
        // re-deriving. A tree at any other span is prefixed with it, and no node tag is `4` or
        // above, so the two forms cannot collide.
        if self.levels != OCTREE_LEVELS {
            transcript.extend_from_slice(&self.levels.to_le_bytes());
        }
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

/// Descend to the region at `target` and put `replacement` there.
///
/// [`set_at`] is this function with `target = 0` and a shape for its node: the recursion, the
/// expansion of a leaf on the way down and the canonicalisation on the way back up are the same
/// code, so a region write and a voxel write cannot drift apart.
fn set_node_at(node: &mut Node, level: u32, local: [i32; 3], target: u32, replacement: Node) {
    if level == target {
        *node = replacement;
        return;
    }
    if level == 0 {
        // `target` is below a leaf's own level, which a canonical tree cannot ask for.
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

    set_node_at(&mut children[index], level - 1, local, target, replacement);
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

/// Walk to `target` and report the node covering each region of that size.
///
/// Two cases the leaf walk does not have. A **branch** above the target is
/// descended into, so the regions come out of the children. A **leaf** above the
/// target is *split*, because it covers several target-sized regions and all of
/// them are that node — reporting it once would hand the caller a region eight
/// times too large and call it exact.
fn visit_at(node: &Node, level: u32, origin: [i32; 3], target: u32, f: &mut impl FnMut([i32; 3], &Node)) {
    if level == target {
        f(origin, node);
        return;
    }
    match node {
        Node::Branch(children) => {
            let half = 1i32 << (level - 1);
            for (i, child) in children.iter().enumerate() {
                let child_origin = [
                    origin[0] + if i & 1 != 0 { half } else { 0 },
                    origin[1] + if i & 2 != 0 { half } else { 0 },
                    origin[2] + if i & 4 != 0 { half } else { 0 },
                ];
                visit_at(child, level - 1, child_origin, target, f);
            }
        }
        leaf => {
            let side = 1i32 << target;
            let n = 1i32 << (level - target);
            for dz in 0..n {
                for dy in 0..n {
                    for dx in 0..n {
                        f([origin[0] + dx * side, origin[1] + dy * side, origin[2] + dz * side], leaf);
                    }
                }
            }
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
    use voxel_model::CHUNK_VOLUME;

    #[test]
    fn a_trees_span_is_its_own_and_the_chunk_default_is_unchanged() {
        // **The scale-generic claim.** A tree of 11 levels spans 2,048 voxels per edge and covers
        // eight billion of them — a domain four hundred and fifty times a chunk's — and **one
        // region write** fills a 512-voxel box of it, which is the operation a coarse lattice needs
        // and a per-voxel `set_voxel` cannot provide (it would be 134 million calls). The chunk's
        // own tree is checked last, against the numbers it has always reported, because making the
        // span a field must not move the world.
        let mut sky = SparseVoxelOctree::with_levels(11);
        assert_eq!(sky.levels(), 11);
        assert_eq!(sky.voxels(), 2048 * 2048 * 2048);
        assert_eq!(sky.leaf_count_at(11), 1, "the root is one region");
        assert_eq!(sky.leaf_count_at(9), 64, "level 9 is 4 regions a side");
        assert_eq!(sky.leaf_count_at(0), 2048u64.pow(3), "level 0 is one region a voxel");
        assert_eq!(sky.leaf_count_at(12), 0, "above the root there is no region");

        let region = 512u64.pow(3);
        sky.set_material_region([0, 0, 0], 9, MaterialId(7));
        assert_eq!(sky.solid_count(), region, "the region is not 2⁹ voxels a side");
        assert_eq!(
            sky.region([0, 0, 0], 9),
            &Node::Uniform(MaterialId(7)),
            "the region does not read back as what was written"
        );
        assert_eq!(
            sky.region([0, 0, 0], 3),
            &Node::Uniform(MaterialId(7)),
            "a leaf must cover every region below it, not only its own level"
        );
        assert!(matches!(sky.region([0, 0, 0], 10), Node::Branch(_)), "level 10 is subdivided: it is not one region");
        assert_eq!(
            sky.region([512, 0, 0], 9),
            &Node::Empty,
            "the box next door was filled by a write to its neighbour"
        );
        assert_eq!(
            sky.region([0, 0, 0], 12),
            &Node::Empty,
            "a level above the root is not empty — it does not exist"
        );

        // A second region write subdivides, and the two read back independently.
        sky.set_material_region([512, 0, 0], 9, MaterialId(8));
        assert_eq!(sky.solid_count(), 2 * region);
        assert_eq!(sky.region([0, 0, 0], 9), &Node::Uniform(MaterialId(7)));
        assert_eq!(sky.region([512, 0, 0], 9), &Node::Uniform(MaterialId(8)));
        // Clearing a region is a write too, and it gives the space back.
        sky.set_material_region([512, 0, 0], 9, MaterialId::EMPTY);
        assert_eq!(sky.solid_count(), region);
        assert_eq!(sky.region([512, 0, 0], 9), &Node::Empty);

        // **And the chunk's own tree is exactly what it was.** Same span, same counts, same
        // regions, and a digest a chunk built before the span was a field would still match.
        let chunk = solid_everywhere(MaterialId(1));
        assert_eq!(chunk.levels(), OCTREE_LEVELS);
        assert_eq!(chunk.voxels(), CHUNK_VOLUME as u64);
        assert_eq!(chunk.leaf_count_at(OCTREE_LEVELS), 1);
        assert_eq!(chunk.region([0, 0, 0], OCTREE_LEVELS), &Node::Uniform(MaterialId(1)));
        assert_eq!(chunk.digest(), solid_everywhere(MaterialId(1)).digest());
        assert_ne!(
            chunk.digest(),
            SparseVoxelOctree::with_levels(OCTREE_LEVELS + 1).digest(),
            "two spans digested alike"
        );
        // The same *nodes* at a different span are a different tree, and the digest says so.
        let same_nodes_bigger = SparseVoxelOctree::from_root(chunk.root().clone(), OCTREE_LEVELS + 1);
        assert_ne!(chunk.digest(), same_nodes_bigger.digest());
    }

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
    fn the_region_count_is_the_same_however_well_the_chunk_compresses() {
        // The claim `visit_at_level` exists for. A solid chunk is **one** leaf and
        // a chunk with one odd voxel in it is thousands; if the unit-of-work walk
        // followed compression, those two chunks would be scheduled differently
        // for the same world.
        let solid = solid_everywhere(MaterialId(1));
        let mut speckled = solid_everywhere(MaterialId(2));
        speckled.set_voxel(pos(7, 9, 11), VoxelShape::solid(MaterialId(3)));

        assert_ne!(
            solid.leaf_count(),
            speckled.leaf_count(),
            "the two trees do not compress differently, so this test proves nothing"
        );

        for level in 0..=OCTREE_LEVELS {
            let mut a = 0u64;
            solid.visit_at_level(level, &mut |_, _| a += 1);
            let mut b = 0u64;
            speckled.visit_at_level(level, &mut |_, _| b += 1);
            assert_eq!(a, speckled.leaf_count_at(level), "solid chunk, level {level}");
            assert_eq!(b, speckled.leaf_count_at(level), "speckled chunk, level {level}");
            assert_eq!(a, b, "compression changed the region count at level {level}");
        }

        // And the count is the one arithmetic says it is, not merely equal.
        assert_eq!(speckled.leaf_count_at(OCTREE_LEVELS), 1);
        assert_eq!(speckled.leaf_count_at(OCTREE_LEVELS - 1), 8);
        assert_eq!(speckled.leaf_count_at(OCTREE_LEVELS - 2), 64);
        // Above the root there is no such region, and that is zero rather than a
        // refusal.
        assert_eq!(speckled.leaf_count_at(OCTREE_LEVELS + 1), 0);
        let mut none = 0u64;
        speckled.visit_at_level(OCTREE_LEVELS + 1, &mut |_, _| none += 1);
        assert_eq!(none, 0);
    }

    #[test]
    fn regions_at_a_level_partition_the_chunk_exactly_once_each() {
        // Every voxel belongs to exactly one reported region: no gap, no overlap.
        // A hole or a double-count here is what a tick stage would read as a
        // duplicated or missing voxel.
        let mut t = solid_everywhere(MaterialId(1));
        t.set_voxel(pos(0, 0, 0), VoxelShape::solid(MaterialId(2)));
        t.set_voxel(pos(31, 31, 31), VoxelShape::solid(MaterialId(3)));
        t.set_voxel(pos(16, 0, 16), VoxelShape::solid(MaterialId(4)));

        for level in 0..=OCTREE_LEVELS {
            let side = 1i32 << level;
            let mut covered = vec![0u8; CHUNK_VOLUME as usize];
            t.visit_at_level(level, &mut |origin, _| {
                for dz in 0..side {
                    for dy in 0..side {
                        for dx in 0..side {
                            let v = [origin[0] + dx, origin[1] + dy, origin[2] + dz];
                            covered[(v[2] * CHUNK_SIDE + v[1]) as usize
                                * CHUNK_SIDE as usize
                                + v[0] as usize] += 1;
                        }
                    }
                }
            });
            let wrong = covered.iter().position(|c| *c != 1);
            assert_eq!(wrong, None, "level {level}: voxel {wrong:?} was covered {} times", covered[wrong.unwrap_or(0)]);
        }
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
