use std::path::PathBuf;
use std::collections::HashSet;
use dashmap::DashMap;

/// metadata for a chunked upload in progress
#[derive(Clone)]
pub struct ChunkedUploadMetadata {
    pub filename: String,
    pub total_size: u64,
    pub chunk_size: usize,
    pub total_chunks: usize,
    pub received_chunks: HashSet<usize>,
}

/// shared application state
#[derive(Clone)]
pub struct AppState {
    pub files_dir: PathBuf,
    /// track ongoing chunked uploads by upload_id
    pub chunked_uploads: DashMap<String, ChunkedUploadMetadata>,
}

impl AppState {
    /// create a new app state with the given files directory
    pub fn new(files_dir: PathBuf) -> Self {
        Self {
            files_dir,
            chunked_uploads: DashMap::new(),
        }
    }
}
