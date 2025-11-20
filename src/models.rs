use serde::{Deserialize, Serialize};
// boring shit ahead

// information about a file in the file system
#[derive(Serialize, Debug)]
pub struct FileInfo {
    pub name: String,
    pub size: u64,
    pub modified: String,
    pub is_dir: bool,
}

// response for file listing endpoint
#[derive(Serialize, Debug)]
pub struct FileListResponse {
    pub files: Vec<FileInfo>,
    pub total: usize,
}

// response for file upload endpoint
#[derive(Serialize, Debug)]
pub struct UploadResponse {
    pub success: bool,
    pub filename: String,
    pub size: u64,
}

// response for file deletion endpoint
#[derive(Serialize, Debug)]
pub struct DeleteResponse {
    pub success: bool,
    pub filename: String,
}

// response for server statistics endpoint
#[derive(Serialize, Debug)]
pub struct StatsResponse {
    pub total_files: usize,
    pub total_size: u64,
    pub files_dir: String,
}

// generic error response
#[derive(Serialize, Debug)]
pub struct ErrorResponse {
    pub error: String,
}

// request for batch delete operation
#[derive(Deserialize, Debug)]
pub struct BatchDeleteRequest {
    pub filenames: Vec<String>,
}

// result of a single file deletion in batch operation
#[derive(Serialize, Debug)]
pub struct BatchDeleteResult {
    pub filename: String,
    pub success: bool,
    pub error: Option<String>,
}

// response for batch delete operation
#[derive(Serialize, Debug)]
pub struct BatchDeleteResponse {
    pub total: usize,
    pub successful: usize,
    pub failed: usize,
    pub results: Vec<BatchDeleteResult>,
}

// request to initialize a chunked upload
#[derive(Deserialize, Debug)]
pub struct ChunkedUploadInit {
    pub filename: String,
    pub total_size: u64,
    pub chunk_size: usize,
}

// response for chunked upload initialization
#[derive(Serialize, Debug)]
pub struct ChunkedUploadInitResponse {
    pub upload_id: String,
    pub chunk_size: usize,
    pub total_chunks: usize,
}

// request to complete a chunked upload
#[derive(Deserialize, Debug)]
pub struct ChunkedUploadComplete {
    pub upload_id: String,
}

// response for chunked upload completion
#[derive(Serialize, Debug)]
pub struct ChunkedUploadCompleteResponse {
    pub success: bool,
    pub filename: String,
    pub size: u64,
}
