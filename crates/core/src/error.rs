use std::path::PathBuf;

/// Errors returned by the core library.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("system information unavailable: {0}")]
    Sys(String),

    #[error("I/O error on {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("not a directory: {0}")]
    NotADirectory(PathBuf),

    #[error("process {0} not found")]
    ProcessNotFound(u32),

    #[error("failed to terminate process {0}")]
    KillFailed(u32),

    #[error("refusing to terminate the current process")]
    RefusedSelf,

    #[error("refusing to touch {0}: not a recognised temporary-files directory")]
    UnsafeRoot(PathBuf),
}

pub type Result<T> = std::result::Result<T, Error>;
