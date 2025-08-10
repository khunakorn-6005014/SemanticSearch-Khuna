//backend/src/store.rs
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, RwLock},
};
use thiserror::Error;

// Metadata record type
#[derive(Debug,Serialize, Deserialize, Clone)]
pub struct ReviewMeta {
    pub review_title: String,
    pub review_body: String,
    pub product_id: String,
    pub review_rating: i32,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct NewReview {
    pub review_title: String,
    pub review_body: String,
    pub product_id: String,
    pub review_rating: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    #[serde(default = "default_top_k")]
    pub top_k: usize,
}
fn default_top_k() -> usize { 5 }

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchHit {
    pub id: u64,
    pub score: f32,
    pub review: Review,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResponse {
    pub hits: Vec<SearchHit>,
}
// ---- Errors ----

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("index error: {0}")]
    Index(String),
    #[error("invalid id {0}")]
    InvalidId(u64),
}
// ---- Embedder + Index traits
pub trait VectorIndex: Send + Sync {
    // Append a vector; returns assigned id (0-based).
    fn append(&mut self, vector: &[f32]) -> Result<u64, StoreError>;
    // Search; returns (id, score) pairs sorted by score desc.
    fn search(&self, vector: &[f32], top_k: usize) -> Result<Vec<(u64, f32)>, StoreError>;
    // Optional: number of vectors currently stored.
    fn len(&self) -> Result<u64, StoreError>;
}

// Holds file handles and offsets
pub struct MetadataStore {
    jsonl_path: PathBuf,
    offsets_path: PathBuf,
    vectors_path: PathBuf,
    // Append file handles guarded by a single mutex to keep ordering atomic.
    jsonl_file: Mutex<File>,
    offsets_file: Mutex<File>,
    vectors_file: Mutex<File>,
    // In-memory offsets for fast id -> byte position.
    offsets: RwLock<Vec<u64>>,
}

impl MetadataStore {
    pub fn open<P: AsRef<Path>>(dir: P) -> Result<Self, StoreError> {
        let dir = dir.as_ref();
        std::fs::create_dir_all(dir)?;

        let jsonl_path = dir.join("reviews.jsonl");
        let offsets_path = dir.join("reviews.offsets");

        // Open index file
        let mut jsonl_file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(&jsonl_path)?;
        // Open metadata file
        let mut offsets_file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&offsets_path)?;
        // Load offsets into memory (or rebuild if missing)
        let mut offsets = Vec::new();
        // If offsets file is empty but jsonl has data, rebuild offsets by scanning once.
        let offsets_len = offsets_file.metadata()?.len();
        if offsets_len == 0 {
            // Build offsets by streaming jsonl and recording starting byte of each line.
            let mut pos: u64 = 0;
            let mut reader = BufReader::new(
                OpenOptions::new().read(true).open(&jsonl_path)?
            );
            loop {
                let start = pos;
                let mut line = String::new();
                let read = reader.read_line(&mut line)?;
                if read == 0 { break; }
                offsets.push(start);
                pos += read as u64;
            }
            // Persist rebuilt offsets
            {
                let mut w = &offsets_file;
                for off in &offsets {
                    w.write_all(&off.to_le_bytes())?;
                }
                w.flush()?;
            }
        } else {
            // Load existing offsets as u64 chunks
            let mut buf = Vec::new();
            offsets_file.seek(SeekFrom::Start(0))?;
            offsets_file.read_to_end(&mut buf)?;
            for chunk in buf.chunks_exact(8) {
                let mut bytes = [0u8; 8];
                bytes.copy_from_slice(chunk);
                offsets.push(u64::from_le_bytes(bytes));
            }
            // Seek append handles to end again
            offsets_file.seek(SeekFrom::End(0))?;
        }

        // Ensure jsonl append handle at end
        jsonl_file.seek(SeekFrom::End(0))?;

        Ok(Self {
            jsonl_path,
            offsets_path,
            jsonl_file: Mutex::new(jsonl_file),
            offsets_file: Mutex::new(offsets_file),
            offsets: RwLock::new(offsets),
        })
    }
    pub fn count(&self) -> u64 {
        self.offsets.read().unwrap().len() as u64
    }
    /// Append one review: write vector bytes then JSON line
    // Append one review; returns assigned id (line number).
    pub fn append(&self, review: &Review) -> Result<u64, StoreError> {
         // Serialize line with trailing newline
        let mut line = serde_json::to_vec(review)?;
        line.push(b'\n');
        /// Lock both files for a single atomic append sequence
        let mut jf = self.jsonl_file.lock().unwrap();
        let mut of = self.offsets_file.lock().unwrap();
        
        // Current byte length = next line's starting offset
        let start_offset = jf.metadata()?.len();
     //Write vector bytes first (optional ordering; choose one policy and keep it)
        for &v in vector {
           vf.write_all(&v.to_le_bytes())?;
        }
            vf.flush()?;
        // Write JSONL
        jf.write_all(&line)?;
        jf.flush()?;

        // Update offsets on disk , Record JSON line start offset
        of.write_all(&start_offset.to_le_bytes())?;
        of.flush()?;
        // Update in-memory and return id
        let mut offs = self.offsets.write().unwrap();
        offs.push(start_offset);
     Ok((offs.len() - 1) as u64)
    }

    // Fetch by id using offsets to seek directly
    pub fn get(&self, id: u64) -> Result<Review, StoreError> {
        let offs = self.offsets.read().unwrap();
        let idx = id as usize;
        if idx >= offs.len() {
            return Err(StoreError::InvalidId(id));
        }
        let start = offs[idx];
        let end = if idx + 1 < offs.len() { offs[idx + 1] } else {
            // If last record, end = file length
            std::fs::metadata(&self.jsonl_path)?.len()
        };

        let len = (end - start) as usize;
        let mut f = File::open(&self.jsonl_path)?;
        f.seek(SeekFrom::Start(start))?;
        let mut buf = vec![0u8; len];
        f.read_exact(&mut buf)?;

        // Strip trailing newline just in case
        if let Some(&b'\n') = buf.last() {
            buf.pop();
        }

        let review: Review = serde_json::from_slice(&buf)?;
        Ok(review)
    }
}