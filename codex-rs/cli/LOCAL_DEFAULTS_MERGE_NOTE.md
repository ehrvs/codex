## 2026-09-25: codex-local default flags (fold into chat-completions/MERGE_NOTES.md)

Fork-only hunks in `codex-rs/cli` to keep on every upstream merge:

- `cli/src/local_defaults.rs` and `cli/src/local_defaults_tests.rs` (new files). They read
  `default_args` from `<codex_home>/codex-local.toml` and insert allowed flags (`-c`, `-m`, `-p`)
  after argv[0] for runtime invocations only. `CODEX_LOCAL_NO_DEFAULTS=1` disables them.
- `cli/src/main.rs`: `mod local_defaults;` plus the one-line wiring in `cli_main`:
  `MultitoolCli::parse_from(local_defaults::apply_local_defaults(std::env::args_os().collect()))`
  instead of `MultitoolCli::parse()`.
- `cli/Cargo.toml`: `codex-utils-home-dir` dependency (and the matching `Cargo.lock` entry).

On merge: if upstream changes the runtime match arm in `profile_v2_for_subcommand`, update
`RUNTIME_SUBCOMMANDS`. If it adds a subcommand, the `runtime_subcommands_parity` test fails until
the subcommand is added to `RUNTIME_SUBCOMMANDS` or the test's `NON_RUNTIME` list. The
`-m`/`-p` dedupe looks up the clap arg ids `model` and `config_profile_v2` from
`SharedCliOptions`, so a rename there needs the constants updated.
