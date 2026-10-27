//! `fa-server init`: creates the instance secrets on first start (D-074). Runs before any
//! configuration is loaded, so it needs no database URL.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::{OpenOptionsExt, chown};
use std::path::Path;

/// Owner of the secrets inside the app image (the `fa` user).
const APP_UID_GID: (u32, u32) = (10001, 10001);

fn os_random(buf: &mut [u8]) -> io::Result<()> {
    File::open("/dev/urandom")?.read_exact(buf)
}

fn write_new(
    dir: &Path,
    name: &str,
    mode: u32,
    owner: Option<(u32, u32)>,
    data: &[u8],
) -> io::Result<bool> {
    let path = dir.join(name);
    let mut file = match OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(mode)
        .open(&path)
    {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => return Ok(false),
        Err(e) => return Err(e),
    };
    file.write_all(data)?;
    file.sync_all()?;
    if let Some((uid, gid)) = owner {
        chown(&path, Some(uid), Some(gid))?;
    }
    Ok(true)
}

/// Creates `master_key` (32 random bytes, 0400) and `db_password` (48 lower-case hex
/// characters, 0444) in `dir` if they are absent. Existing files are never touched.
/// Returns the names of the files it created.
pub fn create_secrets(dir: &Path, owner: Option<(u32, u32)>) -> io::Result<Vec<&'static str>> {
    fs::create_dir_all(dir)?;
    let mut created = Vec::new();

    let mut key = [0u8; 32];
    os_random(&mut key)?;
    if write_new(dir, "master_key", 0o400, owner, &key)? {
        created.push("master_key");
    }

    let mut raw = [0u8; 24];
    os_random(&mut raw)?;
    let password: String = raw.iter().map(|b| format!("{b:02x}")).collect();
    if write_new(dir, "db_password", 0o444, owner, password.as_bytes())? {
        created.push("db_password");
    }
    Ok(created)
}

/// CLI entry: default directory is `FA_SECRETS_DIR` or `/run/fa-secrets`. Prints only names.
pub fn run(dir: Option<&Path>) -> anyhow::Result<()> {
    let env_dir = std::env::var_os("FA_SECRETS_DIR");
    let dir = match (dir, env_dir.as_deref()) {
        (Some(d), _) => d,
        (None, Some(d)) => Path::new(d),
        (None, None) => Path::new("/run/fa-secrets"),
    };
    let created = create_secrets(dir, Some(APP_UID_GID))?;
    if created.is_empty() {
        println!("secrets already present, nothing created");
    } else {
        for name in created {
            println!("created {name}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::MetadataExt;

    use super::*;

    fn is_root() -> bool {
        fs::metadata("/proc/self")
            .map(|m| m.uid() == 0)
            .unwrap_or(false)
    }

    #[test]
    fn creates_both_files_with_modes() {
        let dir = tempfile::tempdir().unwrap();
        let created = create_secrets(dir.path(), None).unwrap();
        assert_eq!(created, vec!["master_key", "db_password"]);

        let key = dir.path().join("master_key");
        assert_eq!(fs::read(&key).unwrap().len(), 32);
        assert_eq!(fs::metadata(&key).unwrap().mode() & 0o777, 0o400);

        let pw = dir.path().join("db_password");
        let text = fs::read_to_string(&pw).unwrap();
        assert_eq!(text.len(), 48);
        assert!(
            text.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        );
        assert_eq!(fs::metadata(&pw).unwrap().mode() & 0o777, 0o444);
    }

    #[test]
    fn never_overwrites() {
        let dir = tempfile::tempdir().unwrap();
        create_secrets(dir.path(), None).unwrap();
        let before = fs::read(dir.path().join("master_key")).unwrap();
        let pw_before = fs::read(dir.path().join("db_password")).unwrap();
        assert!(create_secrets(dir.path(), None).unwrap().is_empty());
        assert_eq!(before, fs::read(dir.path().join("master_key")).unwrap());
        assert_eq!(pw_before, fs::read(dir.path().join("db_password")).unwrap());
    }

    #[test]
    fn creates_only_the_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("master_key"), b"x").unwrap();
        assert_eq!(
            create_secrets(dir.path(), None).unwrap(),
            vec!["db_password"]
        );
        assert_eq!(fs::read(dir.path().join("master_key")).unwrap(), b"x");
    }

    #[test]
    fn chowns_when_root() {
        if !is_root() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        create_secrets(dir.path(), Some((10001, 10001))).unwrap();
        let meta = fs::metadata(dir.path().join("master_key")).unwrap();
        assert_eq!((meta.uid(), meta.gid()), (10001, 10001));
    }
}
