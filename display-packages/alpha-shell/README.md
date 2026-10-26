alpha-shell — the shell's own display package
=============================================

The game's first display package is its own shell: the arrangement the user ruled — **six tabs**, the
console as the one bar entry that is not a tab, and three windows pinned in the HUD — carried as a
recipe rather than as literals in the client.

What it is
----------

`package.shspec` is the `shploingus.recipe/3` source. It declares **16 windows**, **6 tabs**
(`inventory`, `trees`, `mods`, `docs`, `settings`, `map-plan`), **one bar entry** (`console`), **six
filed** windows offered by a tab's menu, and **three pins** (`streamer`, `determinism`, `refusals`).
The eight tabs of UI-SPEC §5 were cut to six by the user: `governance` and `tools` left the bar and
are console-opened, which is a real state rather than a missing home (H-15.2).

`package.cells` declares the drawing (`cells 1`, the Holo Dark raster). **Changing that line to
`cells 0` brings the old drawing back** — the elements as composed, version 0 — and the line is read
where the package is mounted, so the version is the package's rather than a constant in `main.rs`. A
declaration written under the older header (`shploingus.cells/0`) is refused by file and line, printed
once, and the version in force keeps drawing. A package that arrives while the shell runs is mounted
by the same loader, so its declaration is read then too.

The source is **byte-identical** to `packages/alpha-shell.shspec` today, and `build.rs` refuses the
build if it is not: the client's mount file and this folder's source are one declaration, and a second
copy that drifted would be two answers to what the shell is.

How it is mounted
-----------------

    cargo build -p shploingus

`crates/shploingus/build.rs` compiles this folder's `package.shspec`, loads the bytes back through the
same codec, embeds them with `include_bytes!`, and fails the build by path when the file is missing or
refused. At boot the binary mounts the embedded package (so a shell exists even outside a workspace),
then **mounts this folder through `Mount::load_package`** — one path, no cache and no fallback
(`crates/voxel-client/src/shell/mount.rs`). That second mount is what reads `package.cells`; a
refused declaration leaves the embedded arrangement and the drawing in force on screen and prints one
sentence.

What is not here
----------------

* **No layers.** The shell draws through the renderer's existing pipelines; a package layer arrives
  with `horizon-lod/`, the first package that has one.
* **No rows.** A row is a record the package implements, and the shell's work is filed in `patches/`
  and the ledger rather than restated here. A package that copied those rows would be a second copy of
  the record.
* **No `claims.shpkg`.** The axis package's claims/pin/reproduce file is M10.3's; nothing writes one
  today and a placeholder would be a claim nobody made.
