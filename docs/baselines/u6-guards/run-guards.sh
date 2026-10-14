#!/usr/bin/env bash
# §29 — U6's policy chokepoint: every new guard perturbed **one value at a time**, observed to fail
# the slice's own bar, and restored **byte-exact** from a copy, with the digests recorded.
#
# The substitutions are anchored to a line number **and** the file is restored from the copy before
# every case, so each perturbation edits the file's final bytes at the line named. A perturbation
# that does not change the digest is reported as NOT APPLIED rather than counted as a proof.
#
# Run from anywhere:  bash docs/baselines/u6-guards/run-guards.sh
set -u

here="$(cd "$(dirname "$0")" && pwd -P)"
crate="$here/../../../clean-room"
cd "$crate" || exit 1

sha() { sha256sum "$1" | cut -d' ' -f1; }

FILES=(src/policy.rs src/bridge/table.rs src/bridge/gate.rs)
: > "$here/digests.txt"
for f in "${FILES[@]}"; do
  cp "$f" "$here/$(echo "$f" | tr '/' '-').orig"
  printf '%s  %s\n' "$(sha "$f")" "$f" >> "$here/digests.txt"
done

: > "$here/proof.txt"

case_run() { # name  file  line  perl-substitution  subject
  local name="$1" file="$2" line="$3" sub="$4" subject="$5"
  local orig="$here/$(echo "$file" | tr '/' '-').orig"
  cp "$orig" "$file"
  local before perturbed restored applied
  before="$(sha "$file")"
  perl -i -pe "$sub if \$. == $line" "$file"
  perturbed="$(sha "$file")"
  applied=$([ "$before" != "$perturbed" ] && echo true || echo false)

  cargo run --offline > "$here/$name.out" 2> "$here/$name.err"
  local run_status=$?
  cargo test --offline > "$here/$name.test.log" 2>&1
  local test_status=$?

  cp "$orig" "$file"
  restored="$(sha "$file")"

  {
    echo "=== $name — subject: $subject"
    echo "    file:         $file (line $line)"
    echo "    applied:      $applied"
    echo "    digest:       before    $before"
    echo "                  perturbed $perturbed"
    echo "                  restored  $restored   (byte-exact = $([ "$before" = "$restored" ] && echo true || echo false))"
    echo "    cargo run --offline   exit = $run_status"
    grep -E "Clean room FAILED|U6 policy: the reader was served|U6 policy construction: |U6 policy: [0-9]+ act\(s\)|U6 policy: an actor holding|U6 policy: the reference" "$here/$name.out" | head -4 | sed 's/^/      | /'
    echo "    cargo test --offline  exit = $test_status"
    grep -E "^test .* FAILED$" "$here/$name.test.log" | head -4 | sed 's/^/      ! /'
    echo
  } >> "$here/proof.txt"
}

# 1 — the matrix's class of a gated act (the closed vocabulary's own entry).
case_run p1-matrix-class src/policy.rs 336 \
  's/PermissionClass::Gated\(Scope::Record\),/PermissionClass::AlwaysAllowed,/' \
  'the matrix: `append_event` declared AlwaysAllowed'

# 2 — the act is the **row's own declaration** (the row that reaches it names it).
case_run p2-row-declares-act src/bridge/table.rs 331 \
  's/crate::policy::Action::AppendEvent,/crate::policy::Action::ReadWorld,/' \
  'the `geometry` row declaring `read_world` instead of `append_event`'

# 3 — the one call site, asking about the act the row declares.
case_run p3-one-call-site src/bridge/gate.rs 204 \
  's/request\.tool\.action/crate::policy::Action::ReadWorld/' \
  'the gate consulting the chokepoint about another act than the row declares'

# 4 — a denial is **recorded** (a refusal nobody can read is a refusal that did not happen).
case_run p4-denial-recorded src/bridge/gate.rs 205 \
  's/self\.record_refusal\(request\.canonical\(\), &denial\)\?;//' \
  'the denial path returning without writing the refusal to the record'

# 5 — grants refused construction, each under **its own** name.
case_run p5-grant-names src/policy.rs 128 \
  's/RefusalKind::GrantUnattributed,/RefusalKind::GrantUnscoped,/' \
  'a grant with no granter refused as `grant_unscoped` rather than `grant_unattributed`'

# 6 — a rejection is not a denial (`never_store`).
case_run p6-rejection-is-not-a-denial src/policy.rs 348 \
  's/PermissionClass::NeverStore,/PermissionClass::AlwaysDenied("no"),/' \
  '`persist_material` declared AlwaysDenied rather than NeverStore'

# 7 — every row of the matrix describes itself (a row a reader cannot understand is refused).
case_run p7-row-readable src/policy.rs 313 \
  's/=> "showing a refusal to a player",/=> "",/' \
  'an act in the matrix that describes nothing'

echo "wrote $here/proof.txt"
grep -c "^=== " "$here/proof.txt"
