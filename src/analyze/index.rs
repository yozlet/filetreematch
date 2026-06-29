use anyhow::Result;
use indicatif::ProgressBar;
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};

const EARLY_VERIFY_THRESHOLD: usize = 32;

#[derive(Clone, Copy)]
pub(crate) struct DirMeta {
    pub(crate) file_count: i64,
    pub(crate) total_size: i64,
    id: i64,
    #[allow(dead_code)]
    parent_id: Option<i64>,
}

pub struct ManifestIndex {
    dirs: Vec<DirMeta>,
    meta_by_id: HashMap<i64, DirMeta>,
    parent_by_id: HashMap<i64, Option<i64>>,
    manifests: HashMap<i64, Vec<(String, i64)>>,
    postings: HashMap<(String, i64), Vec<i64>>,
}

impl ManifestIndex {
    pub fn build(conn: &Connection) -> Result<Self> {
        let mut dirs = Vec::new();
        let mut meta_by_id = HashMap::new();
        let mut parent_by_id = HashMap::new();
        {
            let mut stmt = conn.prepare(
                "SELECT id, file_count, total_size, parent_id FROM directories
                 WHERE deleted = 0 AND file_count > 0 ORDER BY file_count ASC",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                ))
            })?;
            for row in rows {
                let (id, file_count, total_size, parent_id) = row?;
                parent_by_id.insert(id, parent_id);
                let meta = DirMeta {
                    id,
                    file_count,
                    total_size,
                    parent_id,
                };
                meta_by_id.insert(id, meta);
                dirs.push(meta);
            }
        }

        let mut manifests: HashMap<i64, Vec<(String, i64)>> = HashMap::new();
        let mut postings: HashMap<(String, i64), Vec<i64>> = HashMap::new();
        {
            let mut stmt = conn.prepare(
                "SELECT directory_id, relative_path, size FROM manifest_entries",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })?;
            for row in rows {
                let (dir_id, path, size) = row?;
                manifests
                    .entry(dir_id)
                    .or_default()
                    .push((path.clone(), size));
                postings.entry((path, size)).or_default().push(dir_id);
            }
        }

        Ok(Self {
            dirs,
            meta_by_id,
            parent_by_id,
            manifests,
            postings,
        })
    }

    pub fn find_pairs(&self, pb: &ProgressBar) -> Vec<(i64, i64)> {
        let mut pairs = Vec::new();
        for dir in &self.dirs {
            let Some(manifest) = self.manifests.get(&dir.id) else {
                pb.inc(1);
                continue;
            };
            if manifest.is_empty() {
                pb.inc(1);
                continue;
            }

            for superset_id in self.find_supersets(dir.id, dir.file_count, dir.total_size, manifest) {
                pairs.push((dir.id, superset_id));
            }
            pb.inc(1);
        }
        pairs
    }

    fn find_supersets(
        &self,
        subset_id: i64,
        subset_count: i64,
        subset_size: i64,
        manifest: &[(String, i64)],
    ) -> Vec<i64> {
        let mut order: Vec<usize> = (0..manifest.len()).collect();
        order.sort_by_key(|&idx| {
            let (path, size) = &manifest[idx];
            self.postings
                .get(&(path.clone(), *size))
                .map(|ids| ids.len())
                .unwrap_or(usize::MAX)
        });

        let mut candidates: Option<HashSet<i64>> = None;
        for idx in order {
            let (path, size) = &manifest[idx];
            let Some(posting) = self.postings.get(&(path.clone(), *size)) else {
                return Vec::new();
            };

            let filtered: HashSet<i64> = posting
                .iter()
                .copied()
                .filter(|&id| id != subset_id && self.meets_bounds(id, subset_count, subset_size))
                .collect();
            if filtered.is_empty() {
                return Vec::new();
            }

            candidates = Some(match candidates {
                None => filtered,
                Some(current) => current.intersection(&filtered).copied().collect(),
            });

            let Some(ref current) = candidates else {
                return Vec::new();
            };
            if current.is_empty() {
                return Vec::new();
            }
            if current.len() <= EARLY_VERIFY_THRESHOLD {
                return current
                    .iter()
                    .copied()
                    .filter(|&id| is_subset_mem(manifest, self.manifests.get(&id).unwrap()))
                    .collect();
            }
        }

        candidates
            .unwrap_or_default()
            .into_iter()
            .filter(|&id| {
                self.manifests
                    .get(&id)
                    .is_some_and(|superset| is_subset_mem(manifest, superset))
            })
            .collect()
    }

    fn meets_bounds(&self, dir_id: i64, subset_count: i64, subset_size: i64) -> bool {
        self.meta_by_id.get(&dir_id).is_some_and(|meta| {
            meta.file_count >= subset_count && meta.total_size >= subset_size
        })
    }

    pub fn parent_of(&self, dir_id: i64) -> Option<i64> {
        self.parent_by_id.get(&dir_id).copied().flatten()
    }

    pub(crate) fn dir_count(&self) -> usize {
        self.dirs.len()
    }

    pub fn meta_for(&self, dir_id: i64) -> Option<DirMeta> {
        self.meta_by_id.get(&dir_id).copied()
    }

    pub(crate) fn is_manifest_subset(&self, subset_id: i64, superset_id: i64) -> bool {
        match (
            self.manifests.get(&subset_id),
            self.manifests.get(&superset_id),
        ) {
            (Some(subset), Some(superset)) => is_subset_mem(subset, superset),
            _ => false,
        }
    }
}

fn is_subset_mem(subset: &[(String, i64)], superset: &[(String, i64)]) -> bool {
    if subset.len() > superset.len() {
        return false;
    }
    let superset_set: HashSet<(&str, i64)> = superset
        .iter()
        .map(|(path, size)| (path.as_str(), *size))
        .collect();
    subset
        .iter()
        .all(|(path, size)| superset_set.contains(&(path.as_str(), *size)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::manifests::insert_manifest_entry;
    use crate::db::Database;

    fn insert_dir(
        conn: &rusqlite::Connection,
        id: i64,
        name: &str,
        full_path: &str,
        parent_id: Option<i64>,
        file_count: i64,
        total_size: i64,
    ) {
        conn.execute(
            "INSERT INTO directories (id, parent_id, name, full_path, file_count, total_size, scan_fingerprint)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, '')",
            rusqlite::params![id, parent_id, name, full_path, file_count, total_size],
        )
        .unwrap();
    }

    #[test]
    fn index_finds_subset_pair() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db = Database::open(&tmp.path().join("test.db")).unwrap();
        let conn = db.conn();
        insert_dir(conn, 1, "a", "/a", None, 1, 10);
        insert_dir(conn, 2, "b", "/b", None, 2, 30);
        insert_manifest_entry(conn, 1, "x.txt", 10).unwrap();
        insert_manifest_entry(conn, 2, "x.txt", 10).unwrap();
        insert_manifest_entry(conn, 2, "y.txt", 20).unwrap();

        let index = ManifestIndex::build(conn).unwrap();
        let pairs = index.find_pairs(&ProgressBar::hidden());
        assert!(pairs.contains(&(1, 2)));
    }
}
