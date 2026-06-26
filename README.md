# filetreematch

Find duplicate folder trees in large archives by comparing file names and sizes — then decide what to safely delete.

Designed for inherited hard drives full of nested backups and replication. Instead of listing individual duplicate files, filetreematch finds entire folder subtrees where one tree is a **subset** of another: every file in A exists at the same relative path with the same size in B. If A ⊆ B, deleting A loses no data.

Matching uses **path + size only** (no content hashing). Empty directories are ignored.

## Requirements

- Rust toolchain (`cargo build`)
- macOS for the default `export --format trash` script (uses Finder via AppleScript)

## Install

```bash
git clone <repo-url> filetreematch   # or use your checkout
cd filetreematch
cargo build --release
```

Binary: `target/release/filetreematch`

## Quick start

```bash
# 1. Scan the archive (writes/updates SQLite cache on your local SSD)
./target/release/filetreematch scan /Volumes/Archive --analyze

# 2. Browse subset pairs and mark delete candidates
./target/release/filetreematch tui

# 3. Export a script — review before running!
./target/release/filetreematch export --format trash -o ~/cleanup.sh
chmod +x ~/cleanup.sh   # trash format is a shell script
# ./cleanup.sh          # only after you have reviewed every path
```

**Tip:** Try a small folder first (`scan ~/test-archive-subset --analyze`) before pointing at a full ~2TB drive. The first full scan on a spinning-platter disk can take **several hours**. Re-scans skip unchanged subtrees and are much faster.

## Typical workflow

| Step | Command | What it does |
|------|---------|--------------|
| Scan | `scan /path/to/archive` | Walk disk, build/update cache |
| Analyze | `analyze` | Find subset pairs from cache (no disk walk) |
| Review | `tui` or `list` | See pairs, annotate keep/delete |
| Export | `export -o script.sh` | Write delete script from annotations |
| Re-scan | `scan /path/to/archive` | Incremental update after deletions |

`scan --analyze` runs analyze immediately after a successful scan.

Re-run `analyze` after changing ignore rules (no rescan needed). Re-run `scan` after deleting folders on the archive.

## Cache and `--db`

### Where the database lives

`scan` writes SQLite to:

```
~/.cache/filetreematch/<name>.db
```

`<name>` is the **last component** of the scan path:

| Scan root | Database file |
|-----------|---------------|
| `/Volumes/Archive` | `~/.cache/filetreematch/Archive.db` |
| `/Users/you/inherited-hd` | `~/.cache/filetreematch/inherited-hd.db` |

Override with the global `--db` flag on any command:

```bash
filetreematch scan /Volumes/Archive --db /tmp/archive.db
filetreematch analyze --db /tmp/archive.db
filetreematch tui --db /tmp/archive.db
```

### Which database `analyze`, `list`, `tui`, and `export` use

If you omit `--db`, these commands open the **most recently modified** `.db` file in `~/.cache/filetreematch/`. If that directory is missing or empty, they fail with an error — run `scan` first or pass `--db` explicitly.

If you work with multiple archives, always pass `--db` to avoid opening the wrong cache.

### Querying the cache directly

The database is plain SQLite. Other tools can query it while filetreematch is not running:

```bash
sqlite3 ~/.cache/filetreematch/Archive.db \
  "SELECT d.full_path FROM directories d
   JOIN annotations a ON a.directory_id = d.id
   WHERE a.status = 'delete_candidate';"
```

## Ignore rules

Default rules skip macOS metadata (`.DS_Store`, `.Trashes`, …), VCS dirs (`.git`, `.svn`), `node_modules`, `__MACOSX`, and similar noise. Hidden files that look like real data (`.bashrc`, `.config/`, etc.) are **not** ignored.

**Load order for `scan`:**

1. If `--ignore-file path.toml` is set → use **only** that file (built-in defaults are not merged in).
2. Else if `~/.config/filetreematch/ignore.toml` exists → use that file.
3. Else → use built-in defaults.

`--ignore-add 'pattern'` appends extra globs to whichever rules were loaded above.

```bash
# Optional: install custom defaults
mkdir -p ~/.config/filetreematch
cp config/ignore.toml.example ~/.config/filetreematch/ignore.toml

# Per-scan extras
filetreematch scan /Volumes/Archive --ignore-add '*.tmp'

# Per-scan replacement config
filetreematch scan /Volumes/Archive --ignore-file ./project-ignores.toml
```

## Understanding results

### Subset pairs

A pair **A ⊂ B** means every file in folder tree A exists at the same relative path with the same size somewhere in tree B. A is a deletion candidate; B contains all of A's data (and possibly more).

### Maximal vs full detail

`analyze` stores **all** subset pairs and marks **maximal** ones (`is_maximal = 1`): the largest folders where a parent is not also a subset of the same superset.

By default, `list` and `tui` show **maximal pairs only**. Pass `--full-detail` to include every subset pair (including nested subfolders).

```bash
filetreematch list
filetreematch list --full-detail
filetreematch tui --full-detail
```

### Exact duplicates

When two trees are identical, both A ⊂ B and B ⊂ A exist. The TUI shows an **`[exact duplicate]`** badge in the detail pane when the reverse pair is also present.

## Commands

### `scan`

```bash
filetreematch scan <ROOT> [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `<ROOT>` | Directory to scan (required) |
| `--analyze` | Run analyze after scan completes |
| `--ignore-add <GLOB>` | Append ignore globs (repeatable) |
| `--ignore-file <PATH>` | Use this ignore config instead of defaults |
| `--db <PATH>` | SQLite cache path (default: `~/.cache/filetreematch/<name>.db`) |

```bash
filetreematch scan /Volumes/Archive
filetreematch scan /Volumes/Archive --analyze
filetreematch scan /Volumes/Archive --ignore-add '*.tmp'
filetreematch scan /Volumes/Archive --db ~/archive.db
```

Symlinks are not followed; they are recorded as opaque files (name + link metadata size).

### `analyze`

```bash
filetreematch analyze [--db <PATH>]
```

Recomputes all subset pairs from cached manifests. Prints a summary like:

```
Found 47 maximal subset pairs (312 total pairs)
```

Use `list` or `tui --full-detail` to browse non-maximal pairs. The `analyze --full-detail` flag is accepted but currently has no effect — filtering is done at display time.

### `list`

```bash
filetreematch list [--full-detail] [--status <STATUS>] [--db <PATH>]
```

Prints pairs to stdout:

```
/Volumes/Archive/old-pc ⊂ /Volumes/Archive/master (4832 files, 2.10 GB)
```

| `--status` value | Shows pairs where subset folder is annotated… |
|------------------|-----------------------------------------------|
| `keep` | keep |
| `delete_candidate` | delete candidate |
| `undecided` | undecided |

### `tui`

```bash
filetreematch tui [--full-detail] [--db <PATH>]
```

Interactive browser (ratatui). Annotations are saved to the database immediately.

| Key | Action |
|-----|--------|
| `↑` / `↓` | Select pair |
| `k` | Mark subset folder **keep** |
| `d` | Mark subset folder **delete candidate** |
| `u` | Mark **undecided** |
| `n` | Edit note — `Enter` save, `Esc` cancel, `Ctrl-u` clear note |
| `f` | Cycle filter: all → unreviewed → delete candidates |
| `/` | Search by path substring — type to filter, `Enter` confirm, `Esc` clear |
| `q` | Quit |

The list shows annotation markers on subset paths. Detail pane shows paths, file count, size, status, notes, and exact-duplicate badge.

### `export`

```bash
filetreematch export --output <PATH> [--format <FORMAT>] [--force] [--db <PATH>]
```

**`--output` / `-o` is required.** Generates a script or path list from folders annotated `delete_candidate`. Does **not** delete anything itself.

| Format | Behavior |
|--------|----------|
| `trash` (default) | Bash script: moves each path to Trash via macOS Finder |
| `paths` | Plain list, one path per line |
| `rm` | `rm -rf` script — **requires `--force`** |

Paths annotated **keep** are excluded even if a parent is marked delete candidate.

```bash
filetreematch export --format trash -o ~/cleanup.sh
filetreematch export --format paths -o ~/paths.txt
filetreematch export --format rm -o ~/cleanup.sh --force
```

Review the generated file before executing it.

## Design

See [design spec](docs/superpowers/specs/2026-06-26-filetreematch-design.md) for schema, edge cases, and planned Phase 2 features (similarity matching).
