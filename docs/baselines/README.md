# Baselines

**A baseline is a number a change must beat or keep, written down with the frame it came from.** The
axis ratchet reads these files: a claim that a change improved the renderer is checked against the
baseline row it names, and a change that moves a row without saying so is refused.

A baseline is not a test. A test asserts a law; a baseline records a measurement that is allowed to
move — deliberately, in its own change, with the reason beside it.

## Format

One directory per package or recipe, one file per measured thing, `.json`:

```json
{
  "what": "light-horizon-coverage",
  "why": "A chunk in the fine cube that has not been stamped is not drawn, so the slot count is the frame's floor.",
  "package": "voxel-client",
  "recipe": "shploingus on the shipped world, eye pinned, ten seconds",
  "measured": "2026-10-26",
  "commit": "6049414",
  "rows": [
    { "name": "fine slots lit after 8 s", "value": 27, "of": 27, "must": "equal" },
    { "name": "drawn chunks refused as unlit", "value": 0, "of": 605, "must": "equal" },
    { "name": "quads in the frame", "value": 60759, "must": "within 5%" },
    { "name": "mesh bytes", "value": 193.9, "of": 256.0, "must": "at most", "unit": "MB" }
  ],
  "unstable": [
    { "name": "fps", "note": "92 to 112 fps, then 38 to 64 fps, same build and same pinned camera: this machine's readings swing by 2x under load, so fps is a note and not a row. A perf claim needs GPU timestamps." }
  ]
}
```

`must` is `equal`, `at most`, `at least`, `within <n>%`, or `note`. A row with no `must` is not a
baseline row: it is a measurement looking for a law.

## What belongs here

* counts the renderer must not lose (slots lit, chunks drawn, rungs present),
* budgets it must not cross (mesh bytes, band memory),
* anything a reviewer would otherwise have to trust a screenshot for.

## What does not

* a law (`every rung hands over between four and one pixels`): that is a test,
* a picture: that is a golden image, and it lives beside the test that renders it,
* a number nobody will check.

## The golden-image rule

A frame held against a stored image is a **test**, not a baseline: it is regenerated deliberately, in
its own change, with the diff of the image beside it. The CPU pixel-parity assertions the shader used
to be held to are being replaced by these — the shader is judged by what it draws, not by whether a
second implementation of the forward path agrees with it on a probe grid.
