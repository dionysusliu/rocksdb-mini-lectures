//! Shared append-only record log: frame = crc32(4) + payload_len(4) + payload.
//! Payload-agnostic. WAL serializes KV into the payload; MANIFEST serializes
//! VersionEdit into the payload. Both reuse this exact framing — same as
//! RocksDB reusing db/log_writer.cc & db/log_reader.cc for WAL and MANIFEST.

use std::{
    fs::{File, OpenOptions},
    io::{BufWriter, Read, Write},
    path::Path,
};

use anyhow::Result;

/// Append-only writer of length-prefixed, CRC-protected records.
pub struct LogWriter {
    w: BufWriter<File>,
}

impl LogWriter {
    /// Open `path` for appending (create if missing).
    pub fn open_append(path: &Path) -> Result<Self> {
        let f = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self {
            w: BufWriter::new(f),
        })
    }

    /// Open `path` fresh, truncating any existing content.
    pub fn create_truncate(path: &Path) -> Result<Self> {
        let f = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;
        Ok(Self {
            w: BufWriter::new(f),
        })
    }

    /// Append one opaque record. The payload schema is the caller's business.
    pub fn append(&mut self, payload: &[u8]) -> Result<()> {
        let crc = crc32fast::hash(payload);
        self.w.write_all(&crc.to_le_bytes())?;
        self.w.write_all(&(payload.len() as u32).to_le_bytes())?;
        self.w.write_all(payload)?;
        Ok(())
    }

    /// Push the buffer to the OS, then fsync to durable storage.
    pub fn sync(&mut self) -> Result<()> {
        self.w.flush()?;
        self.w.get_ref().sync_all()?;
        Ok(())
    }
}

/// Read every intact record from `path`, stopping at the first torn or
/// corrupted record (tolerating a half-written tail). Missing file → empty.
pub fn read_all(path: &Path) -> Result<Vec<Vec<u8>>> {
    let mut buf = Vec::new();
    match File::open(path) {
        Ok(mut f) => {
            f.read_to_end(&mut buf)?;
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => return Err(e.into()),
    }

    let mut out = Vec::new();
    let mut pos = 0;
    while pos + 8 <= buf.len() {
        let crc = u32::from_le_bytes(buf[pos..pos + 4].try_into().unwrap());
        let len = u32::from_le_bytes(buf[pos + 4..pos + 8].try_into().unwrap()) as usize;
        let start = pos + 8;
        if start + len > buf.len() {
            break; // record not fully flushed → torn tail
        }
        let payload = &buf[start..start + len];
        if crc32fast::hash(payload) != crc {
            break; // corrupted
        }
        out.push(payload.to_vec());
        pos = start + len;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn roundtrip_many_records() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("t.log");
        {
            let mut w = LogWriter::open_append(&p).unwrap();
            w.append(b"alpha").unwrap();
            w.append(b"").unwrap(); // empty payload is a valid record
            w.append(b"gamma").unwrap();
            w.sync().unwrap();
        }
        let recs = read_all(&p).unwrap();
        assert_eq!(
            recs,
            vec![b"alpha".to_vec(), b"".to_vec(), b"gamma".to_vec()]
        );
    }

    #[test]
    fn torn_tail_drops_last_record() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("t.log");
        {
            let mut w = LogWriter::open_append(&p).unwrap();
            w.append(b"keep").unwrap();
            w.append(b"lose-me").unwrap();
            w.sync().unwrap();
        }
        let len = std::fs::metadata(&p).unwrap().len();
        let f = OpenOptions::new().write(true).open(&p).unwrap();
        f.set_len(len - 3).unwrap(); // tear the last record's payload
        let recs = read_all(&p).unwrap();
        assert_eq!(recs, vec![b"keep".to_vec()]);
    }

    #[test]
    fn corruption_caught() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("t.log");
        {
            let mut w = LogWriter::open_append(&p).unwrap();
            w.append(b"good").unwrap();
            w.append(b"bad").unwrap();
            w.sync().unwrap();
        }
        let mut raw = std::fs::read(&p).unwrap();
        let n = raw.len();
        raw[n - 1] ^= 0xff;
        std::fs::write(&p, &raw).unwrap();
        assert_eq!(read_all(&p).unwrap(), vec![b"good".to_vec()]);
    }
}
