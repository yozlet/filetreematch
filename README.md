# filetreematch

Find duplicate folder trees in large archives by comparing file names and sizes — then decide what to safely delete.

Designed for inherited hard drives full of nested backups and replication. Instead of listing individual duplicate files, filetreematch finds entire folder subtrees where one tree is a **subset** of another (every file in A exists at the same relative path with the same size in B), so you can delete whole branches at once.

## Install

```bash
cargo build --release
# binary: target/release/filetreematch
```

## Typical workflow

1. **Scan** the archive — walks the drive and builds a SQLite cache on your local SSD.
2. **Analyze** — finds subset relationships from cached manifests (no disk walk).
3. **TUI** — browse pairs, annotate keep/delete/notes.
4. **Export** — generate a recoverable delete script from your annotations.

```bash
filetreematch scan /Volumes/Archive
filetreematch analyze
filetreematch tui
filetreematch export --format trash -o ~/cleanup.sh
```

Re-run `scan` after deleting folders on the archive; incremental rescans skip unchanged subtrees. Re-run `analyze` any time you change ignore rules — it only reads the cache.

**First scan of a ~2TB spinning-platter drive can take several hours.** Subsequent scans are much faster when most subtrees are unchanged.

## Cache and database

Scan writes a SQLite database under `~/.cache/filetreematch/`, named after the volume (e.g. `Archive.db` for `/Volumes/Archive`).

Other commands (`analyze`, `list`, `tui`, `export`) use the most recently modified `.db` in that directory, or you can pass an explicit path:

```bash
filetreematch analyze --db ~/.cache/filetreematch/Archive.db
filetreematch list --db ~/.cache/filetreematch/Archive.db
filetreematch tui --db ~/.cache/filetreematch/Archive.db
filetreematch export --format trash -o ~/cleanup.sh --db ~/.cache/filetreematch/Archive.db
```

## Ignore rules

Default ignore rules skip macOS metadata, VCS dirs, `node_modules`, and similar noise. Configure them at:

```
~/.config/filetreematch/ignore.toml
```

See [`config/ignore.toml.example`](config/ignore.toml.example) for the default template. Per-scan overrides:

```bash
filetreematch scan /Volumes/Archive --ignore-add '*.tmp' --ignore-file project-ignores.toml
```

`--ignore-file` replaces the default config for that scan; `--ignore-add` appends extra globs.

## Commands

### scan

Walk a directory tree, populate or update the cache, and optionally run analyze when finished.

```bash
filetreematch scan /Volumes/Archive
filetreematch scan /Volumes/Archive --analyze
filetreematch scan /Volumes/Archive --ignore-add '*.tmp'
filetreematch scan /Volumes/Archive --ignore-file ./project-ignores.toml
filetreematch scan /Volumes/Archive --db /tmp/custom.db
```

### analyze

Compute subset pairs from cached directory manifests. By default only **maximal** pairs are stored (highest useful deletion points); use `--full-detail` to keep every subset relationship.

```bash
filetreematch analyze
filetreematch analyze --full-detail
filetreematch analyze --db ~/.cache/filetreematch/Archive.db
```

### list

Print subset pairs to the terminal.

```bash
filetreematch list
filetreematch list --full-detail
filetreematch list --status delete_candidate
filetreematch list --db ~/.cache/filetreematch/Archive.db
```

Output format: `subset ⊂ superset (N files, size)`.

### tui

Interactive browser for subset pairs (ratatui).

```bash
filetreematch tui
filetreematch tui --full-detail
filetreematch tui --db ~/.cache/filetreematch/Archive.db
```

| Key | Action |
|-----|--------|
| `↑` / `↓` | Select pair |
| `k` | Mark subset as **keep** |
| `d` | Mark subset as **delete candidate** |
| `u` | Mark **undecided** |
| `n` | Edit note |
| `f` | Cycle filter: all / unreviewed / delete candidates |
| `/` | Search by path substring |
| `q` | Quit |

Annotations are saved to the database immediately.

### export

Generate a script or path list from folders marked **delete candidate**. Never deletes directly — review the output before running it.

```bash
filetreematch export --format trash -o ~/cleanup.sh
filetreematch export --format paths -o ~/delete-list.txt
filetreematch export --format rm -o ~/cleanup.sh --force
filetreematch export --format trash -o ~/cleanup.sh --db ~/.cache/filetreematch/Archive.db
```

| Format | Behavior |
|--------|----------|
| `trash` (default) | Bash script that moves paths to Trash via macOS Finder |
| `paths` | Plain list of paths, one per line |
| `rm` | `rm -rf` script; requires `--force` |

Paths annotated **keep** are skipped even if a parent is marked delete candidate.

## Design

See [design spec](docs/superpowers/specs/2026-06-26-filetreematch-design.md) for matching rules, schema, and edge cases.
