use std::{path::PathBuf, process::Command};

#[test]
fn renamed_zbus_without_direct_serde_compiles() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join("tests/downstream-renamed/Cargo.toml");
    let target = manifest_dir.join("../target/downstream-renamed");
    let output = Command::new(env!("CARGO"))
        .args(["check", "--offline", "--manifest-path"])
        .arg(fixture)
        .env("CARGO_TARGET_DIR", target)
        .output()
        .expect("failed to run cargo check for downstream fixture");

    assert!(
        output.status.success(),
        "downstream fixture failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
