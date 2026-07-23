//! Shared SQLite connection configuration.

use codex_utils_absolute_path::AbsolutePathBuf;
use log::LevelFilter;
use sqlx::ConnectOptions;
use sqlx::Error;
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteAutoVacuum;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::sqlite::SqliteJournalMode;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::sqlite::SqliteSynchronous;
use std::path::Path;
use std::time::Duration;

/// Resolved configuration shared by all Codex SQLite connections.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SqliteConfig {
    sqlite_home: AbsolutePathBuf,
}

impl SqliteConfig {
    pub fn from_sqlite_home(sqlite_home: AbsolutePathBuf) -> Self {
        Self { sqlite_home }
    }

    pub fn new_for_testing(sqlite_home: AbsolutePathBuf) -> Self {
        Self::from_sqlite_home(sqlite_home)
    }

    pub fn home(&self) -> &Path {
        self.sqlite_home.as_path()
    }

    /// Open a writable Codex SQLite database, creating it if necessary.
    pub async fn open_read_write_pool(&self, path: &Path) -> Result<SqlitePool, Error> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(journal_mode_for_path(path))
            .synchronous(SqliteSynchronous::Normal)
            .auto_vacuum(SqliteAutoVacuum::Incremental)
            .busy_timeout(Duration::from_secs(5))
            .log_statements(LevelFilter::Off);
        SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
    }

    /// Open an existing Codex SQLite database without creating or modifying it.
    pub async fn open_read_only_pool(&self, path: &Path) -> Result<SqlitePool, Error> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(false)
            .read_only(true)
            .log_statements(LevelFilter::Off);
        SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
    }
}

/// Choose a rollback journal on NFS because SQLite WAL relies on coherent mmap.
fn journal_mode_for_path(path: &Path) -> SqliteJournalMode {
    let on_nfs = path_is_on_nfs(path);
    if on_nfs {
        log::warn!(
            "SQLite database on NFS detected at {}; using TRUNCATE journal mode (WAL is unsafe on network filesystems)",
            path.display()
        );
    }
    pick_journal_mode(on_nfs)
}

fn pick_journal_mode(is_nfs: bool) -> SqliteJournalMode {
    if is_nfs {
        SqliteJournalMode::Truncate
    } else {
        SqliteJournalMode::Wal
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn path_is_on_nfs(path: &Path) -> bool {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    // The database may not exist yet, so inspect its existing parent directory.
    let target = path.parent().unwrap_or_else(|| Path::new("."));
    let Ok(c_path) = CString::new(target.as_os_str().as_bytes()) else {
        return true;
    };
    // SAFETY: statfs initializes the provided buffer from the valid C path.
    let mut buf: libc::statfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statfs(c_path.as_ptr(), &mut buf) } != 0 {
        return true;
    }
    is_nfs_statfs(&buf)
}

#[cfg(target_os = "linux")]
fn is_nfs_statfs(buf: &libc::statfs) -> bool {
    buf.f_type as u64 == libc::NFS_SUPER_MAGIC as u64
}

#[cfg(target_os = "macos")]
fn is_nfs_statfs(buf: &libc::statfs) -> bool {
    (buf.f_flags & libc::MNT_NFS) != 0
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn path_is_on_nfs(_path: &Path) -> bool {
    false
}

#[cfg(test)]
#[path = "sqlite_tests.rs"]
mod tests;
