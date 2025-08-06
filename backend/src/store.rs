use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Seek, SeekFrom, Write},
    path::Path,
    sync::RwLock,
};

// Metadata record type
#[derive(Serialize, Deserialize, Clone)]
pub struct ReviewMeta {
    pub id: usize,
    pub title: String,
    pub body: String,
}

// Holds file handles and offsets
pub struct Store {
    idx_path: String,
    meta_path: String,
    // vector file in append-only binary mode
    idx_file: RwLock<File>,
    // metadata file in append-only text mode
    meta_file: RwLock<File>,
    // byte offsets of each JSON line
    offsets: RwLock<Vec<u64>>,
}

impl Store {
    pub fn new(idx_path: &str, meta_path: &str) -> anyhow::Result<Self> {
        // Open index file
        let idx_file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(idx_path)?;
        // Open metadata file
        let meta_file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(meta_path)?;
        // Scan existing metadata for offsets
        let mut offsets = Vec::new();
        let mut reader = BufReader::new(&meta_file);
        let mut pos = 0u64;
        loop {
            let len = reader.read_line(&mut String::new())?;
            if len == 0 { break; }
            offsets.push(pos);
            pos += len as u64;
        }
        Ok(Self {
            idx_path: idx_path.to_string(),
            meta_path: meta_path.to_string(),
            idx_file: RwLock::new(idx_file),
            meta_file: RwLock::new(meta_file),
            offsets: RwLock::new(offsets),
        })
    }

    /// Append one review: write vector bytes then JSON line
    pub fn append_review(
        &self,
        vector: &[f32],
        meta: &ReviewMeta,
    ) -> anyhow::Result<usize> {
        // Lock files for thread-safe append
        let mut idx_f = self.idx_file.write().unwrap();
        let mut meta_f = self.meta_file.write().unwrap();

        idx_f.lock_exclusive()?;  // block other writers
        meta_f.lock_exclusive()?;

        // Write vector: simple f32 bytes sequence
        for &v in vector {
            idx_f.write_all(&v.to_le_bytes())?;
        }
        idx_f.flush()?;

        // Serialize and append metadata
        let mut line = serde_json::to_string(meta)? + "\n";
        let offset = meta_f.seek(SeekFrom::End(0))?;
        meta_f.write_all(line.as_bytes())?;
        meta_f.flush()?;

        // Update in-memory offsets
        self.offsets.write().unwrap().push(offset);

        meta_f.unlock()?;
        idx_f.unlock()?;

        Ok(meta.id)
    }

    /// Load metadata by vector-id
    pub fn load_meta(&self, vid: usize) -> anyhow::Result<ReviewMeta> {
        let offset = {
            let o = self.offsets.read().unwrap();
            *o.get(vid).ok_or_else(|| anyhow::anyhow!("id out of range"))?
        };
        let mut f = File::open(&self.meta_path)?;
        f.seek(SeekFrom::Start(offset))?;
        let mut reader = BufReader::new(f);
        let mut line = String::new();
        reader.read_line(&mut line)?;
        let meta: ReviewMeta = serde_json::from_str(&line)?;
        Ok(meta)
    }
}