//! Version — the materialized set of live files, = fold(all VersionEdits).
//! Flat MVP: just a set of file numbers (no levels yet; Phase M adds levels).
//! Read order is derived: higher file number = flushed later = newer, so the
//! engine sorts descending when building readers (same rule discover_ssts used).

use crate::version_edit::VersionEdit;
use std::collections::BTreeSet;

#[derive(Debug, Default, Clone)]
pub struct Version {
    files: BTreeSet<u64>, // live SST file numbers
}

impl Version {
    pub fn new() -> Self {
        Self::default()
    }

    /// Fold one edit into the current state: adds insert, deletes remove.
    /// (deleted_files has no producer until compaction in Phase N, but the
    /// removal branch is the whole point of an *edit* — tested via synthetic edits.)
    pub fn apply(&mut self, edit: &VersionEdit) {
        for &f in &edit.new_files {
            self.files.insert(f);
        }
        for &f in &edit.deleted_files {
            self.files.remove(&f);
        }
    }

    /// Live file numbers, newest first (descending), for the read path.
    pub fn files_newest_first(&self) -> Vec<u64> {
        self.files.iter().rev().copied().collect()
    }

    /// Snapshot the current live set into a single full VersionEdit — the first
    /// record written into a freshly rotated MANIFEST (folds history to a point).
    pub fn to_snapshot_edit(&self, next_file_number: u64, last_sequence: u64) -> VersionEdit {
        VersionEdit {
            new_files: self.files.iter().copied().collect(),
            deleted_files: Vec::new(),
            next_file_number: Some(next_file_number),
            last_sequence: Some(last_sequence),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fold_adds_then_deletes() {
        let mut v = Version::new();
        let mut e1 = VersionEdit::new();
        e1.new_files = vec![1, 2, 3];
        v.apply(&e1);
        let mut e2 = VersionEdit::new();
        e2.new_files = vec![4];
        e2.deleted_files = vec![2]; // synthetic delete — exercises removal branch
        v.apply(&e2);
        assert_eq!(v.files_newest_first(), vec![4, 3, 1]); // 2 gone, newest first
    }
}
