use serde::Serialize;
use std::path::{Component, Path};
use std::process::{Command, Output};

const MAX_DIFF: usize = 512 * 1024;
#[derive(Serialize)]
pub struct GitFile {
    pub path: String,
    pub status: String,
}
#[derive(Serialize)]
pub struct GitStatus {
    pub branch: String,
    pub root: String,
    pub files: Vec<GitFile>,
}

fn git(cwd: &str, args: &[&str]) -> Result<Output, String> {
    Command::new("git")
        .args(["--no-pager", "-c", "core.fsmonitor=false"])
        .args(args)
        .current_dir(cwd)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_LITERAL_PATHSPECS", "1")
        .output()
        .map_err(|e| e.to_string())
}
fn output(cwd: &str, args: &[&str]) -> Result<String, String> {
    let out = git(cwd, args)?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    String::from_utf8(out.stdout).map_err(|_| "Git returned a non-UTF-8 path or diff".into())
}
fn parse_status(text: &str) -> Vec<GitFile> {
    let mut records = text.split('\0');
    let mut files = Vec::new();
    while let Some(record) = records.next() {
        if record.len() < 4 || !record.is_char_boundary(3) {
            continue;
        }
        let status = &record[..2];
        files.push(GitFile {
            status: status.into(),
            path: record[3..].into(),
        });
        // porcelain -z rename records contain destination then original path.
        if status.contains('R') || status.contains('C') {
            records.next();
        }
    }
    files
}
pub fn status(cwd: &str) -> Result<GitStatus, String> {
    let root = output(cwd, &["rev-parse", "--show-toplevel"])?
        .trim()
        .to_string();
    let branch = output(&root, &["symbolic-ref", "--quiet", "--short", "HEAD"])
        .or_else(|_| output(&root, &["rev-parse", "--short", "HEAD"]))?
        .trim()
        .to_string();
    let text = output(
        &root,
        &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
    )?;
    Ok(GitStatus {
        root,
        branch,
        files: parse_status(&text),
    })
}
fn validate_path(path: &str) -> Result<(), String> {
    if path.is_empty()
        || !Path::new(path)
            .components()
            .all(|p| matches!(p, Component::Normal(_)))
    {
        return Err("Diff path must be a relative repository file".into());
    }
    Ok(())
}
pub fn diff(cwd: &str, path: &str, staged: bool) -> Result<String, String> {
    validate_path(path)?;
    let repo = status(cwd)?;
    let file = repo
        .files
        .iter()
        .find(|f| f.path == path)
        .ok_or("File is no longer changed. Refresh changes.")?;
    if file.status == "??" {
        if staged {
            return Ok(String::new());
        }
        let full = Path::new(&repo.root).join(path);
        let meta = std::fs::symlink_metadata(&full).map_err(|e| e.to_string())?;
        if !meta.is_file() || meta.len() > MAX_DIFF as u64 {
            return Err(
                "Untracked preview unavailable: not a regular file or larger than 512 KiB".into(),
            );
        }
        let canonical = full.canonicalize().map_err(|e| e.to_string())?;
        if !canonical.starts_with(
            Path::new(&repo.root)
                .canonicalize()
                .map_err(|e| e.to_string())?,
        ) {
            return Err("File is outside the repository".into());
        }
        let text = std::fs::read_to_string(canonical).map_err(|_| "Binary file — no text diff")?;
        if text.contains('\0') {
            return Err("Binary file — no text diff".into());
        }
        let lines: Vec<_> = text.lines().collect();
        return Ok(format!(
            "--- /dev/null\n+++ b/{path}\n@@ -0,0 +1,{} @@\n{}",
            lines.len(),
            lines.iter().map(|l| format!("+{l}\n")).collect::<String>()
        ));
    }
    let mut args = vec![
        "diff",
        "--no-ext-diff",
        "--no-textconv",
        "--no-color",
        "--unified=3",
    ];
    if staged {
        args.push("--cached");
    }
    args.extend(["--", path]);
    let result = output(&repo.root, &args)?;
    if result.len() > MAX_DIFF {
        return Err("Diff exceeds 512 KiB; inspect this file in your editor".into());
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_untracked_status_and_diff_in_isolated_repository() {
        let root = std::env::temp_dir().join(format!(
            "pi-console-git-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let cwd = root.to_str().unwrap();
        assert!(git(cwd, &["init", "--quiet"]).unwrap().status.success());
        std::fs::write(root.join("a b.txt"), "hello\nworld\n").unwrap();
        let repo = status(cwd).unwrap();
        assert_eq!(repo.files.len(), 1);
        assert_eq!(repo.files[0].status, "??");
        assert!(diff(cwd, "a b.txt", false)
            .unwrap()
            .contains("@@ -0,0 +1,2 @@\n+hello\n+world"));
        assert_eq!(diff(cwd, "a b.txt", true).unwrap(), "");
        assert!(diff(cwd, "../outside", false).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn porcelain_renames_spaces_and_newlines() {
        let files = parse_status(" M src/a b.ts\0R  new\nname\0old\0?? untracked\0");
        assert_eq!(files.len(), 3);
        assert_eq!(files[1].path, "new\nname");
        assert_eq!(files[2].status, "??");
    }
    #[test]
    fn rejects_path_traversal_and_absolute_paths() {
        for p in ["../secret", "/etc/passwd", "src/../../x", ""] {
            assert!(validate_path(p).is_err());
        }
        assert!(validate_path("src/a b.ts").is_ok());
    }
}
