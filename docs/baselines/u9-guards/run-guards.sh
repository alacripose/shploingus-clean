#!/usr/bin/env bash
# U9 document form (§28) — the three guard proofs.
#
# One perturbation at a time, each a single declared value changed, each run through the
# product's own evidence (`cargo run --offline`), then restored from the byte-exact copy in
# this directory and verified by digest. Nothing is left perturbed.
set -u
ROOM=/c/Users/ala/Documents/shploingus-clean/clean-room
GUARD=/c/Users/ala/Documents/shploingus-clean/docs/baselines/u9-guards

perturb() { # name, file, old, new, source-of-copy
  local name="$1" file="$2" old="$3" new="$4" copy="$5"
  echo "=================== $name ==================="
  [ -f "$ROOM/$file" ] || { echo "NO SUCH FILE: $ROOM/$file"; return 1; }
  [ -f "$GUARD/$copy" ] || { echo "NO SUCH COPY: $GUARD/$copy"; return 1; }
  python - "$ROOM/$file" "$old" "$new" <<'PY'
import sys
path, old, new = sys.argv[1], sys.argv[2], sys.argv[3]
text = open(path, encoding='utf-8', newline='').read()
assert text.count(old) == 1, f"expected exactly one occurrence of {old!r}, found {text.count(old)}"
open(path, 'w', encoding='utf-8', newline='').write(text.replace(old, new))
PY
  ( cd "$ROOM" && cargo build --offline >/dev/null 2>&1 )
  "$ROOM/target/debug/shploingus-clean-room.exe" > "$GUARD/$name.out" 2> "$GUARD/$name.err"
  echo "exit=$?"
  echo "--- stderr ---"; cat "$GUARD/$name.err"
  echo "--- the lines that moved ---"
  grep -E "^U9 (document|streams|save|could)|^Clean room|NOT OK|each by its own name|receipt verifies|each the world's own" "$GUARD/$name.out" || true
  cp "$GUARD/$copy" "$ROOM/$file"
  echo "--- restored: $(sha256sum "$ROOM/$file" | cut -d' ' -f1)"
  echo "--- expected: $(grep " \*$copy$" "$GUARD/digests.txt" | cut -d' ' -f1)"
}

perturb p1-a-seed  src/save/mod.rs \
  'seed: world.stream_seed(*domain),' \
  'seed: world.stream_seed(*domain) ^ 1,' \
  mod.rs

perturb p2-a-digest src/intake.rs \
  'if found != source.digest() {' \
  'if false {' \
  intake.rs

perturb p3-a-record-field src/bridge/gate.rs \
  'record_len: self.chain.len(),' \
  'record_len: self.chain.len() + 1,' \
  gate.rs

echo "=================== all three restored ==================="
cd "$ROOM" && sha256sum src/save/mod.rs src/intake.rs src/bridge/gate.rs
