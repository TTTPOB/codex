use super::*;
use pretty_assertions::assert_eq;

#[test]
fn journal_mode_matches_filesystem_type() {
    assert_eq!(pick_journal_mode(true), SqliteJournalMode::Truncate);
    assert_eq!(pick_journal_mode(false), SqliteJournalMode::Wal);
}

#[cfg(target_os = "linux")]
#[test]
fn detects_linux_nfs_magic() {
    // SAFETY: statfs is a plain C struct whose fields may be initialized directly.
    let mut buf: libc::statfs = unsafe { std::mem::zeroed() };
    buf.f_type = libc::NFS_SUPER_MAGIC as _;
    assert!(is_nfs_statfs(&buf));
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn classifies_database_by_its_parent_directory() {
    let path = std::env::temp_dir().join("codex-nfs-test-missing.sqlite");
    assert!(!path_is_on_nfs(&path));
}
