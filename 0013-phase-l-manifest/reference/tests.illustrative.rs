//! L2 flagship: metadata crash recovery (a / b1 / b2 / c).
//! L3: engine end-to-end reopen + orphan-sst-ignored regression.

use bytes::Bytes;
use lphase::{
    engine::Engine,
    log::LogWriter,
    manifest::{self, Manifest},
    version::Version,
    version_edit::VersionEdit,
};
use std::fs;
use tempfile::tempdir;

fn b(s: &str) -> Bytes {
    Bytes::from(s.to_string())
}

// ---- L3: engine integrates the manifest end-to-end --------------------------

#[test]
fn survives_reopen_via_manifest() {
    let dir = tempdir().unwrap();
    {
        let mut e = Engine::open(dir.path()).unwrap();
        e.put(b("a"), b("1")).unwrap();
        e.put(b("b"), b("2")).unwrap();
        e.put(b("a"), b("3")).unwrap();
        e.flush().unwrap(); // recorded in MANIFEST
        e.put(b("c"), b("4")).unwrap(); // stays in WAL
    }
    let e2 = Engine::open(dir.path()).unwrap();
    assert_eq!(e2.get(b"a").as_deref(), Some(&b"3"[..]));
    assert_eq!(e2.get(b"b").as_deref(), Some(&b"2"[..]));
    assert_eq!(e2.get(b"c").as_deref(), Some(&b"4"[..])); // from WAL
    assert_eq!(e2.get(b"missing"), None);
}

// The headline regression: the engine trusts the MANIFEST, not the directory.
// An SST file that no edit ever recorded must be invisible.
#[test]
fn orphan_sst_is_ignored() {
    let dir = tempdir().unwrap();
    {
        let mut e = Engine::open(dir.path()).unwrap();
        e.put(b("real"), b("yes")).unwrap();
        e.flush().unwrap();
    }
    // Forge a perfectly valid-looking SST that the manifest never mentions.
    let mut forged = lphase::sst::SstBuilder::new();
    forged.add(b"ghost", b"boo");
    forged.finish(&dir.path().join("999999.sst")).unwrap();

    let e2 = Engine::open(dir.path()).unwrap();
    assert_eq!(e2.get(b"real").as_deref(), Some(&b"yes"[..]));
    assert_eq!(e2.get(b"ghost"), None, "orphan SST must not be read");
}

// next_file_number is recovered from the manifest, so reopen never reuses a
// number and never clobbers a live SST.
#[test]
fn file_numbers_monotonic_across_reopen() {
    let dir = tempdir().unwrap();
    {
        let mut e = Engine::open(dir.path()).unwrap();
        e.put(b("x"), b("1")).unwrap();
        e.flush().unwrap();
    }
    {
        let mut e = Engine::open(dir.path()).unwrap();
        e.put(b("y"), b("2")).unwrap();
        e.flush().unwrap();
        assert_eq!(e.get(b"x").as_deref(), Some(&b"1"[..]));
        assert_eq!(e.get(b"y").as_deref(), Some(&b"2"[..]));
    }
    let e3 = Engine::open(dir.path()).unwrap();
    assert_eq!(e3.get(b"x").as_deref(), Some(&b"1"[..]));
    assert_eq!(e3.get(b"y").as_deref(), Some(&b"2"[..]));
}

// ---- L2 flagship: metadata crash recovery ----------------------------------
//
// We build real on-disk state, then perform filesystem surgery reproducing the
// exact bytes a crash at step X would leave, then recover and assert the
// Version is one consistent state (fully-old or fully-new) — never torn.

fn current_name(dir: &std::path::Path) -> String {
    fs::read_to_string(dir.path_current())
        .unwrap()
        .trim()
        .to_string()
}

trait PathCurrent {
    fn path_current(&self) -> std::path::PathBuf;
}
impl PathCurrent for std::path::Path {
    fn path_current(&self) -> std::path::PathBuf {
        self.join("CURRENT")
    }
}

// Helper: recover and return the live file set, newest first.
fn recovered_files(dir: &std::path::Path) -> Vec<u64> {
    Manifest::recover(dir)
        .unwrap()
        .map(|r| r.version.files_newest_first())
        .unwrap_or_default()
}

// (a) torn tail of an incremental edit -> replay the prefix before the tear.
#[test]
fn crash_a_torn_incremental_edit() {
    let dir = tempdir().unwrap();
    let mut v = Version::new();
    let mut e0 = VersionEdit::new();
    e0.new_files = vec![1];
    v.apply(&e0);
    // install MANIFEST-000000 with snapshot {1}
    let mut m = Manifest::install(dir.path(), 0, &v, 1, 10).unwrap();
    // append a second edit {add 2}, then tear its tail off on disk
    let mut e1 = VersionEdit::new();
    e1.new_files = vec![2];
    e1.next_file_number = Some(3);
    e1.last_sequence = Some(20);
    m.append_edit(&e1).unwrap();

    let mpath = manifest::current_target(dir.path()).unwrap().unwrap();
    let len = fs::metadata(&mpath).unwrap().len();
    fs::OpenOptions::new()
        .write(true)
        .open(&mpath)
        .unwrap()
        .set_len(len - 3) // tear the last record
        .unwrap();

    // recovery stops at the torn record -> only the snapshot {1} survives.
    assert_eq!(recovered_files(dir.path()), vec![1]);
}

// (b1) new manifest fully written but CURRENT not switched -> fully OLD.
#[test]
fn crash_b1_new_manifest_complete_not_switched() {
    let dir = tempdir().unwrap();
    let mut old = Version::new();
    let mut e0 = VersionEdit::new();
    e0.new_files = vec![1];
    old.apply(&e0);
    Manifest::install(dir.path(), 0, &old, 1, 10).unwrap(); // CURRENT -> MANIFEST-000000
    assert_eq!(current_name(dir.path()), "MANIFEST-000000");

    // Simulate: a rotation wrote a COMPLETE new MANIFEST-000002 with set {1,2},
    // but crashed before swapping CURRENT.
    let mut newv = old.clone();
    let mut e1 = VersionEdit::new();
    e1.new_files = vec![2];
    newv.apply(&e1);
    {
        let mut w = LogWriter::create_truncate(&dir.path().join("MANIFEST-000002")).unwrap();
        w.append(&newv.to_snapshot_edit(3, 20).encode()).unwrap();
        w.sync().unwrap();
    }
    // CURRENT still points old -> recovery yields the fully-old set {1}.
    assert_eq!(current_name(dir.path()), "MANIFEST-000000");
    assert_eq!(recovered_files(dir.path()), vec![1]);
}

// (b2) new manifest only half written AND CURRENT not switched -> fully OLD,
// and the half file is never read (CURRENT didn't move).
#[test]
fn crash_b2_new_manifest_half_written() {
    let dir = tempdir().unwrap();
    let mut old = Version::new();
    let mut e0 = VersionEdit::new();
    e0.new_files = vec![1];
    old.apply(&e0);
    Manifest::install(dir.path(), 0, &old, 1, 10).unwrap();

    // Write a TORN MANIFEST-000002 (garbage half-record), CURRENT untouched.
    fs::write(dir.path().join("MANIFEST-000002"), b"\x00\x01\x02half").unwrap();

    assert_eq!(current_name(dir.path()), "MANIFEST-000000");
    assert_eq!(recovered_files(dir.path()), vec![1]); // half file never consulted
}

// (c) leftover CURRENT.tmp -> ignored; CURRENT (old) still authoritative.
#[test]
fn crash_c_leftover_current_tmp() {
    let dir = tempdir().unwrap();
    let mut old = Version::new();
    let mut e0 = VersionEdit::new();
    e0.new_files = vec![1];
    old.apply(&e0);
    Manifest::install(dir.path(), 0, &old, 1, 10).unwrap();

    // A crash between writing CURRENT.tmp and the rename leaves a stray tmp.
    fs::write(dir.path().join("CURRENT.tmp"), b"MANIFEST-000002\n").unwrap();

    assert_eq!(current_name(dir.path()), "MANIFEST-000000");
    assert_eq!(recovered_files(dir.path()), vec![1]); // tmp ignored by recovery
}

// Invariant cross-check: after a clean rotation, recovery lands on the NEW set.
#[test]
fn clean_rotation_lands_new() {
    let dir = tempdir().unwrap();
    let mut v = Version::new();
    let mut e0 = VersionEdit::new();
    e0.new_files = vec![1];
    v.apply(&e0);
    Manifest::install(dir.path(), 0, &v, 1, 10).unwrap();

    let mut e1 = VersionEdit::new();
    e1.new_files = vec![2];
    v.apply(&e1);
    Manifest::install(dir.path(), 2, &v, 3, 20).unwrap(); // clean swap

    assert_eq!(current_name(dir.path()), "MANIFEST-000002");
    assert_eq!(recovered_files(dir.path()), vec![2, 1]);
}
