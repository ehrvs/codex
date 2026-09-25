use super::*;
use clap::Parser;
use pretty_assertions::assert_eq;
use std::collections::BTreeSet;

/// Top-level subcommands that never receive defaults (they reject a root
/// `--profile`, or, like `debug`, only some of their children accept one).
const NON_RUNTIME: &[&str] = &[
    "tcp-tunnel",
    "login",
    "logout",
    "plugin",
    "app-server",
    "remote-control",
    "app",
    "completion",
    "update",
    "doctor",
    "debug",
    "execpolicy",
    "apply",
    "migrate-rollouts",
    "cloud",
    "responses-api-proxy",
    "stdio-to-uds",
    "exec-server",
    "features",
    "help",
];

fn os(args: &[&str]) -> Vec<OsString> {
    args.iter().map(OsString::from).collect()
}

struct Run {
    out: Vec<OsString>,
    warnings: Vec<String>,
}

fn run_with(file: Option<&str>, args: &[&str], no_defaults: bool) -> Run {
    let home = tempfile::tempdir().expect("tempdir");
    if let Some(contents) = file {
        std::fs::write(home.path().join(LOCAL_DEFAULTS_FILE), contents).expect("write");
    }
    let mut warnings = Vec::new();
    let out = apply_local_defaults_with(os(args), Some(home.path()), no_defaults, |message| {
        warnings.push(message.to_string())
    });
    Run { out, warnings }
}

fn run(file: &str, args: &[&str]) -> Run {
    run_with(Some(file), args, /*no_defaults*/ false)
}

fn assert_applied(file: &str, args: &[&str], expected: &[&str]) {
    let result = run(file, args);
    assert_eq!(result.out, os(expected));
    assert_eq!(result.warnings, Vec::<String>::new());
    MultitoolCli::try_parse_from(&result.out).expect("result should parse");
}

fn assert_unchanged(file: &str, args: &[&str]) {
    let result = run(file, args);
    assert_eq!(result.out, os(args));
    assert_eq!(result.warnings, Vec::<String>::new());
}

fn assert_malformed(file: &str) {
    let args = ["codex", "exec", "hi"];
    let result = run(file, &args);
    assert_eq!(result.out, os(&args), "file: {file}");
    assert_eq!(result.warnings.len(), 1, "file: {file}");
    assert!(
        result.warnings[0].contains(LOCAL_DEFAULTS_FILE),
        "{:?}",
        result.warnings
    );
}

#[test]
fn no_file_passes_through() {
    let args = ["codex", "exec", "hi"];
    let result = run_with(None, &args, /*no_defaults*/ false);
    assert_eq!(result.out, os(&args));
    assert_eq!(result.warnings, Vec::<String>::new());
}

#[test]
fn no_codex_home_passes_through() {
    let args = os(&["codex", "exec", "hi"]);
    let out = apply_local_defaults_with(args.clone(), None, false, |_| panic!("no warning"));
    assert_eq!(out, args);
}

#[test]
fn no_defaults_env_passes_through() {
    let args = ["codex", "exec", "hi"];
    let result = run_with(
        Some(r#"default_args = ["-m", "qwen"]"#),
        &args,
        /*no_defaults*/ true,
    );
    assert_eq!(result.out, os(&args));
    assert_eq!(result.warnings, Vec::<String>::new());
}

#[test]
fn inserts_defaults_after_argv0_in_file_order() {
    let file = r#"default_args = ["-c", "model_provider=ollama", "--model=qwen3.8:27B", "-c=a=1"]"#;
    assert_applied(
        file,
        &["codex", "exec", "--skip-git-repo-check", "say ok"],
        &[
            "codex",
            "--config=model_provider=ollama",
            "--model=qwen3.8:27B",
            "--config=a=1",
            "exec",
            "--skip-git-repo-check",
            "say ok",
        ],
    );
    assert_applied(
        file,
        &["codex"],
        &[
            "codex",
            "--config=model_provider=ollama",
            "--model=qwen3.8:27B",
            "--config=a=1",
        ],
    );
}

#[test]
fn empty_default_args_passes_through() {
    assert_unchanged("default_args = []", &["codex", "exec", "hi"]);
}

#[test]
fn unknown_keys_are_ignored() {
    assert_applied(
        "note = \"managed\"\ndefault_args = [\"-m\", \"qwen\"]\n[extra]\nx = 1\n",
        &["codex", "exec", "hi"],
        &["codex", "--model=qwen", "exec", "hi"],
    );
}

#[test]
fn user_model_drops_default_model_in_every_form() {
    let file = r#"default_args = ["-c", "k=v", "--model", "qwen"]"#;
    for user in [
        vec!["codex", "-m", "other"],
        vec!["codex", "--model", "other"],
        vec!["codex", "--model=other"],
        vec!["codex", "-m=other"],
        vec!["codex", "exec", "-m", "other", "hi"],
        vec!["codex", "exec", "--model=other", "hi"],
    ] {
        let mut expected = vec!["codex", "--config=k=v"];
        expected.extend(&user[1..]);
        assert_applied(file, &user, &expected);
    }
}

#[test]
fn user_profile_drops_default_profile_in_every_form() {
    let file = r#"default_args = ["-c", "k=v", "-p", "a"]"#;
    for user in [
        vec!["codex", "-p", "b"],
        vec!["codex", "--profile", "b"],
        vec!["codex", "--profile=b"],
        vec!["codex", "exec", "-p", "b", "hi"],
        vec!["codex", "exec", "--profile=b", "hi"],
    ] {
        let mut expected = vec!["codex", "--config=k=v"];
        expected.extend(&user[1..]);
        assert_applied(file, &user, &expected);
    }
}

#[test]
fn user_profile_drops_default_profile_and_model() {
    assert_applied(
        r#"default_args = ["-p", "A", "-m", "mA"]"#,
        &["codex", "exec", "-p", "B", "hi"],
        &["codex", "exec", "-p", "B", "hi"],
    );
    assert_applied(
        r#"default_args = ["-p", "A", "-m", "mA"]"#,
        &["codex", "-p", "B"],
        &["codex", "-p", "B"],
    );
}

#[test]
fn user_config_keeps_default_config() {
    assert_applied(
        r#"default_args = ["-c", "model_provider=ollama"]"#,
        &["codex", "exec", "-c", "model_provider=other", "hi"],
        &[
            "codex",
            "--config=model_provider=ollama",
            "exec",
            "-c",
            "model_provider=other",
            "hi",
        ],
    );
}

#[test]
fn non_runtime_subcommands_pass_through_with_profile_default() {
    let file = r#"default_args = ["-p", "x", "-c", "k=v", "-m", "qwen"]"#;
    assert_unchanged(file, &["codex", "login", "status"]);
    assert_unchanged(file, &["codex", "features", "list"]);
    assert_unchanged(file, &["codex", "debug", "models"]);
    assert_unchanged(file, &["codex", "plugin", "list"]);
}

#[test]
fn clap_error_passes_through() {
    let file = r#"default_args = ["-m", "qwen"]"#;
    assert_unchanged(file, &["codex", "--definitely-not-a-flag"]);
    assert_unchanged(file, &["codex", "--help"]);
    assert_unchanged(file, &["codex", "--version"]);
}

#[test]
fn runtime_subcommands_receive_defaults() {
    let file = r#"default_args = ["-p", "x"]"#;
    assert_applied(
        file,
        &["codex", "resume", "--last"],
        &["codex", "--profile=x", "resume", "--last"],
    );
    assert_applied(
        file,
        &["codex", "mcp", "list"],
        &["codex", "--profile=x", "mcp", "list"],
    );
}

#[test]
fn malformed_invalid_toml() {
    assert_malformed("default_args = [");
}

#[test]
fn malformed_missing_default_args() {
    assert_malformed("other = 1");
}

#[test]
fn malformed_default_args_not_array() {
    assert_malformed(r#"default_args = "-m qwen""#);
}

#[test]
fn malformed_default_args_not_strings() {
    assert_malformed(r#"default_args = ["-m", 3]"#);
}

#[test]
fn malformed_disallowed_flag() {
    assert_malformed(r#"default_args = ["--sandbox", "danger-full-access"]"#);
    assert_malformed(r#"default_args = ["-s=read-only"]"#);
    assert_malformed(r#"default_args = ["qwen"]"#);
    assert_malformed(r#"default_args = ["-mqwen"]"#);
}

#[test]
fn malformed_dangling_flags() {
    for flag in ["-c", "--config", "-m", "--model", "-p", "--profile"] {
        assert_malformed(&format!(r#"default_args = ["{flag}"]"#));
        assert_malformed(&format!(r#"default_args = ["{flag}", "-m", "qwen"]"#));
    }
}

#[test]
fn malformed_empty_values() {
    assert_malformed(r#"default_args = ["-m", ""]"#);
    assert_malformed(r#"default_args = ["--model="]"#);
    assert_malformed(r#"default_args = ["-c="]"#);
    assert_malformed(r#"default_args = ["--profile", ""]"#);
}

#[test]
fn runtime_subcommands_parity() {
    let runtime: BTreeSet<&str> = RUNTIME_SUBCOMMANDS.iter().copied().collect();
    let non_runtime: BTreeSet<&str> = NON_RUNTIME.iter().copied().collect();
    assert_eq!(
        runtime.intersection(&non_runtime).collect::<Vec<_>>(),
        Vec::<&&str>::new(),
        "a subcommand cannot be both runtime and non-runtime"
    );

    let command = MultitoolCli::command();
    let names: BTreeSet<String> = command
        .get_subcommands()
        .map(|sub| sub.get_name().to_string())
        .collect();
    let unclassified: Vec<&String> = names
        .iter()
        .filter(|name| !runtime.contains(name.as_str()) && !non_runtime.contains(name.as_str()))
        .collect();
    assert_eq!(
        unclassified,
        Vec::<&String>::new(),
        "classify new subcommands in RUNTIME_SUBCOMMANDS (if profile_v2_for_subcommand accepts them) or NON_RUNTIME"
    );
    let missing: Vec<&&str> = RUNTIME_SUBCOMMANDS
        .iter()
        .filter(|name| !names.contains(**name))
        .collect();
    assert_eq!(
        missing,
        Vec::<&&str>::new(),
        "stale RUNTIME_SUBCOMMANDS entry"
    );
}
