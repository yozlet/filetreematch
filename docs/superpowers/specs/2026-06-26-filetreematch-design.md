# filetreematch Design Spec

**Date:** 2026-06-26  
**Status:** Approved (brainstorming)  
**Goal:** Identify entire folder trees that can be safely deleted from a large inherited archive, minimizing data that needs to be backed up and sorted.

---

## Problem

Large inherited hard drives contain nested copies of data from many sources. Existing duplicate-file tools produce overwhelming lists of individual files. filetreematch takes a different approach: compare folders by contents (file names and sizes, no content hashing in v1), find **entire file trees** that are redundant, and report matches from the highest points downward.

**Scale:** ~2TB on an external spinning-platter drive. Scanning runs against the external drive; SQLite cache lives on local SSD.

---

## Requirements Summary

| Requirement | Decision |
|-------------|----------|
| Matching criteria | Relative path + file size (no hashing in v1) |
| Relationship type | **Subset:** A ⊆ B when every file in A exists at the same relative path with the same size in B |
| Delete safety | A ⊆ B means A can be safely deleted without data loss |
| Keeper selection | Report all subset relationships; user decides what to delete |
| Output granularity | **Highest points by default** (maximal pairs only); `--full-detail` for all pairs |
| Results UI | Interactive TUI (ratatui) |
| Cache | SQLite DB on local SSD, queryable by other tools |
| File matching | Files only; empty directories ignored for subset checks |
| Ignore rules | Configurable ignore list with sensible defaults |
| TUI workflow | Browse + annotate (keep / delete candidate / notes) + export delete script |
| Rescan strategy | Incremental (fingerprint-based subtree skip) |
| Similarity matching | **Phase 2** — exact subset only in v1 |

---

## Architecture

```
┌─────────────────┐     walk (parallel)      ┌──────────────────┐
│  External HD    │ ────────────────────────▶│  Scan Engine     │
│  (~2TB)         │     respect ignore rules │  (Rust)          │
└─────────────────┘                          └────────┬─────────┘
                                                      │ write
                                                      ▼
                                             ┌──────────────────┐
                                             │  SQLite cache    │
                                             │  (local SSD)     │
                                             │                  │
                                             │  • dirs + files  │
                                             │  • manifests     │
                                             │  • subset pairs  │
                                             │  • annotations   │
                                             └────────┬─────────┘
                                                      │ read
                              ┌────────────────────────┼────────────────────────┐
                              ▼                        ▼                        ▼
                     ┌──────────────┐        ┌──────────────┐        ┌──────────────┐
                     │  TUI         │        │  CLI         │        │  Other tools │
                     │  (ratatui)   │        │  export/scan │        │  (SQL query) │
                     └──────────────┘        └──────────────┘        └──────────────┘
```

**Single Rust binary, three commands:**

| Command | Purpose |
|---------|---------|
| `filetreematch scan <root>` | Walk drive, populate/update cache incrementally |
| `filetreematch analyze` | Compute subset relationships from cached manifests |
| `filetreematch tui` | Browse relationships, annotate, export delete script |

Additional CLI helpers: `list`, `export`.

Scan and analyze are separate so ignore rules can be changed and analysis re-run without re-walking 2TB. DB path is configurable; default `~/.cache/filetreematch/<volume-id>.db`.

### Technology choices

- **Rust** — performance for 2TB metadata walk, single binary
- **SQLite** — cache on SSD, queryable by external tools
- **ratatui** — terminal UI for browsing and annotation
- **jwalk** (or similar) — parallel directory walking

### Performance expectations

First full scan is metadata-bound on spinning disk — comparable to `ls -alR`, modestly faster due to parallelism and no text formatting overhead. Expect hours, not minutes.

Efficiency gains come after the first scan:
- Incremental rescans skip unchanged subtrees entirely
- Analyze runs against SSD-cached manifests (seconds to minutes)
- No need to re-walk disk when tweaking ignore rules or re-analyzing

---

## Data Model

### Schema

```sql
-- Scan metadata
scans (id, root_path, started_at, completed_at, volume_id)

-- Directory tree
directories (
  id, parent_id, name, full_path,     -- full_path indexed
  file_count,          -- recursive file count (excl. ignored)
  total_size,          -- recursive byte sum
  scan_fingerprint,    -- hash(file_count, total_size, max_mtime)
  last_scanned_at
)

-- Individual files (ignored paths never inserted)
files (
  id, directory_id,    -- immediate parent only
  name, size, mtime,
  relative_path,       -- path from directory_id downward
  name_raw             -- optional: original bytes for non-UTF8 names
)

-- Precomputed manifest: every file under a directory, flattened
manifest_entries (
  directory_id,
  relative_path,       -- relative to THAT directory
  size,
  PRIMARY KEY (directory_id, relative_path)
)

-- Computed subset relationships
subset_pairs (
  id,
  subset_dir_id,       -- A: the contained tree
  superset_dir_id,     -- B: contains all of A's files
  file_count,          -- denormalized for display
  total_size,
  is_maximal           -- true = "highest point"
)

-- Scan errors (permission denied, etc.)
scan_errors (
  id, scan_id, path, error_message, occurred_at
)

-- User annotations (TUI writes here)
annotations (
  directory_id PRIMARY KEY,
  status,              -- 'keep' | 'delete_candidate' | 'undecided'
  notes,
  updated_at
)
```

External tools query directly, e.g.:

```sql
SELECT d.full_path
FROM directories d
JOIN annotations a ON d.id = a.directory_id
WHERE a.status = 'delete_candidate';
```

### Subset matching rule

Directory **A ⊆ B** when every row in `manifest_entries` for A has a matching row in B:

```
(A.relative_path, A.size) = (B.relative_path, B.size)
```

Directories with `file_count = 0` (after ignore rules) are excluded from analysis.

### Highest-points collapse

After finding all A ⊆ B pairs, set `is_maximal = true` only when **parent(A) is not also a subset of B**.

Example: if `/archive/old-pc` ⊆ `/archive/master`, the subfolder pair `/archive/old-pc/Photos` ⊆ `/archive/master/Photos` exists but only the parent is maximal.

- Default TUI/CLI view: `WHERE is_maximal = true`
- `--full-detail`: all pairs

### Candidate filtering

Before manifest containment check:
1. `A.file_count ≤ B.file_count`
2. `A.total_size ≤ B.total_size`

Containment check (SQL):

```sql
SELECT NOT EXISTS (
  SELECT 1 FROM manifest_entries ma
  WHERE ma.directory_id = :A
  AND NOT EXISTS (
    SELECT 1 FROM manifest_entries mb
    WHERE mb.directory_id = :B
    AND mb.relative_path = ma.relative_path
    AND mb.size = ma.size
  )
)
```

---

## Scan Engine

### Parallel directory walk

- Parallel walker with configurable thread count (default: CPU cores)
- Work-stealing across threads
- Progress output: files/sec, directories/sec, elapsed, ETA

### Default ignore list

Config file: `~/.config/filetreematch/ignore.toml`

```toml
ignore_names = [
  ".DS_Store", "Thumbs.db", "desktop.ini",
  ".Spotlight-V100", ".Trashes", ".fseventsd",
  ".TemporaryItems", ".DocumentRevisions-V100",
]

ignore_globs = [
  "**/.git/**",
  "**/.svn/**",
  "**/node_modules/**",
  "**/__MACOSX/**",
  "**/.cache/**",
]
```

Hidden files that look like real data (`.bashrc`, `.config/`, etc.) are **not** blanket-ignored.

Per-scan overrides:

```bash
filetreematch scan /Volumes/Archive --ignore-add '*.tmp' --ignore-file project-ignores.toml
```

### Manifest build (bottom-up, single pass)

1. Insert each file into `files` table
2. After subtree complete, roll up child manifests into parent (prefix child paths with child dir name)
3. Compute `file_count`, `total_size`, `scan_fingerprint`
4. Store in `directories` and `manifest_entries`

### Incremental rescan

```
filetreematch scan /Volumes/Archive          # initial
# ... delete folders ...
filetreematch scan /Volumes/Archive          # incremental
```

Top-down walk:
1. Compute fingerprint at each directory
2. **Match → skip subtree** (no disk reads below)
3. **Mismatch → re-walk**, delete stale manifests, rebuild
4. **Path gone → soft-delete** in DB (retain history)
5. Optionally auto-run analyze with `--analyze` flag

Fingerprint: `hash(file_count, total_size, max_mtime)`. Any file change propagates up the ancestor chain, triggering manifest rebuild for affected branches only.

### Analyze phase

```
filetreematch analyze [--full-detail]
```

1. Load directories with `file_count > 0`
2. Sort by `file_count` ascending
3. For each A, find candidates B where counts/sizes allow containment
4. Run manifest containment check
5. Store pairs, compute `is_maximal`
6. Progress bar; report maximal pair count

Re-running analyze after ignore-rule changes is cheap (SSD only, no disk walk).

---

## TUI, Annotations & Export

### TUI layout (ratatui)

Three-pane layout:

```
┌─ Subset Pairs ──────────────────────┬─ Detail ─────────────────────────────┐
│ ▶ /archive/old-pc        2.1 GB   │ Subset:  /archive/old-pc              │
│   ⊂ /archive/master               │ Superset: /archive/master/backup-2019  │
│                                     │ Files: 4,832  |  Size: 2.1 GB         │
│   /backup/drive2/docs      340 MB  │                                       │
│   ⊂ /archive/master/docs          │ Annotations:                          │
│                                     │   old-pc:     [delete candidate]      │
├─────────────────────────────────────┴───────────────────────────────────────┤
│ [k] keep  [d] delete candidate  [u] undecided  [n] note  [e] export  [q]  │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Navigation:**
- `↑/↓` — select pair
- `Enter` — toggle full-detail for selected pair
- `Tab` — switch pane focus
- `f` — filter: all / unreviewed / delete candidates
- `/` — search by path substring

**Annotations (persisted to DB immediately):**
- `k` — keep
- `d` — delete candidate (subset folder)
- `u` — undecided
- `n` — note editor

### Export

```
filetreematch export [--format trash|rm|paths] [--output delete.sh]
```

| Format | Behavior |
|--------|----------|
| `trash` (default) | Move to Trash (recoverable) via macOS osascript or `trash` CLI |
| `rm` | `rm -rf` script; requires `--force` |
| `paths` | Plain path list |

Safety rules:
- Never deletes directly — export only
- Skips paths annotated `keep` even if parent is `delete_candidate`
- Header with timestamp and count
- TUI `e` key shows dry-run preview before export

### CLI summary

```bash
filetreematch scan /Volumes/Archive
filetreematch analyze
filetreematch list [--full-detail] [--status delete_candidate]
filetreematch export --format trash -o ~/cleanup.sh
filetreematch tui
```

---

## Error Handling & Edge Cases

| Situation | Behavior |
|-----------|----------|
| Permission denied | Log warning, skip subtree, record in `scan_errors`, continue |
| Drive disconnected mid-scan | Abort cleanly; partial DB retained; next scan resumes incrementally |
| Symlinks | Don't follow; record as opaque files (link name + size) |
| Hard links | Each path recorded independently |
| Very long paths | Store as-is; skip if stat fails |
| Non-UTF8 filenames | Lossy UTF-8 in `name`; optional `name_raw` for round-trip |
| Empty directories | Excluded from analysis |
| A ⊆ B and A ⊆ C | Report both pairs |
| Case-insensitive FS (macOS APFS) | Match paths case-sensitively as stored on disk |
| Identical trees (A ⊆ B and B ⊆ A) | Both pairs stored; TUI flags as "exact duplicates" |

---

## Testing

**Unit tests:**
- Manifest rollup
- Subset containment (match, no match, empty)
- Maximal-pair collapse
- Fingerprint change detection
- Ignore rule matching

**Integration tests (temp dirs):**
- Full scan + analyze on fixture trees
- Incremental rescan after add/remove/rename
- Annotation + export with safety checks

**Manual acceptance:**
- Scan ~1GB known subset of real archive
- Verify TUI, annotations, export script
- Then run against full 2TB drive

---

## Phase 2: Similarity Matching (out of v1 scope)

Deferred feature using existing manifest data:

```bash
filetreematch analyze --min-similarity 0.8
```

- Asymmetric containment by file count (default) or byte size (flag)
- List unique files in A not found in B before deletion
- Separate collapse rules for fuzzy pairs (likely no collapse in v2)

No schema changes required for v1; add `similarity_pairs` table or extend `subset_pairs` with a `similarity` column in Phase 2.

---

## Open Questions (resolved)

All major design questions resolved during brainstorming:

- Subset (not exact mirror) matching ✓
- User decides keep/delete (report all pairs) ✓
- Highest points default + `--full-detail` ✓
- TUI + queryable SQLite cache ✓
- Configurable ignores + files-only matching ✓
- Annotate + export workflow ✓
- Incremental rescan ✓
- Exact subset v1, similarity Phase 2 ✓
