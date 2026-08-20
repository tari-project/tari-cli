// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

//! `tari lint` — checks a template crate for issues that hurt the published WASM binary or the
//! developer experience: Rust lints (via `cargo clippy`), missing size-optimizing Cargo profile
//! settings, slow test runtimes, missing package metadata and a bloated `crate-type`.
//!
//! Every check other than the Rust lints carries a concrete, copy-pasteable fix, and most can be
//! applied automatically with `--fix`.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::{Context, anyhow};
use clap::Parser;
use tokio::fs;
use tokio::process::Command;
use toml_edit::{Array, DocumentMut, Item, Table, Value, value};

use crate::cli::commands::template::init_metadata;

const WASM_TARGET: &str = "wasm32-unknown-unknown";
const TARI_TEMPLATE_METADATA_KEY: &str = "tari-template";
const BUILD_DEP_KEY: &str = "tari_ootle_template_build";

/// The size-optimizing release profile every template should declare. `tari build` injects these
/// via `cargo build --config`, but a plain `cargo build --release` (or any other tool building the
/// crate) will not, so the manifest should carry them too.
const RELEASE_PROFILE_SNIPPET: &str = r#"[profile.release]
opt-level = 's'     # Optimize for size.
lto = true          # Enable Link Time Optimization.
codegen-units = 1   # Reduce number of codegen units to increase optimizations.
panic = 'abort'     # Abort on panic.
strip = true        # Strip symbols from the binary."#;

/// `[profile.release]` keys checked (and written by `--fix`), with the comment `--fix` writes.
const RELEASE_PROFILE_KEYS: &[(&str, &str)] = &[
    ("opt-level", "Optimize for size."),
    ("lto", "Enable Link Time Optimization."),
    (
        "codegen-units",
        "Reduce number of codegen units to increase optimizations.",
    ),
    ("panic", "Abort on panic."),
    ("strip", "Strip symbols from the binary."),
];

/// Crates that make template tests painfully slow when compiled without optimizations.
const TEST_RUNTIME_CRATES: &[&str] = &[
    "wasmer",
    "wasmer-compiler",
    "wasmer-compiler-cranelift",
    "cranelift-codegen",
    "cranelift-frontend",
    "cranelift-entity",
];

#[derive(Clone, Parser, Debug)]
pub struct LintArgs {
    /// Path to the template crate directory (or its Cargo.toml).
    /// Defaults to the current directory.
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Apply the suggested fix for every issue that can be fixed automatically.
    /// Issues needing your input (e.g. a template description) are still only reported.
    #[arg(long, default_value_t = false)]
    pub fix: bool,

    /// Skip the `cargo clippy` run and only check the manifests.
    #[arg(long, default_value_t = false)]
    pub no_clippy: bool,

    /// Exit non-zero when any warning or suggestion is reported, not just errors.
    #[arg(long, short = 'D', default_value_t = false)]
    pub deny_warnings: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Severity {
    Suggestion,
    Warning,
    Error,
}

impl Severity {
    fn symbol(self) -> &'static str {
        match self {
            Severity::Error => "❌",
            Severity::Warning => "⚠️",
            Severity::Suggestion => "💡",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Suggestion => "suggestion",
        }
    }
}

/// A fix `--fix` knows how to apply on the user's behalf.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Fix {
    /// Write the size-optimizing `[profile.release]` into the workspace manifest.
    ReleaseProfile,
    /// Write `opt-level` overrides for the named WASM runtime crates into the workspace manifest.
    TestRuntimeProfiles(Vec<String>),
    /// Reduce `[lib] crate-type` to `["cdylib"]` in the crate manifest.
    CrateType,
    /// Add the metadata build dependency and `build.rs` to the crate.
    MetadataBuild,
}

#[derive(Debug)]
pub struct Finding {
    severity: Severity,
    /// Stable identifier, printed like a rustc lint name so it can be searched for.
    code: &'static str,
    message: String,
    /// Where the problem lives, e.g. `Cargo.toml [lib]`.
    location: String,
    /// The recommended solution. Rust lints have none — clippy prints its own.
    help: Option<String>,
    /// Set when `--fix` can resolve the finding without asking the user anything.
    fix: Option<Fix>,
}

impl Finding {
    fn new(severity: Severity, code: &'static str, message: impl Into<String>, location: impl Into<String>) -> Self {
        Self {
            severity,
            code,
            message: message.into(),
            location: location.into(),
            help: None,
            fix: None,
        }
    }

    fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    fn with_fix(mut self, fix: Fix) -> Self {
        self.fix = Some(fix);
        self
    }

    fn print(&self, fix_available: bool) {
        println!(
            "{} {}[{}]: {}",
            self.severity.symbol(),
            self.severity.label(),
            self.code,
            self.message
        );
        println!("   --> {}", self.location);
        if let Some(help) = &self.help {
            println!("   help:");
            for line in help.lines() {
                println!("       {line}");
            }
        }
        if fix_available && self.fix.is_some() {
            println!("   (fixable with `tari lint --fix`)");
        }
        println!();
    }
}

pub async fn handle(args: LintArgs) -> anyhow::Result<()> {
    let crate_dir = crate_dir(&args.path)?;
    let manifest_path = crate_dir.join("Cargo.toml");
    if !manifest_path.exists() {
        return Err(anyhow!("No Cargo.toml found at {}", manifest_path.display()));
    }

    let manifest = read_manifest(&manifest_path).await?;

    let crate_name = manifest
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or("<unknown>")
        .to_string();

    println!(
        "🔍 Linting template crate **{crate_name}** at {}\n",
        crate_dir.display()
    );

    // Cargo profiles are only honoured in the workspace root manifest, so profile checks have to
    // read (and point their fixes at) that manifest rather than the crate's own.
    let workspace_manifest_path = workspace_root(&crate_dir)
        .await
        .map(|root| root.join("Cargo.toml"))
        .unwrap_or_else(|_| manifest_path.clone());
    let same_manifest = workspace_manifest_path == manifest_path;
    let workspace_manifest = if same_manifest {
        manifest.clone()
    } else {
        read_manifest(&workspace_manifest_path).await?
    };
    let workspace_location = display_path(&workspace_manifest_path);
    let crate_location = display_path(&manifest_path);

    let mut findings = Vec::new();

    if !args.no_clippy {
        findings.extend(run_clippy(&manifest_path, args.fix).await?);
    }

    findings.extend(check_crate_type(&manifest, &crate_location));
    findings.extend(check_release_profile(&workspace_manifest, &workspace_location));
    if has_tests(&manifest, &crate_dir).await {
        findings.extend(check_test_runtime_profiles(&workspace_manifest, &workspace_location));
    }
    findings.extend(check_package_metadata(&manifest, &crate_location));
    findings.extend(check_metadata_generation(&manifest, &crate_dir, &crate_location).await);

    if args.fix {
        let fixer = Fixer {
            crate_manifest_path: manifest_path,
            crate_doc: manifest,
            workspace_manifest_path,
            workspace_doc: workspace_manifest,
            same_manifest,
        };
        findings = fixer.apply_all(findings, &crate_dir).await?;
    }

    report(findings, args.deny_warnings, args.fix)
}

async fn read_manifest(path: &Path) -> anyhow::Result<DocumentMut> {
    let src = fs::read_to_string(path)
        .await
        .with_context(|| format!("reading {}", path.display()))?;
    src.parse::<DocumentMut>()
        .with_context(|| format!("parsing {}", path.display()))
}

fn report(mut findings: Vec<Finding>, deny_warnings: bool, fixed_run: bool) -> anyhow::Result<()> {
    if findings.is_empty() {
        println!("✅ No issues found.");
        return Ok(());
    }

    // Stable sort: most severe first, insertion order preserved within a severity.
    findings.sort_by_key(|f| std::cmp::Reverse(f.severity));
    for finding in &findings {
        finding.print(!fixed_run);
    }

    let errors = count(&findings, Severity::Error);
    let warnings = count(&findings, Severity::Warning);
    let suggestions = count(&findings, Severity::Suggestion);
    println!("Summary: {errors} error(s), {warnings} warning(s), {suggestions} suggestion(s)");
    if !fixed_run && findings.iter().any(|f| f.fix.is_some()) {
        println!("Run `tari lint --fix` to apply the fixes marked above.");
    }

    if errors > 0 {
        return Err(anyhow!("lint failed with {errors} error(s)"));
    }
    if deny_warnings && warnings + suggestions > 0 {
        return Err(anyhow!(
            "lint failed with {} warning(s)/suggestion(s) (--deny-warnings)",
            warnings + suggestions
        ));
    }

    Ok(())
}

fn count(findings: &[Finding], severity: Severity) -> usize {
    findings.iter().filter(|f| f.severity == severity).count()
}

/// Accept either a crate directory or a path to its Cargo.toml.
fn crate_dir(path: &Path) -> anyhow::Result<PathBuf> {
    if path.file_name().is_some_and(|n| n == "Cargo.toml") {
        return Ok(path
            .parent()
            .ok_or_else(|| anyhow!("Cannot determine crate directory of {}", path.display()))?
            .to_path_buf());
    }
    Ok(path.to_path_buf())
}

/// Paths are shown relative to the working directory when possible — a lint report full of
/// absolute paths is hard to read.
fn display_path(path: &Path) -> String {
    std::env::current_dir()
        .ok()
        .and_then(|cwd| path.strip_prefix(cwd).ok())
        .unwrap_or(path)
        .display()
        .to_string()
}

async fn workspace_root(dir: &Path) -> anyhow::Result<PathBuf> {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version=1", "--no-deps"])
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;

    if !output.status.success() {
        return Err(anyhow!(
            "Failed to get cargo metadata: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout).context("parsing cargo metadata")?;
    metadata["workspace_root"]
        .as_str()
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("cargo metadata missing workspace_root"))
}

// --------------------------------------------------------------------------------------------
// Applying fixes
// --------------------------------------------------------------------------------------------

struct Fixer {
    crate_manifest_path: PathBuf,
    crate_doc: DocumentMut,
    workspace_manifest_path: PathBuf,
    workspace_doc: DocumentMut,
    /// True when the crate is its own workspace root — both edits then target one document.
    same_manifest: bool,
}

impl Fixer {
    /// Profiles are only honoured in the workspace root, which may be the crate manifest itself.
    fn profile_doc(&mut self) -> &mut DocumentMut {
        if self.same_manifest {
            &mut self.crate_doc
        } else {
            &mut self.workspace_doc
        }
    }

    /// Apply every fixable finding, returning the ones that remain.
    async fn apply_all(mut self, findings: Vec<Finding>, crate_dir: &Path) -> anyhow::Result<Vec<Finding>> {
        let (fixable, mut remaining): (Vec<Finding>, Vec<Finding>) =
            findings.into_iter().partition(|f| f.fix.is_some());

        if fixable.is_empty() {
            return Ok(remaining);
        }

        println!("🔧 Applying fixes\n");

        let mut crate_doc_dirty = false;
        let mut profile_doc_dirty = false;
        // `MetadataBuild` rewrites Cargo.toml from disk, so it must run after our own edits land.
        let mut metadata_build = false;

        for finding in &fixable {
            match finding.fix.as_ref().expect("partitioned on is_some") {
                Fix::ReleaseProfile => {
                    apply_release_profile(self.profile_doc())?;
                    profile_doc_dirty = true;
                    println!("   ✅ wrote size-optimizing [profile.release]");
                },
                Fix::TestRuntimeProfiles(crates) => {
                    apply_test_runtime_profiles(self.profile_doc(), crates)?;
                    profile_doc_dirty = true;
                    println!("   ✅ wrote dev opt-level overrides for {}", crates.join(", "));
                },
                Fix::CrateType => {
                    apply_crate_type(&mut self.crate_doc)?;
                    crate_doc_dirty = true;
                    println!("   ✅ set [lib] crate-type = [\"cdylib\"]");
                },
                Fix::MetadataBuild => metadata_build = true,
            }
        }

        if crate_doc_dirty || (profile_doc_dirty && self.same_manifest) {
            write_manifest(&self.crate_manifest_path, &self.crate_doc).await?;
        }
        if profile_doc_dirty && !self.same_manifest {
            write_manifest(&self.workspace_manifest_path, &self.workspace_doc).await?;
        }

        if metadata_build {
            init_metadata::auto_init(crate_dir).await?;
            println!("   ✅ added {BUILD_DEP_KEY} and build.rs");
        }

        println!("\n🔧 Fixed {} issue(s)\n", fixable.len());

        // Anything left needs the user; make that explicit rather than silently passing.
        remaining.retain(|f| f.fix.is_none());
        Ok(remaining)
    }
}

async fn write_manifest(path: &Path, doc: &DocumentMut) -> anyhow::Result<()> {
    fs::write(path, doc.to_string())
        .await
        .with_context(|| format!("writing {}", path.display()))
}

/// `table[key] = value  # comment`, preserving the comment style of the documented snippet.
fn set_commented(table: &mut Table, key: &str, val: Value, comment: &str) {
    let padding = " ".repeat(
        20usize
            .saturating_sub(key.len() + val.to_string().trim().len() + 4)
            .max(1),
    );
    table.insert(key, Item::Value(val.decorated(" ", format!("{padding}# {comment}"))));
}

fn sub_table<'a>(table: &'a mut Table, key: &str) -> anyhow::Result<&'a mut Table> {
    let entry = table.entry(key).or_insert_with(|| Item::Table(Table::new()));
    entry
        .as_table_mut()
        .ok_or_else(|| anyhow!("[{key}] exists but is not a table"))
}

/// Fetch (creating as needed) a nested table, marking intermediate tables implicit so they render
/// as `[profile.dev.package.wasmer]` rather than an empty `[profile]` header.
fn nested_table<'a>(doc: &'a mut DocumentMut, path: &[&str]) -> anyhow::Result<&'a mut Table> {
    let mut table = doc.as_table_mut();
    for (i, key) in path.iter().enumerate() {
        table = sub_table(table, key)?;
        if i + 1 < path.len() {
            table.set_implicit(true);
        }
    }
    Ok(table)
}

fn apply_release_profile(doc: &mut DocumentMut) -> anyhow::Result<()> {
    let release = nested_table(doc, &["profile", "release"])?;
    let comment = |key: &str| {
        RELEASE_PROFILE_KEYS
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, c)| *c)
            .unwrap_or_default()
    };
    set_commented(release, "opt-level", "s".into(), comment("opt-level"));
    set_commented(release, "lto", true.into(), comment("lto"));
    set_commented(release, "codegen-units", 1.into(), comment("codegen-units"));
    set_commented(release, "panic", "abort".into(), comment("panic"));
    set_commented(release, "strip", true.into(), comment("strip"));
    Ok(())
}

fn apply_test_runtime_profiles(doc: &mut DocumentMut, crates: &[String]) -> anyhow::Result<()> {
    for name in crates {
        let table = nested_table(doc, &["profile", "dev", "package", name])?;
        table.insert("opt-level", value(2));
    }
    Ok(())
}

fn apply_crate_type(doc: &mut DocumentMut) -> anyhow::Result<()> {
    let lib = nested_table(doc, &["lib"])?;
    let mut arr = Array::new();
    arr.push("cdylib");
    lib.insert("crate-type", value(arr));
    Ok(())
}

// --------------------------------------------------------------------------------------------
// 1. Rust lints
// --------------------------------------------------------------------------------------------

/// Run `cargo clippy` against the WASM target, printing clippy's own rendered diagnostics and
/// summarising them as findings. Clippy already explains how to fix what it reports, so these
/// findings carry no `help` of their own. With `fix`, clippy applies its machine-applicable
/// suggestions itself and only the remaining diagnostics are reported.
async fn run_clippy(manifest_path: &Path, fix: bool) -> anyhow::Result<Vec<Finding>> {
    if fix {
        println!("▶ Running `cargo clippy --fix --target {WASM_TARGET}`\n");
    } else {
        println!("▶ Running `cargo clippy --target {WASM_TARGET}`\n");
    }

    let mut cmd = Command::new("cargo");
    cmd.arg("clippy")
        .arg("--manifest-path")
        .arg(manifest_path)
        .arg(format!("--target={WASM_TARGET}"));
    if fix {
        // Templates are usually edited in a dirty tree; refusing to fix there is unhelpful.
        cmd.args(["--fix", "--allow-dirty", "--allow-staged"]);
    }
    let output = cmd
        .arg("--message-format=json")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .context("running cargo clippy (is cargo installed?)")?;

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Toolchain problems are not template problems — tell the user how to fix their setup.
    if stderr.contains("no such command") || stderr.contains("not installed for the toolchain") {
        return Ok(vec![
            Finding::new(
                Severity::Warning,
                "rust::clippy-missing",
                "cargo clippy is not installed, skipping Rust lints",
                "toolchain",
            )
            .with_help("rustup component add clippy"),
        ]);
    }
    if stderr.contains("target may not be installed") || stderr.contains("can't find crate for `core`") {
        return Ok(vec![
            Finding::new(
                Severity::Error,
                "rust::wasm-target-missing",
                format!("the `{WASM_TARGET}` target is not installed"),
                "toolchain",
            )
            .with_help(format!("rustup target add {WASM_TARGET}")),
        ]);
    }

    let (errors, warnings) = print_and_count_diagnostics(&String::from_utf8_lossy(&output.stdout));

    // A non-zero exit with no parsed diagnostics means cargo itself failed (bad manifest,
    // unresolvable dependency, ...). Surface its stderr rather than silently passing.
    if !output.status.success() && errors == 0 {
        return Ok(vec![Finding::new(
            Severity::Error,
            "rust::clippy-failed",
            format!("cargo clippy failed to run:\n{}", stderr.trim()),
            "toolchain",
        )]);
    }

    let mut findings = Vec::new();
    if errors > 0 {
        findings.push(Finding::new(
            Severity::Error,
            "rust::clippy",
            format!("cargo clippy reported {errors} error(s), see the output above"),
            "crate sources",
        ));
    }
    if warnings > 0 {
        findings.push(Finding::new(
            Severity::Warning,
            "rust::clippy",
            format!("cargo clippy reported {warnings} warning(s), see the output above"),
            "crate sources",
        ));
    }
    if findings.is_empty() {
        println!("✅ No clippy findings\n");
    } else {
        println!();
    }

    Ok(findings)
}

/// Print each rendered clippy diagnostic and return `(errors, warnings)`.
fn print_and_count_diagnostics(stdout: &str) -> (usize, usize) {
    let mut errors = 0usize;
    let mut warnings = 0usize;

    for line in stdout.lines() {
        let Ok(msg) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if msg["reason"].as_str() != Some("compiler-message") {
            continue;
        }
        let diagnostic = &msg["message"];
        // Trailing summaries ("aborting due to N errors", "`foo` generated 3 warnings") have no
        // code and no spans; counting them would double-report.
        let is_summary =
            diagnostic["code"].is_null() && diagnostic["spans"].as_array().is_none_or(|spans| spans.is_empty());
        match diagnostic["level"].as_str() {
            Some("error" | "error: internal compiler error") if !is_summary => errors += 1,
            Some("warning") if !is_summary => warnings += 1,
            _ => {},
        }
        if let Some(rendered) = diagnostic["rendered"].as_str() {
            print!("{rendered}");
        }
    }

    (errors, warnings)
}

// --------------------------------------------------------------------------------------------
// 2. Release profile (binary size)
// --------------------------------------------------------------------------------------------

fn check_release_profile(doc: &DocumentMut, location: &str) -> Vec<Finding> {
    let profile = doc.get("profile").and_then(|p| p.get("release"));
    let get = |key: &str| profile.and_then(|p| p.get(key));

    let mut missing: Vec<&str> = Vec::new();
    let mut check = |key: &'static str, ok: bool| {
        if !ok {
            missing.push(key);
        }
    };

    check(
        "opt-level = 's'",
        get("opt-level").is_some_and(|v| matches!(v.as_str(), Some("s" | "z"))),
    );
    check(
        "lto = true",
        get("lto").is_some_and(|v| v.as_bool() == Some(true) || matches!(v.as_str(), Some("fat" | "thin"))),
    );
    check(
        "codegen-units = 1",
        get("codegen-units").and_then(|v| v.as_integer()) == Some(1),
    );
    // `immediate-abort` skips the unwinding machinery entirely, so it is at least as good as
    // `abort` for binary size.
    check(
        "panic = 'abort'",
        matches!(get("panic").and_then(|v| v.as_str()), Some("abort" | "immediate-abort")),
    );
    check(
        "strip = true",
        get("strip").is_some_and(|v| v.as_bool() == Some(true) || matches!(v.as_str(), Some("symbols" | "debuginfo"))),
    );

    if missing.is_empty() {
        return Vec::new();
    }

    let message = if profile.is_none() {
        "missing `[profile.release]` size optimizations — the published WASM binary will be larger (and cost more \
         to publish) than it needs to be"
            .to_string()
    } else {
        format!(
            "`[profile.release]` is missing size optimizations: {}",
            missing.join(", ")
        )
    };

    vec![
        Finding::new(
            Severity::Warning,
            "cargo::release-profile",
            message,
            format!("{location} [profile.release]"),
        )
        .with_help(format!(
            "Add (or complete) this section in {location}:\n\n{RELEASE_PROFILE_SNIPPET}\n\n\
             Note: `tari build` and `tari publish` already pass these via `cargo --config`, but a plain \
             `cargo build --release` does not."
        ))
        .with_fix(Fix::ReleaseProfile),
    ]
}

// --------------------------------------------------------------------------------------------
// 3. Test runtime optimizations
// --------------------------------------------------------------------------------------------

fn check_test_runtime_profiles(doc: &DocumentMut, location: &str) -> Vec<Finding> {
    let packages = doc
        .get("profile")
        .and_then(|p| p.get("dev"))
        .and_then(|d| d.get("package"));

    let missing: Vec<&str> = TEST_RUNTIME_CRATES
        .iter()
        .copied()
        .filter(|name| {
            !packages
                .and_then(|p| p.get(name))
                .and_then(|p| p.get("opt-level"))
                .is_some_and(is_optimized)
        })
        .collect();

    if missing.is_empty() {
        return Vec::new();
    }

    let snippet: String = missing
        .iter()
        .map(|name| format!("[profile.dev.package.{name}]\nopt-level = 2\n"))
        .collect::<Vec<_>>()
        .join("\n");

    vec![
        Finding::new(
            Severity::Suggestion,
            "cargo::test-runtime-profile",
            format!(
                "template tests run ~10x slower without optimized WASM runtime crates ({} unoptimized in dev builds)",
                missing.join(", ")
            ),
            format!("{location} [profile.dev.package]"),
        )
        .with_help(format!(
            "Wasmer and Cranelift are extremely slow when compiled in debug mode, which makes template\n\
             tests painfully slow. Optimize these crates even in dev/test builds by adding to {location}:\n\n{}",
            snippet.trim_end()
        ))
        .with_fix(Fix::TestRuntimeProfiles(
            missing.iter().map(|s| s.to_string()).collect(),
        )),
    ]
}

/// `opt-level` counts as optimized when it is any non-zero level, numeric or size-oriented.
fn is_optimized(item: &Item) -> bool {
    if let Some(level) = item.as_integer() {
        return level > 0;
    }
    matches!(item.as_str(), Some("1" | "2" | "3" | "s" | "z"))
}

// --------------------------------------------------------------------------------------------
// 4. Package metadata
// --------------------------------------------------------------------------------------------

fn check_package_metadata(doc: &DocumentMut, location: &str) -> Vec<Finding> {
    let package = doc.get("package");
    let tari_template = package
        .and_then(|p| p.get("metadata"))
        .and_then(|m| m.get(TARI_TEMPLATE_METADATA_KEY));

    let non_empty_str = |item: Option<&Item>| item.and_then(|v| v.as_str()).is_some_and(|s| !s.trim().is_empty());

    let mut missing: Vec<(&str, &str)> = Vec::new();
    if !non_empty_str(package.and_then(|p| p.get("description"))) {
        missing.push(("description", "--description \"<what the template does>\""));
    }

    let has_tags = tari_template
        .and_then(|t| t.get("tags"))
        .and_then(|t| t.as_array())
        .is_some_and(|arr| arr.iter().any(|v| v.as_str().is_some_and(|s| !s.trim().is_empty())));
    if !has_tags {
        missing.push(("tags", "--tags \"<tag1,tag2>\""));
    }

    for (field, flag) in [
        ("category", "--category \"<category>\""),
        ("documentation", "--documentation \"<url>\""),
        ("homepage", "--homepage \"<url>\""),
        ("logo_url", "--logo-url \"<url>\""),
    ] {
        if !non_empty_str(tari_template.and_then(|t| t.get(field))) {
            missing.push((field, flag));
        }
    }

    if missing.is_empty() {
        return Vec::new();
    }

    // A missing description hides the template in listings, the rest only degrade discoverability.
    let severity = if missing.iter().any(|(field, _)| *field == "description") {
        Severity::Warning
    } else {
        Severity::Suggestion
    };

    let fields = missing.iter().map(|(field, _)| *field).collect::<Vec<_>>().join(", ");
    let flags = missing
        .iter()
        .map(|(_, flag)| *flag)
        .collect::<Vec<_>>()
        .join(" \\\n        ");

    // Deliberately not auto-fixable: only the author knows what to put in these fields.
    vec![
        Finding::new(
            severity,
            "template::metadata",
            format!("missing template metadata: {fields} — published templates without it are hard to discover"),
            format!("{location} [package.metadata.{TARI_TEMPLATE_METADATA_KEY}]"),
        )
        .with_help(format!(
            "Fill it in interactively:\n\n    \
             tari template init\n\n\
             or non-interactively:\n\n    \
             tari template init -y \\\n        {flags}\n\n\
             which writes:\n\n\
             description = \"A guessing game\"\n\n\
             [package.metadata.{TARI_TEMPLATE_METADATA_KEY}]\n\
             tags = [\"game\", \"fun\", \"example\"]\n\
             category = \"game\""
        )),
    ]
}

/// Metadata in Cargo.toml is only emitted into the published binary when the crate runs
/// `tari_ootle_template_build` from a build script.
async fn check_metadata_generation(doc: &DocumentMut, crate_dir: &Path, location: &str) -> Vec<Finding> {
    let has_build_dep = doc
        .get("build-dependencies")
        .and_then(|d| d.get(BUILD_DEP_KEY))
        .is_some();
    let build_rs = crate_dir.join("build.rs");
    let has_build_rs = fs::read_to_string(&build_rs)
        .await
        .is_ok_and(|src| src.contains("TemplateMetadataBuilder"));

    if has_build_dep && has_build_rs {
        return Vec::new();
    }

    let what = match (has_build_dep, has_build_rs) {
        (false, false) => format!("neither `{BUILD_DEP_KEY}` in [build-dependencies] nor a metadata build.rs"),
        (false, true) => format!("`{BUILD_DEP_KEY}` is missing from [build-dependencies]"),
        // (true, true) returned above.
        _ => "build.rs does not call `TemplateMetadataBuilder`".to_string(),
    };

    let mut finding = Finding::new(
        Severity::Warning,
        "template::metadata-build",
        format!("template metadata will not be generated: {what}"),
        location.to_string(),
    )
    .with_help(
        "Run:\n\n    tari template init\n\nwhich adds the build dependency and a build.rs containing:\n\n\
         fn main() {\n    \
         tari_ootle_template_build::TemplateMetadataBuilder::new()\n        \
         .build()\n        \
         .expect(\"Failed to build template metadata\");\n\
         }",
    );

    // An existing build.rs that does something else is the user's to reconcile.
    if !build_rs.exists() || has_build_rs {
        finding = finding.with_fix(Fix::MetadataBuild);
    }

    vec![finding]
}

// --------------------------------------------------------------------------------------------
// 5. crate-type
// --------------------------------------------------------------------------------------------

fn check_crate_type(doc: &DocumentMut, location: &str) -> Vec<Finding> {
    let crate_type = doc.get("lib").and_then(|l| l.get("crate-type"));

    let Some(types) = crate_type.and_then(|c| c.as_array()) else {
        return vec![
            Finding::new(
                Severity::Error,
                "cargo::crate-type",
                "no `crate-type` declared — building for wasm32 will not produce a `.wasm` binary",
                format!("{location} [lib]"),
            )
            .with_help(format!("Add to {location}:\n\n[lib]\ncrate-type = [\"cdylib\"]"))
            .with_fix(Fix::CrateType),
        ];
    };

    let types: Vec<&str> = types.iter().filter_map(|v| v.as_str()).collect();
    let extra: Vec<&str> = types.iter().copied().filter(|t| *t != "cdylib").collect();

    if !types.contains(&"cdylib") {
        return vec![
            Finding::new(
                Severity::Error,
                "cargo::crate-type",
                format!(
                    "`crate-type = {types:?}` does not include `cdylib` — building for wasm32 will not produce a \
                     `.wasm` binary"
                ),
                format!("{location} [lib]"),
            )
            .with_help(format!("In {location}:\n\n[lib]\ncrate-type = [\"cdylib\"]"))
            .with_fix(Fix::CrateType),
        ];
    }

    if extra.is_empty() {
        return Vec::new();
    }

    vec![
        Finding::new(
            Severity::Warning,
            "cargo::crate-type",
            format!(
                "`crate-type` also contains {} — every extra crate type is compiled and linked in, bloating the \
                 published binary",
                extra.iter().map(|t| format!("`{t}`")).collect::<Vec<_>>().join(", ")
            ),
            format!("{location} [lib]"),
        )
        .with_help(format!(
            "In {location}, keep only the dynamic library:\n\n[lib]\ncrate-type = [\"cdylib\"]"
        ))
        .with_fix(Fix::CrateType),
    ]
}

/// The test-runtime check is only relevant to crates that actually have tests.
async fn has_tests(doc: &DocumentMut, crate_dir: &Path) -> bool {
    doc.get("dev-dependencies").is_some() || fs::try_exists(crate_dir.join("tests")).await.unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(src: &str) -> DocumentMut {
        src.parse::<DocumentMut>().unwrap()
    }

    const COMPLETE_RELEASE_PROFILE: &str = r#"
[profile.release]
opt-level = 's'
lto = true
codegen-units = 1
panic = 'abort'
strip = true
"#;

    #[test]
    fn crate_type_only_cdylib_is_clean() {
        let findings = check_crate_type(&doc("[lib]\ncrate-type = [\"cdylib\"]\n"), "Cargo.toml");
        assert!(findings.is_empty());
    }

    #[test]
    fn crate_type_flags_extra_types() {
        let findings = check_crate_type(&doc("[lib]\ncrate-type = [\"cdylib\", \"rlib\"]\n"), "Cargo.toml");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Warning);
        assert!(findings[0].message.contains("`rlib`"));
        assert_eq!(findings[0].fix, Some(Fix::CrateType));
    }

    #[test]
    fn crate_type_missing_is_an_error() {
        let findings = check_crate_type(&doc("[package]\nname = \"t\"\n"), "Cargo.toml");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Error);
    }

    #[test]
    fn crate_type_without_cdylib_is_an_error() {
        let findings = check_crate_type(&doc("[lib]\ncrate-type = [\"rlib\"]\n"), "Cargo.toml");
        assert_eq!(findings[0].severity, Severity::Error);
        assert!(findings[0].message.contains("does not include `cdylib`"));
    }

    #[test]
    fn complete_release_profile_is_clean() {
        assert!(check_release_profile(&doc(COMPLETE_RELEASE_PROFILE), "Cargo.toml").is_empty());
    }

    #[test]
    fn release_profile_accepts_equivalent_values() {
        let findings = check_release_profile(
            &doc(
                "[profile.release]\nopt-level = 'z'\nlto = 'fat'\ncodegen-units = 1\npanic = 'abort'\nstrip = 'symbols'\n",
            ),
            "Cargo.toml",
        );
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn missing_release_profile_is_reported_once() {
        let findings = check_release_profile(&doc("[package]\nname = \"t\"\n"), "Cargo.toml");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Warning);
        assert_eq!(findings[0].fix, Some(Fix::ReleaseProfile));
        assert!(findings[0].help.as_ref().unwrap().contains("opt-level = 's'"));
    }

    #[test]
    fn partial_release_profile_lists_only_missing_keys() {
        let findings = check_release_profile(&doc("[profile.release]\nopt-level = 's'\nlto = true\n"), "Cargo.toml");
        let message = &findings[0].message;
        assert!(message.contains("codegen-units"), "{message}");
        assert!(message.contains("panic"), "{message}");
        assert!(message.contains("strip"), "{message}");
        assert!(!message.contains("lto"), "{message}");
    }

    #[test]
    fn immediate_abort_panic_is_accepted() {
        let findings = check_release_profile(
            &doc("[profile.release]\nopt-level = 's'\nlto = true\ncodegen-units = 1\n\
                 panic = 'immediate-abort'\nstrip = true\n"),
            "Cargo.toml",
        );
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn speed_opt_level_still_flagged_for_size() {
        let findings = check_release_profile(&doc("[profile.release]\nopt-level = 3\n"), "Cargo.toml");
        assert!(findings[0].message.contains("opt-level"));
    }

    #[test]
    fn applying_release_profile_fix_satisfies_the_check() {
        let mut manifest = doc("[package]\nname = \"t\"\n");
        apply_release_profile(&mut manifest).unwrap();
        assert!(check_release_profile(&manifest, "Cargo.toml").is_empty());
        let rendered = manifest.to_string();
        assert!(rendered.contains("[profile.release]"), "{rendered}");
        assert!(rendered.contains("# Optimize for size."), "{rendered}");
    }

    #[test]
    fn applying_crate_type_fix_satisfies_the_check() {
        let mut manifest = doc("[package]\nname = \"t\"\n\n[lib]\ncrate-type = [\"cdylib\", \"rlib\"]\n");
        apply_crate_type(&mut manifest).unwrap();
        assert!(check_crate_type(&manifest, "Cargo.toml").is_empty());
        assert!(manifest.to_string().contains("crate-type = [\"cdylib\"]"));
    }

    #[test]
    fn test_runtime_profiles_clean_when_all_optimized() {
        let src: String = TEST_RUNTIME_CRATES
            .iter()
            .map(|name| format!("[profile.dev.package.{name}]\nopt-level = 2\n"))
            .collect();
        assert!(check_test_runtime_profiles(&doc(&src), "Cargo.toml").is_empty());
    }

    #[test]
    fn test_runtime_profiles_report_missing_crates_only() {
        let findings = check_test_runtime_profiles(&doc("[profile.dev.package.wasmer]\nopt-level = 2\n"), "Cargo.toml");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Suggestion);
        assert!(!findings[0].message.contains("(wasmer,"), "{}", findings[0].message);
        assert!(findings[0].message.contains("cranelift-codegen"));
    }

    #[test]
    fn zero_opt_level_counts_as_unoptimized() {
        let findings = check_test_runtime_profiles(&doc("[profile.dev.package.wasmer]\nopt-level = 0\n"), "Cargo.toml");
        assert!(findings[0].message.contains("wasmer,"), "{}", findings[0].message);
    }

    #[test]
    fn applying_test_runtime_fix_satisfies_the_check() {
        let mut manifest = doc("[package]\nname = \"t\"\n");
        let crates: Vec<String> = TEST_RUNTIME_CRATES.iter().map(|s| s.to_string()).collect();
        apply_test_runtime_profiles(&mut manifest, &crates).unwrap();
        assert!(check_test_runtime_profiles(&manifest, "Cargo.toml").is_empty());
        assert!(manifest.to_string().contains("[profile.dev.package.cranelift-entity]"));
    }

    #[test]
    fn complete_metadata_is_clean() {
        let manifest = doc(r#"
[package]
name = "t"
description = "A guessing game"

[package.metadata.tari-template]
tags = ["game"]
category = "game"
documentation = "https://example.com/docs"
homepage = "https://example.com"
logo_url = "https://example.com/logo.png"
"#);
        assert!(check_package_metadata(&manifest, "Cargo.toml").is_empty());
    }

    #[test]
    fn missing_description_is_a_warning() {
        let manifest = doc(r#"
[package]
name = "t"

[package.metadata.tari-template]
tags = ["game"]
category = "game"
documentation = "https://example.com/docs"
homepage = "https://example.com"
logo_url = "https://example.com/logo.png"
"#);
        let findings = check_package_metadata(&manifest, "Cargo.toml");
        assert_eq!(findings[0].severity, Severity::Warning);
        assert!(findings[0].help.as_ref().unwrap().contains("tari template init"));
    }

    #[test]
    fn missing_optional_fields_are_only_suggestions() {
        let manifest = doc(r#"
[package]
name = "t"
description = "A guessing game"

[package.metadata.tari-template]
tags = ["game"]
category = "game"
"#);
        let findings = check_package_metadata(&manifest, "Cargo.toml");
        assert_eq!(findings[0].severity, Severity::Suggestion);
        assert!(findings[0].message.contains("documentation"));
        // Only the author can supply these, so `--fix` must not claim to.
        assert_eq!(findings[0].fix, None);
    }

    #[test]
    fn empty_metadata_values_count_as_missing() {
        let manifest = doc(r#"
[package]
name = "t"
description = "   "

[package.metadata.tari-template]
tags = [""]
"#);
        let findings = check_package_metadata(&manifest, "Cargo.toml");
        assert!(findings[0].message.contains("description"));
        assert!(findings[0].message.contains("tags"));
    }

    #[tokio::test]
    async fn metadata_generation_clean_when_configured() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("build.rs"),
            "fn main() { tari_ootle_template_build::TemplateMetadataBuilder::new(); }",
        )
        .unwrap();
        let manifest = doc("[build-dependencies]\ntari_ootle_template_build = \"0.7\"\n");
        assert!(
            check_metadata_generation(&manifest, dir.path(), "Cargo.toml")
                .await
                .is_empty()
        );
    }

    #[tokio::test]
    async fn metadata_generation_missing_is_fixable() {
        let dir = tempfile::tempdir().unwrap();
        let findings = check_metadata_generation(&doc("[package]\nname = \"t\"\n"), dir.path(), "Cargo.toml").await;
        assert_eq!(findings[0].severity, Severity::Warning);
        assert_eq!(findings[0].fix, Some(Fix::MetadataBuild));
    }

    #[tokio::test]
    async fn foreign_build_rs_is_not_auto_fixable() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("build.rs"),
            "fn main() { println!(\"something else\"); }",
        )
        .unwrap();
        let manifest = doc("[build-dependencies]\ntari_ootle_template_build = \"0.7\"\n");
        let findings = check_metadata_generation(&manifest, dir.path(), "Cargo.toml").await;
        assert_eq!(
            findings[0].fix, None,
            "we must not silently rewrite someone else's build.rs"
        );
    }

    #[test]
    fn clippy_summary_lines_are_not_counted() {
        let stdout = concat!(
            r#"{"reason":"compiler-message","message":{"level":"warning","code":{"code":"clippy::ptr_arg"},"spans":[{"line_start":1}],"rendered":""}}"#,
            "\n",
            r#"{"reason":"compiler-message","message":{"level":"warning","code":null,"spans":[],"rendered":""}}"#,
            "\n",
            r#"{"reason":"build-finished","success":true}"#,
            "\n",
        );
        assert_eq!(print_and_count_diagnostics(stdout), (0, 1));
    }

    #[test]
    fn clippy_errors_are_counted() {
        let stdout = concat!(
            r#"{"reason":"compiler-message","message":{"level":"error","code":{"code":"E0425"},"spans":[{"line_start":3}],"rendered":""}}"#,
            "\n",
            r#"{"reason":"compiler-message","message":{"level":"error","code":null,"spans":[],"rendered":""}}"#,
            "\n",
        );
        assert_eq!(print_and_count_diagnostics(stdout), (1, 0));
    }

    #[test]
    fn accepts_a_manifest_path_as_well_as_a_directory() {
        assert_eq!(crate_dir(Path::new("foo/Cargo.toml")).unwrap(), PathBuf::from("foo"));
        assert_eq!(crate_dir(Path::new("foo")).unwrap(), PathBuf::from("foo"));
    }

    #[test]
    fn errors_fail_the_run_and_warnings_do_not() {
        let error = || Finding::new(Severity::Error, "x", "boom", "Cargo.toml");
        let warning = || Finding::new(Severity::Warning, "x", "meh", "Cargo.toml");
        assert!(report(vec![warning()], false, false).is_ok());
        assert!(report(vec![warning()], true, false).is_err());
        assert!(report(vec![error()], false, false).is_err());
        assert!(report(vec![], false, false).is_ok());
    }
}
