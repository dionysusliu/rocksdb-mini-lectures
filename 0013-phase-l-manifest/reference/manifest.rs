//! MANIFEST — an append-only log of VersionEdits, plus the CURRENT pointer.
//! CURRENT holds the active manifest's bare filename; it is swapped atomically
//! via write-tmp + rename (POSIX-atomic). Mirrors RocksDB db/version_set.cc
//! (VersionSet::Recover, SetCurrentFile, LogAndApply).

use crate::{
    log::{self, LogWriter},
    version::Version,
    version_edit::VersionEdit,
};
use anyhow::{Result, bail};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

/// State rebuilt from a MANIFEST on open.
pub struct Recovered {
    pub version: Version,
    pub next_file_number: u64,
    pub last_sequence: u64,
}

/// Writer over the currently-active MANIFEST-<n>. flush()/compaction append
/// incremental edits here; when it grows past a threshold the engine rotates.
pub struct Manifest {
    writer: LogWriter,
    size: u64, // approximate on-disk bytes, for the rotation threshold
}

fn manifest_name(num: u64) -> String {
    format!("MANIFEST-{num:06}")
}

impl Manifest {
    /// Replay the MANIFEST that CURRENT points at. None = fresh db (no CURRENT).
    pub fn recover(dir: &Path) -> Result<Option<Recovered>> {
        let current = dir.join("CURRENT");
        let name = match fs::read_to_string(&current) {
            Ok(s) => s.trim().to_string(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        if name.is_empty() {
            bail!("empty CURRENT");
        }
        let records = log::read_all(&dir.join(&name))?;
        let mut version = Version::new();
        let mut next_file_number = 0u64;
        let mut last_sequence = 0u64;
        for rec in &records {
            let edit = VersionEdit::decode(rec)?;
            version.apply(&edit);
            if let Some(n) = edit.next_file_number {
                next_file_number = n;
            }
            if let Some(s) = edit.last_sequence {
                last_sequence = s;
            }
        }
        Ok(Some(Recovered {
            version,
            next_file_number,
            last_sequence,
        }))
    }

    /// Atomically install a fresh MANIFEST whose first record is a full snapshot
    /// of `version`, then point CURRENT at it. This is BOTH the open-time forced
    /// checkpoint and the size-triggered rotation — one machine, two callers.
    ///
    ///   1. write snapshot edit into MANIFEST-<manifest_num>   (CURRENT still old)
    ///   2. fsync MANIFEST-<manifest_num>                       (durable before switch)
    ///   3. write CURRENT.tmp = "MANIFEST-<manifest_num>\n"
    ///   4. fsync CURRENT.tmp
    ///   5. rename CURRENT.tmp -> CURRENT                        (atomic commit point)
    ///   6. fsync(dir)                                           (make the rename durable)
    pub fn install(
        dir: &Path,
        manifest_num: u64,
        version: &Version,
        next_file_number: u64,
        last_sequence: u64,
    ) -> Result<Self> {
        let path = dir.join(manifest_name(manifest_num));
        let mut writer = LogWriter::create_truncate(&path)?;
        let snapshot = version.to_snapshot_edit(next_file_number, last_sequence);
        let payload = snapshot.encode();
        let size = (payload.len() + 8) as u64;
        writer.append(&payload)?;
        writer.sync()?; // (2) new manifest durable before CURRENT moves

        let tmp = dir.join("CURRENT.tmp");
        {
            let mut f = fs::File::create(&tmp)?;
            writeln!(f, "{}", manifest_name(manifest_num))?; // (3)
            f.sync_all()?; // (4)
        }
        fs::rename(&tmp, dir.join("CURRENT"))?; // (5) atomic commit point
        fsync_dir(dir)?; // (6) persist the rename itself

        Ok(Self { writer, size })
    }

    /// Append one incremental edit (a flush) and durably fsync it. The fsync
    /// of this edit is the flush's commit point.
    pub fn append_edit(&mut self, edit: &VersionEdit) -> Result<()> {
        let payload = edit.encode();
        self.size += (payload.len() + 8) as u64;
        self.writer.append(&payload)?;
        self.writer.sync()?;
        Ok(())
    }

    pub fn size(&self) -> u64 {
        self.size
    }
}

/// fsync a directory so a rename/create inside it is durable.
fn fsync_dir(dir: &Path) -> Result<()> {
    let f = fs::File::open(dir)?;
    f.sync_all()?;
    Ok(())
}

/// Read the manifest filename CURRENT points at (test/inspection helper).
pub fn current_target(dir: &Path) -> Result<Option<PathBuf>> {
    match fs::read_to_string(dir.join("CURRENT")) {
        Ok(s) => Ok(Some(dir.join(s.trim()))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}
