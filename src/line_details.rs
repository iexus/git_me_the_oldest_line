use chrono::prelude::*;
use std::fmt;

#[derive(Debug, Clone)]
pub struct LineDetails {
    pub file_name: String,
    pub original_filename: String,
    pub commit_hash: String,
    pub author: String,
    pub datetime: DateTime<Utc>,
    pub code: String,
}

impl Default for LineDetails {
    fn default() -> Self {
        LineDetails{
            file_name: String::default(),
            original_filename: String::default(),
            author: String::default(),
            commit_hash: String::default(),
            datetime: Utc::now(),
            code: String::default(),
        }
    }
}

impl fmt::Display for LineDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f, "File: {} ({})\nHash: {}\nAuthor: {}\nDate: {}\nCode:\n{}\n",
            self.file_name, self.original_filename, self.commit_hash, self.author, self.datetime, self.code
        )
    }
}
