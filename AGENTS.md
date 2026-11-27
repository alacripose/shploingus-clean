

## 1. Formats: upgraded and legacy

This project carries formats at their own revisions, and the audit tools inventory them rather than
trusting a summary: `scan_drives_reaudit.py` in `secure_shell_v2` reads dotted schema identifiers and
package magics across every project, and its register is checked rather than described.

Read here on 2026-11-04 under `C:\Users\ala\Documents\shploingus-clean` (head `6049414`):

- identifiers: `shploingus.recipe/3`x62, `shploingus.cellframe/1`x16, `doi.org/10`x16, `shploingus.ballot/1`x14, `shploingus.layer/1`x11, `shploingus.cells/0`x10, `shploingus.compendium/1`x7, `shploingus.cells/1`x7, `shploingus.receipt/2`x3, `shploingus.recipe/1`x2, `shploingus.api-answer/1`x2, `shploingus.api-field-values/1`x2
- magics: `SHPKG002`x1, `SHPKG003`x1

Upgraded versus legacy, by family:

  - `shploingus.cellframe`: newest `shploingus.cellframe/1`, older `shploingus.cellframe/0`
  - `shploingus.cells`: newest `shploingus.cells/1`, older `shploingus.cells/0`
  - `shploingus.receipt`: newest `shploingus.receipt/2`, older `shploingus.receipt/0`, `shploingus.receipt/1`
  - `shploingus.recipe`: newest `shploingus.recipe/3`, older `shploingus.recipe/1`, `shploingus.recipe/2`
  - `shploingus.save`: newest `shploingus.save/2`, older `shploingus.save/1`

A legacy value is evidence to read, never a value to convert silently: an older frame is refused by
name, and `dl-enclave` §1 and §3 own that rule for this family.

## 2. Status to the shell connection

This project's connection state, from commands on 2026-11-04, not from its own README:

| Question | Answer |
|---|---|
| Does a running unit, admitted package or drill step name this project? | `absent` |
| Evidence | no admitted package, running unit or drill step names this project |
| Layers this project keeps | `.scratch`, `packages`, `docs` |

`held` here means the named command produced the named result. `absent` means nothing proved it yet;
it is not a judgement about the project's quality, and it changes only when a command changes it.

## 3. Resynthesizing this file

This section was appended by `secure_shell_v2/tools/resynth_agents.py`, which refuses
`agents-authority-trimmed` rather than write a file that lost a prior line. A resynthesis of this
project adds: its formats at their revisions, its connection state, and its conventions as the file
already states them. It does not delete, re-word or "tidy" an existing rule; a stale claim is
superseded by a dated line beside it.

## 2026-11-20 — agents files stay readable, and the judge reads the archive

**Owner instruction, verbatim:** *"agents in every project should be easy as the user says what they
want and 'read agents in full' and it gives about however many lines of text are in the agent file in
the repo, but all of this is bad: and the shorthand shit is awfull that that is an abhorrent affrontt
to language"*.

- **An agents file is read in full.** Its job is to be understood, so a rule keeps its reasoning, its
  measured numbers and its phrasing. A rule cut down to a bold lead and a pointer is refused by name,
  `shorthand-compaction`, and the refusal is recorded rather than argued.
- **The evidence is not deleted, it is classified.** `docs/history/agents.md.jsonl` keeps every dated
  section with its digest. `artifacts/agents-judge.json` carries the same rows on the Axis ladders,
  so the Debian governance stack grades the record without a model re-reading the prose.
- **The ladders are named, never scored.** `evidence_grade` is A measured, B receipted, C asserted,
  D intended. `gate_status` is current or refuted. `evidence_origin` is owner, read-back or
  diagnostic. No percentage, no average.
- **The archive never replaces the file.** Reading `AGENTS.md` end to end stays the first step of
  every session, and the archive is what a judge reads after that.
- **A refusal about prose is a refusal about the harness.** A file that needs a decoder has already
  failed the reader it was written for.

## 2026-11-20 — evidence is superseded, and every gap is a ticket

**Owner instruction, verbatim:** *"Agents bloated with prior evidence work despite multiple turns
outlining the security system allowing for supersession"*.

- **A later verdict replaces an earlier one.** When a section names a refusal code that an earlier
  section also named, the earlier verdict is superseded. The archive carries `superseded_by`, and
  the judge reads the current row instead of the oldest claim it can find.
- **Nothing is deleted to achieve that.** The file keeps its prose and the archive keeps every row.
  Currency is computed, never achieved by cutting the record.
- **A "Not checked" line is a ticket, not an apology.** It carries an id in
  `artifacts/agents-judge.json` under `open_limits`. It is closed by a later measured section that
  names the id, and it is never closed by silence.
- **A repeated gap is a refusal.** A limit that survives several turns without a closing attempt is
  `limit-stale`, and the work to close it is the ticket it already has.
- **Reading the file is not reading the evidence.** The first read is the rules. The second read is
  the judge export, which states what is current, what is superseded and what is still open.
