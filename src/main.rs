use chrono::prelude::*;
use regex::Regex;
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader, Error, ErrorKind};

use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;

mod line_details;
use crate::line_details::LineDetails;

const QUIT_MESSAGE: &'static str = "QUIT_TASK";

fn main() {
    match gather_files() {
        Ok(()) => println!("Success"),
        Err(error) => panic!("Problem occurred: {}", error),
    };
}

fn gather_files() -> Result<(), Error> {
    let num_workers = 2;

    // We want a number of workers to handle the filenames
    let mut workers = Vec::new();

    // Create a channel to get messages back
    let (result_sender, result_receiver) = channel::<LineDetails>();

    // Create a number of channels to send tasks to workers
    let mut send_lines_here = Vec::new();

    for i in 0..num_workers {
        println!("Creating Thread: {}", i);

        // create the channels for sending shit
        let (sender, receiver) = channel::<String>();
        send_lines_here.push(sender);

        // Spawn threads and shove in the workers for us to join to later
        let result_sender_clone = result_sender.clone();
        let worker = thread::spawn(move || {
            handle_work(i, receiver, result_sender_clone);
        });

        // Save the worker
        workers.push(worker);
    }

    // Get a list of every file that git tracks
    let gitls_stdout = Command::new("git")
        .args(&["ls-tree", "-r", "--name-only", "HEAD"])
        .stdout(Stdio::piped())
        .spawn()?
        .stdout
        .ok_or_else(|| Error::new(ErrorKind::Other,"Could not capture standard output."))?;

    // For each one run a git blame on it.
    let reader = BufReader::new(gitls_stdout);
    let mut round_robin = 0;
    reader
        .lines()
        .filter_map(|line| line.ok())
        .for_each(|file_name| {
            // get the current worker
            let message_sender = send_lines_here.get(round_robin).unwrap();
            message_sender.send(file_name).unwrap();

            round_robin = (round_robin + 1) % num_workers;
        });

    // We send an end message down the queues so that the thread knows to quit
    for sender in send_lines_here {
        sender.send(QUIT_MESSAGE.to_string()).unwrap();
    }

    // Join on all the threads
    for worker in workers {
        worker.join().unwrap();
    }

    println!("Joined to all threads, all input parsed");
    // Close the original result sender
    drop(result_sender);

    // Now we can finally parse all the details
    let mut oldest_line_so_far = LineDetails::default();
    for message in result_receiver {
        println!("Receiving message in main thread");
        if message.datetime < oldest_line_so_far.datetime {
            oldest_line_so_far = message;
        }
    };

    println!("{}", oldest_line_so_far);
    Ok(())
}

fn handle_work(thread_id: usize, receiver: Receiver<String>, transmitter: Sender<LineDetails>) {
    // AVERT YOUR EYES CHILDREN
    let line_regex = Regex::new(
        r"(?P<commit_hash>(^.{40})) (?P<original_filename>(.+)) \((?P<author>(.+))(?P<datetime>(\d{4}-\d{2}-\d{2}[T\s]?\d{2}:\d{2}:\d{2}\s?[+-]\d{2}:?\d{2})).+\d{1}\)(?P<code>(.+$))"
    ).unwrap();

    for message in receiver {
        if message == QUIT_MESSAGE.to_string() {
            println!("Thread {} quitting.", thread_id);
            break;
        }

        println!("Thread {}, blaming file: {}", thread_id, message);
        match message {
            message => {
                match blame_file(message.clone(), &line_regex) {
                    Ok(details) => {
                        println!("Thread {} sending message.", thread_id);
                        transmitter.send(details).unwrap();
                    },
                    Err(error) => {
                        println!(
                            "Encountered error getting details for line: {}, with error: {}",
                            message, error
                        )
                    }
                }
            }
        };
    }

    drop(transmitter);
}

fn blame_file(file_name: String, line_regex: &regex::Regex) -> Result<LineDetails, Error> {
    // -l is for the long commit reference
    // -f to always show the file name of where the code came from (movement tracking)
    // -M and -C are related to tracking down code movements to the original commit
    // rather than just the latest that touched them
    let git_blame_output = Command::new("git")
        .args(&["blame", "-l", "-f", "-M", "-C", &file_name])
        .output()?;

    let mut oldest_line_so_far = LineDetails::default();
    git_blame_output.stdout
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
