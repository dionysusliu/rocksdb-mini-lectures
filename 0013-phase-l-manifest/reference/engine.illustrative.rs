//! Engine wired to the manifest. open() replays the MANIFEST (no more dir-scan);
//! flush() emits a VersionEdit with ordering SST -> manifest -> WAL-truncate.

use crate::{
    manifest::Manifest,
    memtable::MemTable,
    sst::{SstBuilder, SstReader},
    version::Version,
    version_edit::VersionEdit,
    wal::Wal,
};
use anyhow::Result;
use bytes::Bytes;
use std::path::{Path, PathBuf};

pub struct Engine {
    mem: MemTable,
    wal: Wal,
    ssts: Vec<SstReader>, // newest first
    version: Version,     // in-memory mirror of the manifest's live set
    manifest: Manifest,
    dir: PathBuf,
    next_file_number: u64,
    seq: u64, // last_sequence
    flush_threshold: usize,
    max_manifest_size: u64,
}

fn sst_path(dir: &Path, num: u64) -> PathBuf {
    dir.join(format!("{num:06}.sst"))
}

impl Engine {
    pub fn open(dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(dir)?;
        let wal_path = dir.join("wal.log");

        // 1. replay WAL into the memtable, tracking the max seq seen.
        let mut mem = MemTable::new();
        let mut max_wal_seq = 0u64;
        for (seq, k, v) in Wal::replay(&wal_path)? {
            mem.put(k, v);
            max_wal_seq = max_wal_seq.max(seq);
        }
        let wal = Wal::open_append(&wal_path)?;

        // 2. recover the live file set from the MANIFEST (replaces discover_ssts).
        let (version, mut next_file_number, manifest_seq) = match Manifest::recover(dir)? {
            Some(r) => (r.version, r.next_file_number, r.last_sequence),
            None => (Version::new(), 0, 0), // fresh db
        };
        let seq = manifest_seq.max(max_wal_seq);

        // 3. open readers for exactly the files the manifest lists, newest first.
        let mut ssts = Vec::new();
        for num in version.files_newest_first() {
            ssts.push(SstReader::open(&sst_path(dir, num))?);
        }

        // 4. open-time forced checkpoint: snapshot the recovered Version into a
        //    fresh MANIFEST and atomically swap CURRENT (folds history).
        let manifest_num = next_file_number;
        next_file_number += 1;
        let manifest = Manifest::install(dir, manifest_num, &version, next_file_number, seq)?;

        Ok(Self {
            mem,
            wal,
            ssts,
            version,
            manifest,
            dir: dir.to_path_buf(),
            next_file_number,
            seq,
            flush_threshold: 4 * 1024 * 1024,
            max_manifest_size: 64 * 1024,
        })
    }

    pub fn put(&mut self, key: Bytes, value: Bytes) -> Result<()> {
        self.seq += 1;
        self.wal.append(self.seq, &key, &value)?;
        self.wal.sync()?;
        self.mem.put(key, value);
        if self.mem.len_bytes() >= self.flush_threshold {
            self.flush()?;
        }
        Ok(())
    }

    pub fn get(&self, key: &[u8]) -> Option<Bytes> {
        self.mem
            .get(key)
            .or_else(|| self.ssts.iter().find_map(|r| r.get(key)))
    }

    /// Flush the memtable to an SST and record it. Crash-safe ordering:
    ///   SST(fsync) -> manifest edit(fsync = COMMIT) -> truncate WAL.
    pub fn flush(&mut self) -> Result<()> {
        if self.mem.is_empty() {
            return Ok(());
        }
        // (1) write SST, fsync (done inside finish()).
        let num = self.next_file_number;
        self.next_file_number += 1;
        let path = sst_path(&self.dir, num);
        let mut b = SstBuilder::new();
        for (k, v) in self.mem.iter() {
            b.add(&k, &v);
        }
        b.finish(&path)?;

        // (2) record the edit and fsync it — this is the commit point.
        let mut edit = VersionEdit::new();
        edit.new_files = vec![num];
        edit.next_file_number = Some(self.next_file_number);
        edit.last_sequence = Some(self.seq);
        self.manifest.append_edit(&edit)?;

        // (3) only now is it safe to drop the WAL and update memory.
        self.version.apply(&edit);
        self.ssts.insert(0, SstReader::open(&path)?);
        self.wal = Wal::reset(&self.dir.join("wal.log"))?;
        self.mem = MemTable::new();

        // size-triggered rotation (same machine as open-time checkpoint).
        if self.manifest.size() > self.max_manifest_size {
            self.rotate_manifest()?;
        }
        Ok(())
    }

    fn rotate_manifest(&mut self) -> Result<()> {
        let manifest_num = self.next_file_number;
        self.next_file_number += 1;
        self.manifest = Manifest::install(
            &self.dir,
            manifest_num,
            &self.version,
            self.next_file_number,
            self.seq,
        )?;
        Ok(())
    }

    pub fn set_flush_threshold(&mut self, bytes: usize) {
        self.flush_threshold = bytes;
    }
    pub fn set_max_manifest_size(&mut self, bytes: u64) {
        self.max_manifest_size = bytes;
    }
}
