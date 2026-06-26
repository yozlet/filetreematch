use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanFingerprint {
    pub file_count: u64,
    pub total_size: u64,
    pub max_mtime: i64,
}

impl ScanFingerprint {
    pub fn empty() -> Self {
        Self { file_count: 0, total_size: 0, max_mtime: 0 }
    }

    pub fn from_file(size: u64, mtime: i64) -> Self {
        Self { file_count: 1, total_size: size, max_mtime: mtime }
    }

    pub fn merge(children: &[Self]) -> Self {
        Self::merge_with_local(children, 0, 0, 0)
    }

    pub fn merge_with_local(children: &[Self], local_count: u64, local_size: u64, local_mtime: i64) -> Self {
        let mut fp = Self {
            file_count: local_count,
            total_size: local_size,
            max_mtime: local_mtime,
        };
        for c in children {
            fp.file_count += c.file_count;
            fp.total_size += c.total_size;
            fp.max_mtime = fp.max_mtime.max(c.max_mtime);
        }
        fp
    }
}

pub fn hash_fingerprint(fp: &ScanFingerprint) -> String {
    let mut hasher = Sha256::new();
    hasher.update(fp.file_count.to_le_bytes());
    hasher.update(fp.total_size.to_le_bytes());
    hasher.update(fp.max_mtime.to_le_bytes());
    format!("{:x}", hasher.finalize())
}
