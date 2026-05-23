use std::process::Command;

fn mdr_bin() -> String {
    // CARGO_BIN_EXE_mdr is set automatically by cargo for integration tests
    env!("CARGO_BIN_EXE_mdr").to_string()
}

#[test]
fn test_cli_file_not_found() {
    let output = Command::new(mdr_bin())
        .arg("nonexistent.md")
        .output()
        .expect("failed to run mdr");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found"), "stderr: {stderr}");
}

#[test]
fn test_cli_stdin_pipe() {
    let mut child = Command::new(mdr_bin())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn mdr");

    {
        let stdin = child.stdin.as_mut().unwrap();
        use std::io::Write;
        stdin.write_all(b"# Hello\n\nWorld.\n").unwrap();
    }

    let _ = child.wait_with_output();
}

#[test]
fn test_cli_version() {
    let output = Command::new(mdr_bin())
        .arg("--version")
        .output()
        .expect("failed to run mdr");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("mdr"), "stdout: {stdout}");
}

#[test]
fn test_cli_help() {
    let output = Command::new(mdr_bin())
        .arg("--help")
        .output()
        .expect("failed to run mdr");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("TUI markdown"), "stdout: {stdout}");
}
