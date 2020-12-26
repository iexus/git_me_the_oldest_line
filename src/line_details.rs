use chrono::prelude::*;
use regex::Regex;
use std::fmt;

lazy_static! {
    // AVERT YOUR EYES CHILDREN
    static ref LINE_MATCHER: Regex = Regex::new(
        r"(?P<commit_hash>(^.{40})) (?P<original_filename>(.+)) \((?P<author>(.+))(?P<datetime>(\d{4}-\d{2}-\d{2}[T\s]?\d{2}:\d{2}:\d{2}\s?[+-]\d{2}:?\d{2})).+\d{1}\)(?P<code>(.+$))"
    ).unwrap();
}

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

pub fn parse_line(line: &str, file_name: &str) -> Option<LineDetails> {
    match &LINE_MATCHER.captures(line) {
        None => None,
        Some(capture) => {
            let commit_hash = capture.name("commit_hash")?.as_str();
            let original_filename = capture.name("original_filename")?.as_str();
            let author = capture.name("author")?.as_str().trim();
            let code = capture.name("code")?.as_str();

            let raw_datetime = capture.name("datetime")?.as_str();
            let datetime = raw_datetime.parse::<DateTime<Utc>>()
                .expect(&format!("Could not parse date time: {}", raw_datetime));

            Some(LineDetails{
                file_name: file_name.to_string(),
                original_filename: original_filename.to_string(),
                commit_hash: commit_hash.to_string(),
                author: author.to_string(),
                datetime: datetime,
                code: code.to_string(),
            })
        }
    }
}
