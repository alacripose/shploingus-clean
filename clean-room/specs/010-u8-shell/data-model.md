# U8 data model — the declared types

> **Derived from** `docs/UI-SPEC.md` §1–§4 and §8 and `docs/SHELL-PORT.md` §4. Every type here is data;
> the shell owns the state. Nothing has a `Default` that invents a value (P2).

## The design system

```
Bucket      ::= Md1Rule | Md1Example | PlatformTranslation | ProjectExtension      // four; ALL pins 4
Constant    { name: &'static str, value: f64, bucket: Bucket, source: &'static str }
CONSTANTS   : &[Constant]                                                          // 16 rows
constant(name) -> Result<f64, Refusal>                                             // or constant_unadmitted
bucket_counts() -> [usize; 4]                                                      // [8, 2, 1, 5]
```

`source` is not optional: a row with no document behind it is not a row. The table holds, in order, the
ink ladder's four steps, the contrast floor, the corner radius, the touch target and the separation
(MD1 rules); the dense-pointer pair (MD1 example); the character cell (platform translation); the panel
opacity, the elevation step and its ceiling, the pane floor and the bar height (project extensions).

## The declaration

```
Lane        ::= Frame | Control | Tab | World | Beacon | Motion                           // six; ALL pins 6
Content     ::= Registry | World | Materials | Stages | Geometry | Light
WindowId    ::= a &'static str, the window's stable identity (never a list position)
WindowSpec  { id: WindowId, title: &'static str, lane: Lane, content: Content, pinnable: bool }
Recipe      { windows: Vec<WindowSpec> }      Recipe::declared() -> the six-window declaration
```

`Content::lines() -> Result<Vec<String>, Refusal>` is the one reader per content kind; each variant calls
a system that already exists (`systems`, `reference`+`world`, `reference`, `process`, `mesh`, `light`).

## The mount

```
PaneId      ::= u64                                    // a pane's own identity
Orientation ::= Horizontal | Vertical                  // side by side | stacked
Node        ::= Pane(PaneId) | Split { orientation, a: Box<Node>, b: Box<Node> }
Layer       ::= Shell | Hud                            // two layers of ONE window set
PointerOwner::= Shell | World
Pointer     { owner: PointerOwner, visible: bool }     // visible is a consequence of the same read
Mount       { recipe, root: Option<Node>, windows: BTreeMap<PaneId, WindowId>,
              focus: Option<PaneId>, layer, grab, next_pane }
```

**Invariants** (each held by a test):

1. `Mount::windows()` is exactly `recipe.windows()` — the mount adds nothing.
2. `panes()` is the tree in order; every declared window has at least one pane at mount time.
3. `split` raises the pane count by one and the new pane holds the **same** window as its target.
4. `close` lowers it by one and drops the pane's window binding with it.
5. `visible_panes()` is all panes in the shell layer, and only the `pinnable` windows' panes in the HUD.
6. `read_pointer()` is pure: two reads of the same state are equal, and `visible == (owner == Shell)`.
7. `layout` never emits a visible pane below the pane floor; below it, it refuses.
8. The frame is a pure function of `(mount state, area, pointer)` — the same state draws the same bytes.

## The frame

```
Area    { row, col, rows, cols }                       // a rectangle of the character grid
Canvas  { rows, cols, cells: Vec<char> }               // render() -> one padded line per row
frame(&Mount, Area, Pointer) -> Result<String, Refusal>
declared_area() -> Area                                // the 100 × 30 extent the checks share
frame_digest(&str) -> Digest                           // what was drawn, not that something was
```
