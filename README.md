# filetreematch

Find duplicate folder trees in large archives by comparing file names and sizes — then decide what to safely delete.

Designed for inherited hard drives full of nested backups and replication. Instead of listing individual duplicate files, filetreematch finds entire folder subtrees where one tree is a **subset** of another (every file in A exists at the same relative path with the same size in B), so you can delete whole branches at once.

## Status

Design phase. See [design spec](docs/superpowers/specs/2026-06-26-filetreematch-design.md).

## Planned commands

```bash
filetreematch scan /Volumes/Archive      # Walk drive, populate/update SQLite cache
filetreematch analyze                   # Compute subset relationships
filetreematch tui                       # Browse, annotate, export delete script
filetreematch list                      # Terminal summary
filetreematch export --format trash     # Generate recoverable delete script
```
