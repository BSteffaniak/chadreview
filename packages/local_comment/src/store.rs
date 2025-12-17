//! XDG-compliant file-based storage for local comments.

use std::collections::HashMap;
use std::fmt::Write;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use sha2::{Digest, Sha256};
use switchy::uuid::Uuid;

use chadreview_local_comment_models::{
    AiExecutionStatus, CommentThreadIndex, LocalComment, LocalCommentType,
};

/// Errors that can occur when using the local comment store.
#[derive(Debug, thiserror::Error)]
pub enum LocalCommentStoreError {
    /// Failed to create storage directory.
    #[error("Failed to create storage directory: {0}")]
    CreateDir(std::io::Error),

    /// Failed to read from storage.
    #[error("Failed to read from storage: {0}")]
    Read(std::io::Error),

    /// Failed to write to storage.
    #[error("Failed to write to storage: {0}")]
    Write(std::io::Error),

    /// Failed to parse stored data.
    #[error("Failed to parse stored data: {0}")]
    Parse(serde_json::Error),

    /// Failed to serialize data.
    #[error("Failed to serialize data: {0}")]
    Serialize(serde_json::Error),

    /// Comment not found.
    #[error("Comment not found: {0}")]
    NotFound(Uuid),

    /// Could not determine data directory.
    #[error("Could not determine XDG data directory")]
    NoDataDir,
}

/// XDG-compliant file-based storage for local comments.
///
/// Storage layout:
/// ```text
/// $XDG_DATA_HOME/chadreview/comments/{repo-hash}/
/// ├── index.json          # Thread index for fast listing
/// └── threads/
///     ├── {uuid}.json     # Individual comment threads
///     └── ...
/// ```
pub struct LocalCommentStore {
    /// Base path for this repository's comments.
    repo_path: PathBuf,
    /// In-memory cache of comment threads (thread-safe).
    cache: RwLock<HashMap<Uuid, LocalComment>>,
}

impl LocalCommentStore {
    /// Create a new store for the given repository path.
    ///
    /// # Errors
    ///
    /// Returns an error if the XDG data directory cannot be determined.
    pub fn new(repo_path: &Path) -> Result<Self, LocalCommentStoreError> {
        let base_path = Self::get_storage_path(repo_path)?;

        Ok(Self {
            repo_path: base_path,
            cache: RwLock::new(HashMap::new()),
        })
    }

    /// Get the XDG-compliant storage path for a repository.
    fn get_storage_path(repo_path: &Path) -> Result<PathBuf, LocalCommentStoreError> {
        let data_dir = dirs::data_dir().ok_or(LocalCommentStoreError::NoDataDir)?;

        // Create a hash of the canonical repo path for the directory name
        let canonical = repo_path
            .canonicalize()
            .unwrap_or_else(|_| repo_path.to_path_buf());
        let repo_hash = Self::hash_path(&canonical);

        Ok(data_dir.join("chadreview").join("comments").join(repo_hash))
    }

    /// Hash a path to create a directory-safe identifier.
    fn hash_path(path: &Path) -> String {
        let mut hasher = Sha256::new();
        hasher.update(path.to_string_lossy().as_bytes());
        let result = hasher.finalize();
        // Use first 16 bytes (32 hex chars) for reasonable uniqueness
        result[..16]
            .iter()
            .fold(String::with_capacity(32), |mut acc, b| {
                write!(acc, "{b:02x}").unwrap();
                acc
            })
    }

    /// Ensure storage directories exist.
    fn ensure_dirs(&self) -> Result<(), LocalCommentStoreError> {
        let threads_dir = self.repo_path.join("threads");
        fs::create_dir_all(&threads_dir).map_err(LocalCommentStoreError::CreateDir)?;
        Ok(())
    }

    /// Get the path to a thread file.
    fn thread_path(&self, id: Uuid) -> PathBuf {
        self.repo_path.join("threads").join(format!("{id}.json"))
    }

    /// Get the path to the index file.
    fn index_path(&self) -> PathBuf {
        self.repo_path.join("index.json")
    }

    /// Save a comment thread to disk.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to disk fails.
    pub fn save_thread(&self, comment: &LocalComment) -> Result<(), LocalCommentStoreError> {
        self.ensure_dirs()?;

        // Save thread file
        let path = self.thread_path(comment.id);
        let file = File::create(&path).map_err(LocalCommentStoreError::Write)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, comment).map_err(LocalCommentStoreError::Serialize)?;

        // Update cache
        if let Ok(mut cache) = self.cache.write() {
            cache.insert(comment.id, comment.clone());
        }

        // Update index
        self.update_index(comment)?;

        Ok(())
    }

    /// Load a comment thread from disk.
    ///
    /// # Errors
    ///
    /// Returns an error if the thread doesn't exist or can't be read.
    pub fn load_thread(&self, id: Uuid) -> Result<LocalComment, LocalCommentStoreError> {
        // Check cache first
        if let Ok(cache) = self.cache.read()
            && let Some(comment) = cache.get(&id)
        {
            return Ok(comment.clone());
        }

        // Load from disk
        let path = self.thread_path(id);
        if !path.exists() {
            return Err(LocalCommentStoreError::NotFound(id));
        }

        let file = File::open(&path).map_err(LocalCommentStoreError::Read)?;
        let reader = BufReader::new(file);
        let comment: LocalComment =
            serde_json::from_reader(reader).map_err(LocalCommentStoreError::Parse)?;

        // Update cache
        if let Ok(mut cache) = self.cache.write() {
            cache.insert(id, comment.clone());
        }

        Ok(comment)
    }

    /// Get a comment by ID (may be a nested reply).
    ///
    /// # Errors
    ///
    /// Returns an error if the comment is not found.
    pub fn get_comment(&self, id: Uuid) -> Result<LocalComment, LocalCommentStoreError> {
        // First try loading as a root thread
        if let Ok(comment) = self.load_thread(id) {
            return Ok(comment);
        }

        // Search through all threads for a reply with this ID
        for entry in self.list_threads()? {
            let thread = self.load_thread(entry.id)?;
            if let Some(comment) = Self::find_comment_in_thread(&thread, id) {
                return Ok(comment);
            }
        }

        Err(LocalCommentStoreError::NotFound(id))
    }

    /// Find a comment within a thread (including replies).
    fn find_comment_in_thread(comment: &LocalComment, id: Uuid) -> Option<LocalComment> {
        if comment.id == id {
            return Some(comment.clone());
        }
        for reply in &comment.replies {
            if let Some(found) = Self::find_comment_in_thread(reply, id) {
                return Some(found);
            }
        }
        None
    }

    /// Delete a comment thread.
    ///
    /// # Errors
    ///
    /// Returns an error if the thread doesn't exist or can't be deleted.
    pub fn delete_thread(&self, id: Uuid) -> Result<(), LocalCommentStoreError> {
        let path = self.thread_path(id);
        if !path.exists() {
            return Err(LocalCommentStoreError::NotFound(id));
        }

        fs::remove_file(&path).map_err(LocalCommentStoreError::Write)?;

        // Remove from cache
        if let Ok(mut cache) = self.cache.write() {
            cache.remove(&id);
        }

        // Remove from index
        self.remove_from_index(id)?;

        Ok(())
    }

    /// Update the thread index.
    fn update_index(&self, comment: &LocalComment) -> Result<(), LocalCommentStoreError> {
        let mut index = self.load_index().unwrap_or_default();

        // Update or add entry
        let entry = comment.to_index_entry();
        if let Some(existing) = index.iter_mut().find(|e| e.id == comment.id) {
            *existing = entry;
        } else {
            index.push(entry);
        }

        self.save_index(&index)
    }

    /// Remove a thread from the index.
    fn remove_from_index(&self, id: Uuid) -> Result<(), LocalCommentStoreError> {
        let mut index = self.load_index().unwrap_or_default();
        index.retain(|e| e.id != id);
        self.save_index(&index)
    }

    /// Load the thread index.
    fn load_index(&self) -> Result<Vec<CommentThreadIndex>, LocalCommentStoreError> {
        let path = self.index_path();
        if !path.exists() {
            return Ok(vec![]);
        }

        let file = File::open(&path).map_err(LocalCommentStoreError::Read)?;
        let reader = BufReader::new(file);
        serde_json::from_reader(reader).map_err(LocalCommentStoreError::Parse)
    }

    /// Save the thread index.
    fn save_index(&self, index: &[CommentThreadIndex]) -> Result<(), LocalCommentStoreError> {
        self.ensure_dirs()?;

        let path = self.index_path();
        let file = File::create(&path).map_err(LocalCommentStoreError::Write)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, index).map_err(LocalCommentStoreError::Serialize)
    }

    /// List all comment threads.
    ///
    /// # Errors
    ///
    /// Returns an error if the index can't be read.
    pub fn list_threads(&self) -> Result<Vec<CommentThreadIndex>, LocalCommentStoreError> {
        self.load_index()
    }

    /// List threads for a specific file.
    ///
    /// # Errors
    ///
    /// Returns an error if the index can't be read.
    pub fn list_threads_for_file(
        &self,
        path: &str,
    ) -> Result<Vec<CommentThreadIndex>, LocalCommentStoreError> {
        let index = self.load_index()?;
        Ok(index
            .into_iter()
            .filter(|entry| match &entry.comment_type {
                LocalCommentType::FileLevelComment { path: p }
                | LocalCommentType::LineLevelComment { path: p, .. } => p == path,
                _ => false,
            })
            .collect())
    }

    /// List threads for a specific line.
    ///
    /// # Errors
    ///
    /// Returns an error if the index can't be read.
    pub fn list_threads_for_line(
        &self,
        path: &str,
        line: chadreview_local_comment_models::LineNumber,
    ) -> Result<Vec<CommentThreadIndex>, LocalCommentStoreError> {
        let index = self.load_index()?;
        Ok(index
            .into_iter()
            .filter(|entry| {
                matches!(
                    &entry.comment_type,
                    LocalCommentType::LineLevelComment { path: p, line: l, .. }
                    if p == path && *l == line
                )
            })
            .collect())
    }

    /// Add a reply to an existing thread.
    ///
    /// # Errors
    ///
    /// Returns an error if the thread doesn't exist or can't be updated.
    pub fn add_reply(
        &self,
        thread_id: Uuid,
        reply: LocalComment,
    ) -> Result<(), LocalCommentStoreError> {
        let mut thread = self.load_thread(thread_id)?;
        thread.replies.push(reply);
        thread.updated_at = chrono::Utc::now();
        self.save_thread(&thread)
    }

    /// Update the AI execution status for a comment.
    ///
    /// # Errors
    ///
    /// Returns an error if the thread doesn't exist or can't be updated.
    pub fn update_ai_status(
        &self,
        thread_id: Uuid,
        status: AiExecutionStatus,
    ) -> Result<(), LocalCommentStoreError> {
        let mut thread = self.load_thread(thread_id)?;
        thread.ai_status = Some(status);
        thread.updated_at = chrono::Utc::now();
        self.save_thread(&thread)
    }

    /// Update the AI execution status for a comment (root or reply).
    ///
    /// This method handles both root comments and nested replies by loading
    /// the thread and finding the appropriate comment to update.
    ///
    /// # Arguments
    /// * `thread_id` - The root thread ID
    /// * `comment_id` - The comment to update (can be the same as `thread_id` for root comments)
    /// * `status` - The new AI execution status
    ///
    /// # Errors
    ///
    /// Returns an error if the thread doesn't exist, the comment isn't found, or can't be updated.
    pub fn update_reply_ai_status(
        &self,
        thread_id: Uuid,
        comment_id: Uuid,
        status: AiExecutionStatus,
    ) -> Result<(), LocalCommentStoreError> {
        let mut thread = self.load_thread(thread_id)?;

        if thread_id == comment_id {
            // Updating the root comment itself
            thread.ai_status = Some(status);
        } else {
            // Find and update the reply
            if !Self::update_reply_status_recursive(&mut thread.replies, comment_id, &status) {
                return Err(LocalCommentStoreError::NotFound(comment_id));
            }
        }

        thread.updated_at = chrono::Utc::now();
        self.save_thread(&thread)
    }

    /// Recursively find and update a reply's AI status.
    fn update_reply_status_recursive(
        replies: &mut [LocalComment],
        target_id: Uuid,
        status: &AiExecutionStatus,
    ) -> bool {
        for reply in replies.iter_mut() {
            if reply.id == target_id {
                reply.ai_status = Some(status.clone());
                return true;
            }
            if Self::update_reply_status_recursive(&mut reply.replies, target_id, status) {
                return true;
            }
        }
        false
    }

    /// Update the `OpenCode` session ID for a thread.
    ///
    /// This is used to continue conversations with `OpenCode` by passing
    /// the session ID to subsequent executions.
    ///
    /// # Arguments
    /// * `thread_id` - The root thread ID
    /// * `session_id` - The `OpenCode` session ID
    ///
    /// # Errors
    ///
    /// Returns an error if the thread doesn't exist or can't be updated.
    pub fn update_session_id(
        &self,
        thread_id: Uuid,
        session_id: String,
    ) -> Result<(), LocalCommentStoreError> {
        let mut thread = self.load_thread(thread_id)?;
        thread.opencode_session_id = Some(session_id);
        thread.updated_at = chrono::Utc::now();
        self.save_thread(&thread)
    }

    /// Delete a reply from a thread.
    ///
    /// This removes a specific reply (and all its nested replies) from a thread.
    ///
    /// # Arguments
    /// * `thread_id` - The root thread ID
    /// * `reply_id` - The reply to delete
    ///
    /// # Errors
    ///
    /// Returns an error if the thread doesn't exist, the reply isn't found, or can't be updated.
    pub fn delete_reply(
        &self,
        thread_id: Uuid,
        reply_id: Uuid,
    ) -> Result<(), LocalCommentStoreError> {
        let mut thread = self.load_thread(thread_id)?;

        if !Self::remove_reply_recursive(&mut thread.replies, reply_id) {
            return Err(LocalCommentStoreError::NotFound(reply_id));
        }

        thread.updated_at = chrono::Utc::now();
        self.save_thread(&thread)
    }

    /// Recursively find and remove a reply from a thread.
    fn remove_reply_recursive(replies: &mut Vec<LocalComment>, target_id: Uuid) -> bool {
        // First check if it's a direct child
        if let Some(pos) = replies.iter().position(|r| r.id == target_id) {
            replies.remove(pos);
            return true;
        }
        // Otherwise recurse into children
        for reply in replies.iter_mut() {
            if Self::remove_reply_recursive(&mut reply.replies, target_id) {
                return true;
            }
        }
        false
    }

    /// Get the storage base path for this repository.
    #[must_use]
    pub fn storage_path(&self) -> &Path {
        &self.repo_path
    }

    // =========================================================================
    // Viewed Files
    // =========================================================================

    /// Get the path to the viewed files index.
    fn viewed_files_path(&self) -> PathBuf {
        self.repo_path.join("viewed_files.json")
    }

    /// Load the viewed files index.
    ///
    /// Returns an empty index if the file doesn't exist.
    ///
    /// # Errors
    ///
    /// Returns an error if the file exists but cannot be read or parsed.
    pub fn load_viewed_files(
        &self,
    ) -> Result<chadreview_local_comment_models::ViewedFilesIndex, LocalCommentStoreError> {
        let path = self.viewed_files_path();

        if !path.exists() {
            return Ok(chadreview_local_comment_models::ViewedFilesIndex::default());
        }

        let file = File::open(&path).map_err(LocalCommentStoreError::Read)?;
        let reader = BufReader::new(file);
        serde_json::from_reader(reader).map_err(LocalCommentStoreError::Parse)
    }

    /// Save the viewed files index.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written.
    pub fn save_viewed_files(
        &self,
        index: &chadreview_local_comment_models::ViewedFilesIndex,
    ) -> Result<(), LocalCommentStoreError> {
        // Ensure directory exists
        fs::create_dir_all(&self.repo_path).map_err(LocalCommentStoreError::CreateDir)?;

        let path = self.viewed_files_path();
        let file = File::create(&path).map_err(LocalCommentStoreError::Write)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, index).map_err(LocalCommentStoreError::Serialize)
    }

    /// Mark a file as viewed.
    ///
    /// # Errors
    ///
    /// Returns an error if the viewed files index cannot be loaded or saved.
    pub fn mark_file_viewed(&self, path: &str) -> Result<(), LocalCommentStoreError> {
        let mut index = self.load_viewed_files()?;
        index.files.insert(path.to_string(), chrono::Utc::now());
        self.save_viewed_files(&index)
    }

    /// Mark a file as not viewed.
    ///
    /// # Errors
    ///
    /// Returns an error if the viewed files index cannot be loaded or saved.
    pub fn mark_file_unviewed(&self, path: &str) -> Result<(), LocalCommentStoreError> {
        let mut index = self.load_viewed_files()?;
        index.files.remove(path);
        self.save_viewed_files(&index)
    }

    /// Check if a file is marked as viewed.
    ///
    /// Returns `false` if the file is not viewed or if there's an error loading the index.
    #[must_use]
    pub fn is_file_viewed(&self, path: &str) -> bool {
        self.load_viewed_files()
            .map(|index| index.files.contains_key(path))
            .unwrap_or(false)
    }

    /// Get the set of all viewed file paths.
    ///
    /// # Errors
    ///
    /// Returns an error if the viewed files index cannot be loaded.
    pub fn get_viewed_file_paths(
        &self,
    ) -> Result<std::collections::HashSet<String>, LocalCommentStoreError> {
        let index = self.load_viewed_files()?;
        Ok(index.files.keys().cloned().collect())
    }

    // =========================================================================
    // Viewed Replies
    // =========================================================================

    /// Get the path to the viewed replies index.
    fn viewed_replies_path(&self) -> PathBuf {
        self.repo_path.join("viewed_replies.json")
    }

    /// Load the viewed replies index.
    ///
    /// Returns an empty index if the file doesn't exist.
    ///
    /// # Errors
    ///
    /// Returns an error if the file exists but cannot be read or parsed.
    pub fn load_viewed_replies(
        &self,
    ) -> Result<chadreview_local_comment_models::ViewedRepliesIndex, LocalCommentStoreError> {
        let path = self.viewed_replies_path();

        if !path.exists() {
            return Ok(chadreview_local_comment_models::ViewedRepliesIndex::default());
        }

        let file = File::open(&path).map_err(LocalCommentStoreError::Read)?;
        let reader = BufReader::new(file);
        serde_json::from_reader(reader).map_err(LocalCommentStoreError::Parse)
    }

    /// Save the viewed replies index.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written.
    pub fn save_viewed_replies(
        &self,
        index: &chadreview_local_comment_models::ViewedRepliesIndex,
    ) -> Result<(), LocalCommentStoreError> {
        // Ensure directory exists
        fs::create_dir_all(&self.repo_path).map_err(LocalCommentStoreError::CreateDir)?;

        let path = self.viewed_replies_path();
        let file = File::create(&path).map_err(LocalCommentStoreError::Write)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, index).map_err(LocalCommentStoreError::Serialize)
    }

    /// Mark a reply as viewed.
    ///
    /// # Errors
    ///
    /// Returns an error if the viewed replies index cannot be loaded or saved.
    pub fn mark_reply_viewed(&self, reply_id: Uuid) -> Result<(), LocalCommentStoreError> {
        let mut index = self.load_viewed_replies()?;
        index.replies.insert(reply_id, chrono::Utc::now());
        self.save_viewed_replies(&index)
    }

    /// Mark a reply as not viewed.
    ///
    /// # Errors
    ///
    /// Returns an error if the viewed replies index cannot be loaded or saved.
    pub fn mark_reply_unviewed(&self, reply_id: Uuid) -> Result<(), LocalCommentStoreError> {
        let mut index = self.load_viewed_replies()?;
        index.replies.remove(&reply_id);
        self.save_viewed_replies(&index)
    }

    /// Check if a reply is marked as viewed.
    ///
    /// Returns `false` if the reply is not viewed or if there's an error loading the index.
    #[must_use]
    pub fn is_reply_viewed(&self, reply_id: Uuid) -> bool {
        self.load_viewed_replies()
            .map(|index| index.replies.contains_key(&reply_id))
            .unwrap_or(false)
    }

    /// Get the set of all viewed reply IDs.
    ///
    /// # Errors
    ///
    /// Returns an error if the viewed replies index cannot be loaded.
    pub fn get_viewed_reply_ids(
        &self,
    ) -> Result<std::collections::HashSet<Uuid>, LocalCommentStoreError> {
        let index = self.load_viewed_replies()?;
        Ok(index.replies.keys().copied().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chadreview_local_comment_models::{LineNumber, LocalUser};
    use std::env;

    fn temp_store() -> LocalCommentStore {
        let temp_dir = env::temp_dir().join(format!("chadreview-test-{}", Uuid::new_v4()));
        LocalCommentStore {
            repo_path: temp_dir,
            cache: RwLock::new(HashMap::new()),
        }
    }

    #[test]
    fn test_save_and_load_thread() {
        let store = temp_store();

        let comment = LocalComment::new(
            LocalUser::default(),
            "Test comment".to_string(),
            LocalCommentType::General,
        );

        store.save_thread(&comment).unwrap();
        let loaded = store.load_thread(comment.id).unwrap();

        assert_eq!(loaded.id, comment.id);
        assert_eq!(loaded.body, comment.body);
    }

    #[test]
    fn test_list_threads() {
        let store = temp_store();

        let comment1 = LocalComment::new(
            LocalUser::default(),
            "Comment 1".to_string(),
            LocalCommentType::General,
        );
        let comment2 = LocalComment::new(
            LocalUser::default(),
            "Comment 2".to_string(),
            LocalCommentType::LineLevelComment {
                path: "src/main.rs".to_string(),
                line: LineNumber::New { line: 10 },
            },
        );

        store.save_thread(&comment1).unwrap();
        store.save_thread(&comment2).unwrap();

        let threads = store.list_threads().unwrap();
        assert_eq!(threads.len(), 2);
    }

    #[test]
    fn test_delete_thread() {
        let store = temp_store();

        let comment = LocalComment::new(
            LocalUser::default(),
            "To be deleted".to_string(),
            LocalCommentType::General,
        );

        store.save_thread(&comment).unwrap();
        assert!(store.load_thread(comment.id).is_ok());

        store.delete_thread(comment.id).unwrap();
        assert!(store.load_thread(comment.id).is_err());
    }
}
