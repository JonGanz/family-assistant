//! Stamps the binary with the git commit and build time for Settings, System, About.
//! `FA_GIT_COMMIT` (a Docker build arg, since `.git` is not in the image) wins over `git`.

use std::process::Command;

fn run(cmd: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(cmd).args(args).output().ok()?;
    let s = String::from_utf8(out.stdout).ok()?.trim().to_owned();
    (out.status.success() && !s.is_empty()).then_some(s)
}

fn main() {
    println!("cargo:rerun-if-env-changed=FA_GIT_COMMIT");
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    let commit = std::env::var("FA_GIT_COMMIT")
        .ok()
        .filter(|c| !c.trim().is_empty())
        .or_else(|| {
            let hash = run("git", &["rev-parse", "--short=12", "HEAD"])?;
            let dirty = run("git", &["status", "--porcelain"]).is_some();
            Some(if dirty { format!("{hash}-dirty") } else { hash })
        });
    if let Some(c) = commit {
        println!("cargo:rustc-env=FA_GIT_COMMIT={c}");
    }
    if let Some(t) = run("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"]) {
        println!("cargo:rustc-env=FA_BUILD_TIME={t}");
    }
}
