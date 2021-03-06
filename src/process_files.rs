use std::io::{BufRead, BufReader, Error, ErrorKind};
use std::process::{Command, Stdio};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;

use crate::line_details::{LineDetails, parse_line};

const QUIT_MESSAGE: &'static str = "QUIT_TASK";

pub fn process_files(num_workers: usize, directory: &str, ignore_list: Vec<String>) -> Result<(), Error> {
    // We want a number of workers to handle the filenames
    let mut workers = Vec::new();

    // One channel to send lines down - another to send parsed results down
    let (line_sender, line_receiver) = channel::<(String, String)>();
    let (result_sender, result_receiver) = channel::<LineDetails>();

    // We maintain a channel for each worker which we send it file names down.
    let mut worker_channels = Vec::new();

    for i in 0..num_workers {
        let (sender, receiver) = channel::<String>();
        worker_channels.push(sender);

        // clone the channel we'll send lines to the worker on
        let line_sender_clone = line_sender.clone();
        let worker = thread::spawn(move || {
            blame_files(i, receiver, line_sender_clone);
        });

        // Save the worker so we can join to it later
        workers.push(worker);
    }

    // Fire off all the messages to these worker channels
    generate_file_messages(worker_channels, directory, ignore_list)?;

    // Start a thread to parse all the returned lines
    let details_parser = thread::spawn(move || {
        find_oldest_line(line_receiver, result_sender);
    });

    // Join on all the threads - wait for them to be done blaming.
    for worker in workers {
        worker.join().unwrap();
    }

    // Close the original result sender - this will tell our thread processing into LineDetails
    // that it is done and it can finish
    line_sender.send((QUIT_MESSAGE.to_string(), QUIT_MESSAGE.to_string())).unwrap();
    details_parser.join().unwrap();

    // This is a list of potential results the last one will be the earliest.
    let result = result_receiver.iter().last().unwrap();
    println!("Oldest record found.");
    println!("{}", result);

    Ok(())
}

fn blame_files(thread_id: usize, receiver: Receiver<String>, transmitter: Sender<(String, String)>) {
    for message in receiver {
        if message == QUIT_MESSAGE.to_string() {
            println!("Thread {} quitting.", thread_id);
            break;
        }

        println!("Thread {}, blaming file: {}", thread_id, message);
        match message {
            message => {
                // -l is for the long commit reference
                // -f to always show the file name of where the code came from (movement tracking)
                // -M and -C are related to tracking down code movements to the original commit
                // rather than just the latest that touched them
                let git_blame_output = Command::new("git")
                    .args(&["blame", "-l", "-f", "-M", "-C", &message])
                    .output().unwrap();

                git_blame_output.stdout
                    .lines()
                    .filter_map(|line| line.ok())
                    .for_each(|line| {
                        transmitter.send((line, message.clone())).unwrap();
                    });
            }
        };
    }

    // we need to tell the main thread which is parsing these messages
    // that we are done - so drop our copy of the transmitter.
    drop(transmitter);
}

fn generate_file_messages(worker_channels: Vec::<Sender<String>>, directory: &str, ignore_list: Vec<String>) -> Result<(), Error> {
    let num_workers = worker_channels.len();

    // Get a list of every file that git tracks
    let gitls_stdout = Command::new("git")
        .args(&["ls-tree", "-r", "--name-only", "HEAD", directory])
        .stdout(Stdio::piped())
        .spawn()?
        .stdout
        .ok_or_else(|| Error::new(ErrorKind::Other,"Could not capture standard output."))?;

    // For each one run a git blame on it.
    let reader = BufReader::new(gitls_stdout);
    let mut round_robin = 0;
    let mut count = 0;
    reader
        .lines()
        .filter_map(|line| line.ok())
        .filter(|line| {
            !ignore_list.contains(line)
        })
        .for_each(|file_name| {
            // get the current worker
            let message_sender = worker_channels.get(round_robin).unwrap();
            message_sender.send(file_name).unwrap();

            count += 1;
            round_robin = (round_robin + 1) % num_workers;
        });

    println!("Found {} files to blame.", count);

    // We send an end message down the queues so that the thread knows to quit
    for sender in worker_channels {
        sender.send(QUIT_MESSAGE.to_string()).unwrap();
    }

    Ok(())
}

fn find_oldest_line(line_receiver: Receiver<(String, String)>, result_sender: Sender<LineDetails>) {
    let mut oldest_line_so_far = LineDetails::default();

    for message in line_receiver {
        if message.0 == QUIT_MESSAGE.to_string() {
            println!("Line parser thread quitting.");
            break;
        }

        match parse_line(&message.0, &message.1) {
            Some(details) => {
                if details.datetime < oldest_line_so_far.datetime {
                    oldest_line_so_far = details;
                    result_sender.send(oldest_line_so_far.clone()).unwrap();
                }
            },
            None => panic!("Could not create details from line: {}, in file: {}", message.0, message.1),
        }
    };
}
