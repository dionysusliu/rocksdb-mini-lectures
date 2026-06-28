//! VersionEdit — one delta to the set of live files, plus housekeeping.
//! Tag-based wire format so new fields (level / key-range, Phase M) are added
//! as new tags without disturbing existing field codecs. Mirrors RocksDB
//! db/version_edit.cc (VersionEdit::EncodeTo / DecodeFrom).

use anyhow::{Result, bail};

// Field tags. Reserve room; Phase M will add e.g. TAG_NEW_FILE_WITH_RANGE.
const TAG_NEXT_FILE_NUMBER: u8 = 1;
const TAG_LAST_SEQUENCE: u8 = 2;
const TAG_NEW_FILE: u8 = 3;
const TAG_DELETED_FILE: u8 = 4;

/// A delta against the current Version. Fields are sparse: only what changed.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct VersionEdit {
    pub new_files: Vec<u64>,     // SSTs this edit adds
    pub deleted_files: Vec<u64>, // SSTs this edit removes (compaction; Phase N onward)
    pub next_file_number: Option<u64>,
    pub last_sequence: Option<u64>,
}

impl VersionEdit {
    pub fn new() -> Self {
        Self::default()
    }

    /// Encode to bytes: a sequence of (tag, value) fields. Each numeric value
    /// is a fixed u64 LE — simple and unambiguous; RocksDB uses varint.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        if let Some(n) = self.next_file_number {
            out.push(TAG_NEXT_FILE_NUMBER);
            out.extend_from_slice(&n.to_le_bytes());
        }
        if let Some(s) = self.last_sequence {
            out.push(TAG_LAST_SEQUENCE);
            out.extend_from_slice(&s.to_le_bytes());
        }
        for &f in &self.new_files {
            out.push(TAG_NEW_FILE);
            out.extend_from_slice(&f.to_le_bytes());
        }
        for &f in &self.deleted_files {
            out.push(TAG_DELETED_FILE);
            out.extend_from_slice(&f.to_le_bytes());
        }
        out
    }

    /// Decode bytes back into an edit. Dispatch on tag; each arm owns its codec.
    pub fn decode(buf: &[u8]) -> Result<Self> {
        let mut edit = VersionEdit::new();
        let mut i = 0;
        while i < buf.len() {
            let tag = buf[i];
            i += 1;
            match tag {
                TAG_NEXT_FILE_NUMBER => edit.next_file_number = Some(read_u64(buf, &mut i)?),
                TAG_LAST_SEQUENCE => edit.last_sequence = Some(read_u64(buf, &mut i)?),
                TAG_NEW_FILE => edit.new_files.push(read_u64(buf, &mut i)?),
                TAG_DELETED_FILE => edit.deleted_files.push(read_u64(buf, &mut i)?),
                other => bail!("unknown VersionEdit tag {other}"),
            }
        }
        Ok(edit)
    }
}

fn read_u64(buf: &[u8], i: &mut usize) -> Result<u64> {
    if *i + 8 > buf.len() {
        bail!("truncated u64 field");
    }
    let v = u64::from_le_bytes(buf[*i..*i + 8].try_into().unwrap());
    *i += 8;
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_full_edit() {
        let mut e = VersionEdit::new();
        e.new_files = vec![7, 8, 9];
        e.deleted_files = vec![3];
        e.next_file_number = Some(10);
        e.last_sequence = Some(4242);
        let back = VersionEdit::decode(&e.encode()).unwrap();
        assert_eq!(e, back);
    }

    #[test]
    fn roundtrip_sparse_edit() {
        // A flush edit: only adds + housekeeping, no deletes.
        let mut e = VersionEdit::new();
        e.new_files = vec![1];
        e.next_file_number = Some(2);
        e.last_sequence = Some(5);
        let back = VersionEdit::decode(&e.encode()).unwrap();
        assert_eq!(e, back);
        assert!(back.deleted_files.is_empty());
    }

    #[test]
    fn empty_edit_roundtrips() {
        let e = VersionEdit::new();
        assert_eq!(VersionEdit::decode(&e.encode()).unwrap(), e);
    }
}
