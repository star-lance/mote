//! Tests that run the mote and glyphgen binaries.

use std::process::Command;

#[test]
fn glyphgen_reproduces_the_bundled_atlas() {
    let root = env!("CARGO_MANIFEST_DIR");
    let out = std::env::temp_dir().join(format!("mote-glyphgen-test-{}.json", std::process::id()));
    let status = Command::new(env!("CARGO_BIN_EXE_glyphgen"))
        .args([
            "--font",
            &format!("{root}/assets/fonts/IoskeleyMonoTermNerdFontMono-Regular.ttf"),
        ])
        .args(["--size-px", "16", "--out"])
        .arg(&out)
        .output()
        .unwrap()
        .status;
    assert!(status.success());
    let generated = std::fs::read_to_string(&out).unwrap();
    std::fs::remove_file(&out).unwrap();
    let bundled = std::fs::read_to_string(format!("{root}/assets/atlas/default.json")).unwrap();
    assert!(
        generated == bundled,
        "glyphgen output differs from assets/atlas/default.json"
    );
}

#[test]
fn mote_reports_a_missing_config_file() {
    let out = Command::new(env!("CARGO_BIN_EXE_mote"))
        .args(["--config", "/nonexistent/mote.toml"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("/nonexistent/mote.toml"));
}
