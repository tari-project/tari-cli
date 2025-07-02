// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use std::path::Path;

pub mod repository;

/// Finds the root of a Git repository by traversing up the directory tree.
pub fn find_git_root<P: AsRef<Path>>(path: P) -> Option<std::path::PathBuf> {
    let mut current = path.as_ref().to_path_buf();
    if !current.exists() {
        return None; // If the path does not exist, return None
    }
    if current.is_file() {
        current.pop(); // If it's a file, go to the parent directory
    }
    loop {
        if current.join(".git").exists() {
            return Some(current);
        }
        if !current.pop() {
            break;
        }
    }
    None
}
