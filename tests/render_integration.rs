use std::io::Write;
use std::process::Command;

fn mdr_bin() -> String {
    // CARGO_BIN_EXE_mkdr is set automatically by cargo for integration tests
    env!("CARGO_BIN_EXE_mkdr").to_string()
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
    assert!(stdout.contains("mkdr"), "stdout: {stdout}");
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

#[test]
fn test_cli_detect_terminal() {
    let output = Command::new(mdr_bin())
        .arg("--detect-terminal")
        .output()
        .expect("failed to run mdr");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty(), "should print terminal name");
}

/// Verifies all new CLI options appear in --help output.
#[test]
fn test_cli_new_options_in_help() {
    let output = Command::new(mdr_bin())
        .arg("--help")
        .output()
        .expect("failed to run mdr");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--no-colour"), "help should mention --no-colour");
    assert!(stdout.contains("--columns"), "help should mention --columns");
    assert!(stdout.contains("--local"), "help should mention --local");
    assert!(stdout.contains("--fail"), "help should mention --fail");
    assert!(stdout.contains("--detect-terminal"), "help should mention --detect-terminal");
    assert!(stdout.contains("--ansi"), "help should mention --ansi");
}

/// --no-colour accepts stdin. Uses `#[ignore]` because it requires a TTY.
#[test]
#[ignore]
fn test_cli_no_colour_accepts_stdin() {
    let mut child = Command::new(mdr_bin())
        .arg("-c")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn mdr");

    {
        let stdin = child.stdin.as_mut().unwrap();
        stdin.write_all(b"# Hello\n\nWorld.\n").unwrap();
    }

    let result = child
        .wait_with_output()
        .expect("failed to wait");
    assert!(
        result.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
}
