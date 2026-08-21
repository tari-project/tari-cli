// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

//! Minimal crates.io lookups, so the CLI can write a current version requirement into a
//! template's manifest instead of a hardcoded one that silently goes stale.
//!
//! Uses the [sparse index] rather than the crates.io API: it needs no authentication, has no
//! User-Agent policy, and returns one compact JSON line per published version.
//!
//! [sparse index]: https://doc.rust-lang.org/cargo/reference/registry-index.html

use std::time::Duration;

use anyhow::{Context, anyhow};

const SPARSE_INDEX_URL: &str = "https://index.crates.io";

/// Kept short: every caller has a usable fallback, so waiting is worse than giving up.
const LOOKUP_TIMEOUT: Duration = Duration::from_secs(3);

/// The latest stable version of `crate_name` as a `major.minor` requirement (e.g. `"0.11"`),
/// which is what a Cargo dependency conventionally pins.
///
/// Yanked and pre-release versions are ignored.
pub async fn latest_version_req(crate_name: &str) -> anyhow::Result<String> {
    let url = format!("{SPARSE_INDEX_URL}/{}", index_path(crate_name));
    let response = reqwest::Client::builder()
        .timeout(LOOKUP_TIMEOUT)
        .build()?
        .get(&url)
        .send()
        .await
        .with_context(|| format!("querying {url}"))?;

    if !response.status().is_success() {
        return Err(anyhow!("{url} returned {}", response.status()));
    }

    let body = response.text().await.context("reading crates.io index response")?;
    let version =
        latest_stable_version(&body).ok_or_else(|| anyhow!("no stable versions of {crate_name} found on crates.io"))?;

    Ok(format!("{}.{}", version.0, version.1))
}

/// The crates.io index shards by name length: 1 and 2 character names live under `1/` and `2/`,
/// 3 character names under `3/{first char}/`, and everything else under `{ab}/{cd}/`.
fn index_path(crate_name: &str) -> String {
    let name = crate_name.to_lowercase();
    match name.len() {
        0 => name,
        1 => format!("1/{name}"),
        2 => format!("2/{name}"),
        3 => format!("3/{}/{name}", &name[0..1]),
        _ => format!("{}/{}/{name}", &name[0..2], &name[2..4]),
    }
}

/// Highest non-yanked, non-prerelease version in a sparse index response (one JSON object per
/// line). Lines are ordered by publication, not by version, so they are compared numerically.
fn latest_stable_version(body: &str) -> Option<(u64, u64, u64)> {
    body.lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|entry| entry["yanked"].as_bool() != Some(true))
        .filter_map(|entry| parse_stable_version(entry["vers"].as_str()?))
        .max()
}

/// `1.2.3` -> `(1, 2, 3)`. Pre-release and build-metadata versions are rejected: a template should
/// not be pinned to one by default.
fn parse_stable_version(vers: &str) -> Option<(u64, u64, u64)> {
    if vers.contains(['-', '+']) {
        return None;
    }
    let mut parts = vers.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next().unwrap_or("0").parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_paths_shard_by_name_length() {
        assert_eq!(index_path("a"), "1/a");
        assert_eq!(index_path("ab"), "2/ab");
        assert_eq!(index_path("abc"), "3/a/abc");
        assert_eq!(
            index_path("tari_ootle_template_build"),
            "ta/ri/tari_ootle_template_build"
        );
    }

    #[test]
    fn index_paths_are_lowercased() {
        assert_eq!(index_path("Inflector"), "in/fl/inflector");
    }

    #[test]
    fn picks_the_highest_version_not_the_last_line() {
        // A patch release of an older line is published after a newer minor.
        let body = concat!(
            r#"{"name":"c","vers":"0.9.0","yanked":false}"#,
            "\n",
            r#"{"name":"c","vers":"0.11.0","yanked":false}"#,
            "\n",
            r#"{"name":"c","vers":"0.9.1","yanked":false}"#,
            "\n",
        );
        assert_eq!(latest_stable_version(body), Some((0, 11, 0)));
    }

    #[test]
    fn skips_yanked_and_prerelease_versions() {
        let body = concat!(
            r#"{"name":"c","vers":"0.7.0","yanked":false}"#,
            "\n",
            r#"{"name":"c","vers":"0.8.0","yanked":true}"#,
            "\n",
            r#"{"name":"c","vers":"0.9.0-rc.1","yanked":false}"#,
            "\n",
        );
        assert_eq!(latest_stable_version(body), Some((0, 7, 0)));
    }

    #[test]
    fn ignores_unparseable_lines() {
        let body = concat!("not json\n", r#"{"name":"c","vers":"1.2.3","yanked":false}"#, "\n");
        assert_eq!(latest_stable_version(body), Some((1, 2, 3)));
    }

    #[test]
    fn no_stable_versions_yields_none() {
        let body = concat!(r#"{"name":"c","vers":"0.1.0","yanked":true}"#, "\n");
        assert_eq!(latest_stable_version(body), None);
    }

    #[test]
    fn version_parsing_rejects_junk() {
        assert_eq!(parse_stable_version("1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_stable_version("1.2"), Some((1, 2, 0)));
        assert_eq!(parse_stable_version("1.2.3.4"), None);
        assert_eq!(parse_stable_version("1.2.3-beta"), None);
        assert_eq!(parse_stable_version("nope"), None);
    }
}
