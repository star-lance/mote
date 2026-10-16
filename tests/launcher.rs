//! Runs scripts/mote-launch against a stub `kitty` to check where and how it
//! writes its fontconfig file.

use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("mote-launch-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn uid() -> String {
    String::from_utf8(Command::new("id").arg("-u").output().unwrap().stdout)
        .unwrap()
        .trim()
        .to_string()
}

/// Installs a stub kitty that records FONTCONFIG_FILE and returns (bin dir, record file).
fn stub_kitty(dir: &Path) -> (PathBuf, PathBuf) {
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let record = dir.join("record");
    let kitty = bin.join("kitty");
    std::fs::write(
        &kitty,
        format!(
            "#!/bin/sh\nprintf '%s' \"$FONTCONFIG_FILE\" > '{}'\n",
            record.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&kitty, std::fs::Permissions::from_mode(0o755)).unwrap();
    (bin, record)
}

fn launch(script: &Path, bin: &Path, tmpdir: &Path) -> std::process::Output {
    let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap());
    Command::new("sh")
        .arg(script)
        .env_remove("XDG_RUNTIME_DIR")
        .env("TMPDIR", tmpdir)
        .env("PATH", path)
        .env("MOTE_BIN", "/bin/true")
        .output()
        .unwrap()
}

fn script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/mote-launch")
}

#[test]
fn uses_private_per_user_dir_without_xdg_runtime_dir() {
    let dir = scratch("private");
    let (bin, record) = stub_kitty(&dir);
    let out = launch(&script(), &bin, &dir);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let conf = PathBuf::from(std::fs::read_to_string(&record).unwrap());
    let conf_dir = dir.join(format!("mote-{}", uid()));
    assert_eq!(conf.parent().unwrap(), conf_dir);
    let mode = std::fs::metadata(&conf_dir).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o700);
}

#[test]
fn refuses_symlinked_runtime_dir() {
    let dir = scratch("symlink");
    let (bin, record) = stub_kitty(&dir);
    let elsewhere = dir.join("elsewhere");
    std::fs::create_dir_all(&elsewhere).unwrap();
    symlink(&elsewhere, dir.join(format!("mote-{}", uid()))).unwrap();
    let out = launch(&script(), &bin, &dir);
    assert!(!out.status.success());
    assert!(!record.exists(), "kitty must not be started");
    assert!(
        std::fs::read_dir(&elsewhere).unwrap().next().is_none(),
        "nothing written through the symlink"
    );
}

#[test]
fn escapes_xml_special_chars_in_repo_path() {
    let dir = scratch("xml");
    let (bin, record) = stub_kitty(&dir);
    let repo = dir.join("a&b<c");
    std::fs::create_dir_all(repo.join("scripts")).unwrap();
    std::fs::copy(script(), repo.join("scripts/mote-launch")).unwrap();
    let out = launch(&repo.join("scripts/mote-launch"), &bin, &dir);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let conf = std::fs::read_to_string(std::fs::read_to_string(&record).unwrap()).unwrap();
    assert!(conf.contains("a&amp;b&lt;c/assets/fonts"), "{conf}");
}
