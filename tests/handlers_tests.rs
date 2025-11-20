use juicebox_omega::handlers::{
    health_check, list_files, delete_file, get_stats, init_chunked_upload, 
    batch_delete_files, complete_chunked_upload
};
use juicebox_omega::state::AppState;
use juicebox_omega::models::{ChunkedUploadInit, BatchDeleteRequest, ChunkedUploadComplete};
use axum::extract::{State, Path};
use axum::Json;
use axum::http::StatusCode;
use std::sync::Arc;
use std::fs::File;
use std::io::Write;
use dashmap::DashMap;

#[tokio::test]
async fn test_health_check() {
    let response = health_check().await;
    assert_eq!(response.0["status"], "healthy");
}

#[tokio::test]
async fn test_list_files() {
    let temp_dir = tempfile::tempdir().unwrap();
    let state = Arc::new(AppState {
        files_dir: temp_dir.path().to_path_buf(),
        chunked_uploads: DashMap::new(),
    });

    // Empty dir
    let response = list_files(State(state.clone())).await.unwrap();
    assert_eq!(response.0.files.len(), 0);
    assert_eq!(response.0.total, 0);

    // Create a file
    let file_path = temp_dir.path().join("test.txt");
    let mut file = File::create(file_path).unwrap();
    writeln!(file, "hello world").unwrap();

    // List again
    let response = list_files(State(state.clone())).await.unwrap();
    assert_eq!(response.0.files.len(), 1);
    assert_eq!(response.0.files[0].name, "test.txt");
}

#[tokio::test]
async fn test_delete_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let state = Arc::new(AppState {
        files_dir: temp_dir.path().to_path_buf(),
        chunked_uploads: DashMap::new(),
    });

    // Create a file
    let file_path = temp_dir.path().join("delete_me.txt");
    File::create(&file_path).unwrap();

    // Delete it
    let response = delete_file(State(state.clone()), Path("delete_me.txt".to_string())).await.unwrap();
    assert!(response.0.success);
    assert!(!file_path.exists());

    // Delete non-existent
    let result = delete_file(State(state.clone()), Path("non_existent.txt".to_string())).await;
    assert!(result.is_err());
    assert_eq!(result.err().unwrap().0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_stats() {
    let temp_dir = tempfile::tempdir().unwrap();
    let state = Arc::new(AppState {
        files_dir: temp_dir.path().to_path_buf(),
        chunked_uploads: DashMap::new(),
    });

    // Create files
    let file1 = temp_dir.path().join("file1.txt");
    let mut f1 = File::create(file1).unwrap();
    f1.write_all(b"12345").unwrap(); // 5 bytes

    let file2 = temp_dir.path().join("file2.txt");
    let mut f2 = File::create(file2).unwrap();
    f2.write_all(b"1234567890").unwrap(); // 10 bytes

    let response = get_stats(State(state.clone())).await.unwrap();
    assert_eq!(response.0.total_files, 2);
    assert_eq!(response.0.total_size, 15);
}

#[tokio::test]
async fn test_init_chunked_upload() {
    let temp_dir = tempfile::tempdir().unwrap();
    let state = Arc::new(AppState {
        files_dir: temp_dir.path().to_path_buf(),
        chunked_uploads: DashMap::new(),
    });

    let payload = ChunkedUploadInit {
        filename: "large_file.bin".to_string(),
        total_size: 1024,
        chunk_size: 256,
    };

    let response = init_chunked_upload(State(state.clone()), Json(payload)).await.unwrap();
    assert_eq!(response.0.chunk_size, 256);
    assert_eq!(response.0.total_chunks, 4);
    
    // check if metadata is stored
    assert!(state.chunked_uploads.contains_key(&response.0.upload_id));
    
    // check if chunks dir is created
    let chunks_dir = temp_dir.path().join(".chunks").join(&response.0.upload_id);
    assert!(chunks_dir.exists());
}

#[tokio::test]
async fn test_batch_delete_files() {
    let temp_dir = tempfile::tempdir().unwrap();
    let state = Arc::new(AppState {
        files_dir: temp_dir.path().to_path_buf(),
        chunked_uploads: DashMap::new(),
    });

    // Create files
    let f1 = temp_dir.path().join("f1.txt");
    File::create(&f1).unwrap();
    let f2 = temp_dir.path().join("f2.txt");
    File::create(&f2).unwrap();

    let payload = BatchDeleteRequest {
        filenames: vec!["f1.txt".to_string(), "f2.txt".to_string(), "f3.txt".to_string()],
    };

    let response = batch_delete_files(State(state.clone()), Json(payload)).await;
    assert_eq!(response.0.total, 3);
    assert_eq!(response.0.successful, 2);
    assert_eq!(response.0.failed, 1);
    
    assert!(!f1.exists());
    assert!(!f2.exists());
}

#[tokio::test]
async fn test_complete_chunked_upload() {
    let temp_dir = tempfile::tempdir().unwrap();
    let state = Arc::new(AppState {
        files_dir: temp_dir.path().to_path_buf(),
        chunked_uploads: DashMap::new(),
    });

    let upload_id = "test-upload-id".to_string();
    let filename = "completed.txt".to_string();
    
    // Setup metadata
    let mut received_chunks = std::collections::HashSet::new();
    received_chunks.insert(0);
    received_chunks.insert(1);
    
    let metadata = juicebox_omega::state::ChunkedUploadMetadata {
        filename: filename.clone(),
        total_size: 10,
        chunk_size: 5,
        total_chunks: 2,
        received_chunks,
    };
    state.chunked_uploads.insert(upload_id.clone(), metadata);

    // Create chunks
    let chunks_dir = temp_dir.path().join(".chunks").join(&upload_id);
    std::fs::create_dir_all(&chunks_dir).unwrap();
    
    let mut c0 = File::create(chunks_dir.join("chunk_0")).unwrap();
    c0.write_all(b"hello").unwrap();
    
    let mut c1 = File::create(chunks_dir.join("chunk_1")).unwrap();
    c1.write_all(b"world").unwrap();

    let payload = ChunkedUploadComplete {
        upload_id: upload_id.clone(),
    };

    let response = complete_chunked_upload(State(state.clone()), Json(payload)).await.unwrap();
    assert!(response.0.success);
    assert_eq!(response.0.filename, filename);
    assert_eq!(response.0.size, 10);
    
    // Check final file
    let final_path = temp_dir.path().join(&filename);
    assert!(final_path.exists());
    let content = std::fs::read_to_string(final_path).unwrap();
    assert_eq!(content, "helloworld");
    
    // Check chunks dir removed
    assert!(!chunks_dir.exists());
}
