use axum::{
    extract::{Path, Multipart, State},
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;
use std::collections::HashSet;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::models::{
    BatchDeleteRequest, BatchDeleteResponse, BatchDeleteResult,
    DeleteResponse, ErrorResponse, FileInfo, FileListResponse, 
    StatsResponse, UploadResponse, ChunkedUploadInit, ChunkedUploadInitResponse,
    ChunkedUploadComplete, ChunkedUploadCompleteResponse,
};
use crate::state::{AppState, ChunkedUploadMetadata};
use crate::utils::sanitize_filename;

// upload a file via multipart form data
pub async fn upload_file(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::debug!("Processing file upload request");
    
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        tracing::error!("Failed to read multipart field: {}", e);
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("Failed to read multipart field: {}", e),
            }),
        )
    })? {
        let filename = field.file_name().ok_or_else(|| {
            tracing::warn!("Upload request missing filename");
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "No filename provided".to_string(),
                }),
            )
        })?;

        tracing::debug!("Receiving file: {}", filename);

        // sanitize filename to prevent directory traversal
        let sanitized_filename = sanitize_filename(filename);
        let file_path = state.files_dir.join(&sanitized_filename);
        tracing::trace!("Sanitized filename: {} -> {}", filename, sanitized_filename);
        tracing::trace!("Target path: {:?}", file_path);

        // read the file data
        let data = field.bytes().await.map_err(|e| {
            tracing::error!("Failed to read file data for {}: {}", sanitized_filename, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to read file data: {}", e),
                }),
            )
        })?;

        let size = data.len() as u64;
        tracing::debug!("File size: {} bytes", size);

        // write to disk
        let mut file = fs::File::create(&file_path).await.map_err(|e| {
            tracing::error!("Failed to create file {}: {}", sanitized_filename, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to create file: {}", e),
                }),
            )
        })?;

        file.write_all(&data).await.map_err(|e| {
            tracing::error!("Failed to write to file {}: {}", sanitized_filename, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to write file: {}", e),
                }),
            )
        })?;

        file.sync_all().await.map_err(|e| {
            tracing::error!("Failed to sync file {}: {}", sanitized_filename, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to sync file: {}", e),
                }),
            )
        })?;

        tracing::info!("✅ Uploaded file: {} ({} bytes)", sanitized_filename, size);

        return Ok(Json(UploadResponse {
            success: true,
            filename: sanitized_filename,
            size,
        }));
    }

    tracing::warn!("Upload request contained no file field");
    Err((
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "No file provided".to_string(),
        }),
    ))
}

// list all files in the files directory
pub async fn list_files(
    State(state): State<Arc<AppState>>,
) -> Result<Json<FileListResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::debug!("Listing files in directory: {:?}", state.files_dir);
    let mut files = Vec::new();

    let mut entries = fs::read_dir(&state.files_dir).await.map_err(|e| {
        tracing::error!("Failed to read directory {:?}: {}", state.files_dir, e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to read directory: {}", e),
            }),
        )
    })?;

    while let Some(entry) = entries.next_entry().await.map_err(|e| {
        tracing::error!("Failed to read directory entry: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to read directory entry: {}", e),
            }),
        )
    })? {
        let metadata = entry.metadata().await.map_err(|e| {
            tracing::warn!("Failed to read metadata for entry: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to read metadata: {}", e),
                }),
            )
        })?;

        let modified = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| {
                chrono::DateTime::from_timestamp(d.as_secs() as i64, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| "Unknown".to_string())
            })
            .unwrap_or_else(|| "Unknown".to_string());

        let name = entry.file_name().to_string_lossy().to_string();
        tracing::trace!("Found file: {} ({} bytes)", name, metadata.len());

        files.push(FileInfo {
            name,
            size: metadata.len(),
            modified,
            is_dir: metadata.is_dir(),
        });
    }

    let total = files.len();
    tracing::debug!("Found {} files total", total);
    Ok(Json(FileListResponse { files, total }))
}

// delete a specific file
pub async fn delete_file(
    State(state): State<Arc<AppState>>,
    Path(filename): Path<String>,
) -> Result<Json<DeleteResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::debug!("Request to delete file: {}", filename);
    
    // sanitize filename to prevent directory traversal
    let sanitized_filename = sanitize_filename(&filename);
    let file_path = state.files_dir.join(&sanitized_filename);
    
    tracing::trace!("Target path for deletion: {:?}", file_path);

    // check if file exists
    if !file_path.exists() {
        tracing::warn!("File not found for deletion: {}", sanitized_filename);
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("File not found: {}", sanitized_filename),
            }),
        ));
    }

    // delete the file
    fs::remove_file(&file_path).await.map_err(|e| {
        tracing::error!("Failed to delete file {}: {}", sanitized_filename, e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to delete file: {}", e),
            }),
        )
    })?;

    tracing::info!("🗑️  Deleted file: {}", sanitized_filename);

    Ok(Json(DeleteResponse {
        success: true,
        filename: sanitized_filename,
    }))
}

// get server statistics
pub async fn get_stats(
    State(state): State<Arc<AppState>>,
) -> Result<Json<StatsResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::debug!("Calculating server statistics");
    let mut total_files = 0;
    let mut total_size = 0u64;

    let mut entries = fs::read_dir(&state.files_dir).await.map_err(|e| {
        tracing::error!("Failed to read directory for stats: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to read directory: {}", e),
            }),
        )
    })?;

    while let Some(entry) = entries.next_entry().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to read directory entry: {}", e),
            }),
        )
    })? {
        let metadata = entry.metadata().await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to read metadata: {}", e),
                }),
            )
        })?;

        if metadata.is_file() {
            total_files += 1;
            total_size += metadata.len();
        }
    }
    
    tracing::debug!("Stats: {} files, {} bytes total", total_files, total_size);

    Ok(Json(StatsResponse {
        total_files,
        total_size,
        files_dir: state
            .files_dir
            .canonicalize()
            .unwrap_or_else(|_| state.files_dir.clone())
            .to_string_lossy()
            .to_string(),
    }))
}

// health check endpoint
pub async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "juicebox-omega-admin",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}

// batch delete multiple files
pub async fn batch_delete_files(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<BatchDeleteRequest>,
) -> Json<BatchDeleteResponse> {
    let mut results = Vec::new();
    let mut successful = 0;
    let mut failed = 0;

    for filename in payload.filenames {
        // sanitize filename to prevent directory traversal like fucken .. and . and all that shit
        let sanitized_filename = sanitize_filename(&filename);
        let file_path = state.files_dir.join(&sanitized_filename);

        // check if file exists and delete
        match fs::remove_file(&file_path).await {
            Ok(_) => {
                tracing::info!("🗑️  Batch deleted file: {}", sanitized_filename);
                successful += 1;
                results.push(BatchDeleteResult {
                    filename: sanitized_filename,
                    success: true,
                    error: None,
                });
            }
            Err(e) => {
                tracing::warn!("❌ Failed to delete file {}: {}", sanitized_filename, e);
                failed += 1;
                results.push(BatchDeleteResult {
                    filename: sanitized_filename,
                    success: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    let total = results.len();
    tracing::info!(
        "📦 Batch delete completed: {}/{} successful",
        successful,
        total
    );

    Json(BatchDeleteResponse {
        total,
        successful,
        failed,
        results,
    })
}

// initialize a chunked upload
pub async fn init_chunked_upload(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ChunkedUploadInit>,
) -> Result<Json<ChunkedUploadInitResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::debug!("Initializing chunked upload for file: {}", payload.filename);
    let upload_id = Uuid::new_v4().to_string();
    let sanitized_filename = sanitize_filename(&payload.filename);
    
    let total_chunks = (payload.total_size as f64 / payload.chunk_size as f64).ceil() as usize;
    tracing::debug!("Calculated {} chunks for size {} (chunk size {})", total_chunks, payload.total_size, payload.chunk_size);
    
    let metadata = ChunkedUploadMetadata {
        filename: sanitized_filename.clone(),
        total_size: payload.total_size,
        chunk_size: payload.chunk_size,
        total_chunks,
        received_chunks: HashSet::new(),
    };
    
    state.chunked_uploads.insert(upload_id.clone(), metadata);
    
    // create temporary directory for chunks lmaooo????
    let chunks_dir = state.files_dir.join(".chunks").join(&upload_id);
    tracing::trace!("Creating chunks directory: {:?}", chunks_dir);
    
    fs::create_dir_all(&chunks_dir).await.map_err(|e| {
        tracing::error!("Failed to create chunks directory: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to create chunks directory: {}", e),
            }),
        )
    })?;
    
    tracing::info!("📤 Initialized chunked upload: {} (ID: {})", sanitized_filename, upload_id);
    
    Ok(Json(ChunkedUploadInitResponse {
        upload_id,
        chunk_size: payload.chunk_size,
        total_chunks,
    }))
}

// upload a single chunk
pub async fn upload_chunk(
    State(state): State<Arc<AppState>>,
    Path((upload_id, chunk_number)): Path<(String, usize)>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    tracing::trace!("Received chunk {} for upload {}", chunk_number, upload_id);
    
    // verify upload exists
    let mut metadata = state.chunked_uploads.get_mut(&upload_id).ok_or_else(|| {
        tracing::warn!("Upload ID not found: {}", upload_id);
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Upload ID not found".to_string(),
            }),
        )
    })?;
    
    // read chunk data
    let field = multipart.next_field().await.map_err(|e| {
        tracing::error!("Failed to read chunk data: {}", e);
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("Failed to read chunk data: {}", e),
            }),
        )
    })?.ok_or_else(|| {
        tracing::warn!("No chunk data provided");
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "No chunk data provided".to_string(),
            }),
        )
    })?;
    
    let data = field.bytes().await.map_err(|e| {
        tracing::error!("Failed to read chunk bytes: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to read chunk bytes: {}", e),
            }),
        )
    })?;
    
    // write chunk to temporary file
    let chunk_path = state.files_dir.join(".chunks").join(&upload_id).join(format!("chunk_{}", chunk_number));
    let mut file = fs::File::create(&chunk_path).await.map_err(|e| {
        tracing::error!("Failed to create chunk file: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to create chunk file: {}", e),
            }),
        )
    })?;
    
    file.write_all(&data).await.map_err(|e| {
        tracing::error!("Failed to write chunk: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to write chunk: {}", e),
            }),
        )
    })?;
    
    file.sync_all().await.map_err(|e| {
        tracing::error!("Failed to sync chunk: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to sync chunk: {}", e),
            }),
        )
    })?;
    
    // mark chunk as received
    metadata.received_chunks.insert(chunk_number);
    let received_count = metadata.received_chunks.len();
    let total_chunks = metadata.total_chunks;
    
    tracing::debug!("📦 Received chunk {}/{} for upload {}", chunk_number, total_chunks, upload_id);
    
    Ok(Json(serde_json::json!({
        "success": true,
        "chunk_number": chunk_number,
        "received_chunks": received_count,
        "total_chunks": total_chunks,
    })))
}

// complete a chunked upload by assembling all chunks
pub async fn complete_chunked_upload(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ChunkedUploadComplete>,
) -> Result<Json<ChunkedUploadCompleteResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::debug!("Completing chunked upload: {}", payload.upload_id);
    
    // get and remove metadata
    let (_, metadata) = state.chunked_uploads.remove(&payload.upload_id).ok_or_else(|| {
        tracing::warn!("Upload ID not found for completion: {}", payload.upload_id);
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Upload ID not found".to_string(),
            }),
        )
    })?;
    
    // verify all chunks received
    if metadata.received_chunks.len() != metadata.total_chunks {
        tracing::warn!("Incomplete upload: {}/{} chunks", metadata.received_chunks.len(), metadata.total_chunks);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!(
                    "Missing chunks: received {}/{}", 
                    metadata.received_chunks.len(), 
                    metadata.total_chunks
                ),
            }),
        ));
    }
    
    // assemble chunks into final file
    let final_path = state.files_dir.join(&metadata.filename);
    tracing::debug!("Assembling chunks into: {:?}", final_path);
    
    let mut final_file = fs::File::create(&final_path).await.map_err(|e| {
        tracing::error!("Failed to create final file: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to create final file: {}", e),
            }),
        )
    })?;
    
    let chunks_dir = state.files_dir.join(".chunks").join(&payload.upload_id);
    
    for chunk_num in 0..metadata.total_chunks {
        let chunk_path = chunks_dir.join(format!("chunk_{}", chunk_num));
        tracing::trace!("Reading chunk: {:?}", chunk_path);
        
        let chunk_data = fs::read(&chunk_path).await.map_err(|e| {
            tracing::error!("Failed to read chunk {}: {}", chunk_num, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to read chunk {}: {}", chunk_num, e),
                }),
            )
        })?;
        
        final_file.write_all(&chunk_data).await.map_err(|e| {
            tracing::error!("Failed to write chunk to final file: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to write chunk to final file: {}", e),
                }),
            )
        })?;
    }
    
    final_file.sync_all().await.map_err(|e| {
        tracing::error!("Failed to sync final file: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to sync final file: {}", e),
            }),
        )
    })?;
    
    // Clean up chunks directory
    tracing::debug!("Cleaning up chunks directory");
    let _ = fs::remove_dir_all(&chunks_dir).await;
    
    let final_size = final_file.metadata().await.map(|m| m.len()).unwrap_or(0);
    
    tracing::info!("✅ Completed chunked upload: {} ({} bytes)", metadata.filename, final_size);
    
    Ok(Json(ChunkedUploadCompleteResponse {
        success: true,
        filename: metadata.filename,
        size: final_size,
    }))
}




