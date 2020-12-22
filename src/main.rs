use std::process::Command;
use std::io::{self, Write};

fn main() {
    let output = Command::new("git")
        .args(&["ls-tree", "-r", "--name-only", "HEAD"])
        .output()
        .expect("failed to execute git ls-tree - is git installed?");

    println!("{}", output.status);

    io::stdout().write_all(&output.stdout).unwrap();
}
