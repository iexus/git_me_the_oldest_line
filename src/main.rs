use chrono::prelude::*;
use std::fmt;
use regex::Regex;
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader, Error, ErrorKind};

#[derive(Debug, Clone)]
struct LineDetails {
    file_name: String,
    original_filename: String,
    commit_hash: String,
    author: String,
    datetime: DateTime<Utc>,
    code: String,
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

fn main() {
    match gather_files() {
        Ok(()) => println!("Success"),
        Err(error) => panic!("Problem occurred: {}", error),
    };
}

fn gather_files() -> Result<(), Error> {
    // Get a list of every file that git tracks
    let gitls_stdout = Command::new("git")
        .args(&["ls-tree", "-r", "--name-only", "HEAD"])
        .stdout(Stdio::piped())
        .spawn()?
        .stdout
        .ok_or_else(|| Error::new(ErrorKind::Other,"Could not capture standard output."))?;

    // AVERT YOUR EYES CHILDREN
    let line_regex = Regex::new(
        r"(?P<commit_hash>(^.{40})) (?P<original_filename>(.+)) \((?P<author>(.+))(?P<datetime>(\d{4}-\d{2}-\d{2}[T\s]?\d{2}:\d{2}:\d{2}\s?[+-]\d{2}:?\d{2})).+\d{1}\)(?P<code>(.+$))"
    ).unwrap();

    // For each one run a git blame on it.
    let reader = BufReader::new(gitls_stdout);
    let mut oldest_line_so_far = LineDetails::default();
    reader
        .lines()
        .filter_map(|line| line.ok())
        .for_each(|file_name| {
            match blame_file(file_name.clone(), &line_regex) {
                Ok(details) => {
                    if details.datetime < oldest_line_so_far.datetime {
                        oldest_line_so_far = details;
                    }
                },
                Err(error) => {
                    println!(
                        "Encountered error getting details for line: {}, with error: {}",
                        file_name, error
                    )
                }
            }
        });

    println!("{}", oldest_line_so_far);
    Ok(())
}

fn blame_file(file_name: String, line_regex: &regex::Regex) -> Result<LineDetails, Error> {
    // -l is for the long commit reference
    // -f to always show the file name of where the code came from (movement tracking)
    // -M and -C are related to tracking down code movements to the original commit
    // rather than just the latest that touched them
    let git_blame_stdout = Command::new("git")
        .args(&["blame", "-l", "-f", "-M", "-C", &file_name])
        .stdout(Stdio::piped())
        .spawn()?
        .stdout
        .ok_or_else(|| Error::new(ErrorKind::Other,"Could not capture standard output."))?;


    let mut oldest_line_so_far = LineDetails::default();
    let reader = BufReader::new(git_blame_stdout);
    reader
        .lines()
        .filter_map(|line| line.ok())
        .for_each(|line| {
            match parse_line(&line_regex, &line, &file_name) {
                Some(details) => {
                    if details.datetime < oldest_line_so_far.datetime {
                        oldest_line_so_far = details.clone();
                    }
                },
                None => panic!("Could not create details from line: {}", line),
            }
        });

    Ok(oldest_line_so_far)
}

fn parse_line(pattern: &regex::Regex, line: &str, file_name: &str) -> Option<LineDetails> {

    match pattern.captures(line) {
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
