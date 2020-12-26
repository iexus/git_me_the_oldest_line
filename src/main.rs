extern crate clap;
use clap::{Arg, App};

#[macro_use]
extern crate lazy_static;

mod process_files;
mod line_details;
use crate::process_files::process_files;

fn main() {
    let matches = App::new("Git me the oldest line")
        .version("0.1.0")
        .author("iexus <2.-@twopointline.com>")
        .about("Finds you the oldest line in a git repo")
        .arg(Arg::with_name("threads")
            .short("t")
            .long("threads")
            .value_name("number_of_workers")
            .help("Sets the number of threads to use.")
            .takes_value(true)
        )
        .arg(Arg::with_name("directory")
            .short("d")
            .long("directory")
            .value_name("directory")
            .help("Set the directory to blame within")
            .takes_value(true)
        )
        .arg(Arg::with_name("ignore")
            .short("i")
            .long("ignore")
            .help("Set any files you wish to ignore.")
            .min_values(1)
            .takes_value(true)
        )
        .get_matches();

    let workers = match matches.value_of("threads") {
        None => 2,
        Some(value) => {
            value.parse::<usize>().unwrap()
        }
    };
    println!("Running with {} workers", workers);

    let directory = match matches.value_of("directory") {
        None => "./",
        Some(value) => value,
    };
    println!("Looking in directory: {}", directory);

    let ignore_list: Vec<String> = match matches.values_of("ignore") {
        None => Vec::<String>::new(),
        Some(values) => {
            values.map(|x| x.to_string()).collect()
        }
    };

    match process_files(workers, directory, ignore_list) {
        Ok(()) => println!("Success"),
        Err(error) => panic!("Problem occurred: {}", error),
    };
}
