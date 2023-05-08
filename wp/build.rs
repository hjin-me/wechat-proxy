// build.rs
use std::process::Command;

fn main() {
    let git_commit_short_id = Command::new("git")
        .arg("rev-parse")
        .arg("--short")
        .arg("HEAD")
        .output()
        .expect("failed to execute git rev-parse")
        .stdout;
    println!(
        "cargo:rustc-env=COMMIT_ID={}",
        String::from_utf8(git_commit_short_id).expect("git commit id is not utf8")
    );
}
