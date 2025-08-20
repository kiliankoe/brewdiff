use crate::error::{Error, Result};
use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// What nix-darwin wants to be installed
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HomebrewIntent {
    pub brews: HashSet<String>,
    pub casks: HashSet<String>,
    pub taps: HashSet<String>,
}

impl HomebrewIntent {
    /// Extract Homebrew intent from a nix-darwin profile
    pub fn extract(profile: &Path) -> Result<Self> {
        Self::extract_from_activation_script(profile)
    }

    fn extract_from_activation_script(profile: &Path) -> Result<Self> {
        let activate_path = profile.join("activate");
        if !activate_path.exists() {
            return Err(Error::NoActivationScript(
                activate_path.to_string_lossy().to_string(),
            ));
        }

        let content = fs::read_to_string(&activate_path)?;

        // Look for the brew bundle command
        // Example: brew bundle --file='/nix/store/xxx-Brewfile' --no-upgrade
        // Also handle paths that aren't in /nix/store for testing
        let brewfile_regex = Regex::new(r"brew bundle --file='([^']+Brewfile)'.*")?;

        if let Some(captures) = brewfile_regex.captures(&content) {
            let brewfile_path = captures.get(1).unwrap().as_str();
            return Self::parse_brewfile(Path::new(brewfile_path));
        }

        Err(Error::BrewfileNotFound)
    }

    fn parse_brewfile(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Err(Error::ParseError(format!(
                "Brewfile not found at: {}",
                path.display()
            )));
        }

        let content = fs::read_to_string(path)?;
        let mut intent = Self::default();

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('#') || line.is_empty() {
                continue;
            }

            if line.starts_with("brew \"") {
                if let Some(formula) = Self::extract_quoted_value(line) {
                    intent.brews.insert(formula);
                }
            } else if line.starts_with("cask \"") {
                if let Some(cask) = Self::extract_quoted_value(line) {
                    intent.casks.insert(cask);
                }
            } else if line.starts_with("tap \"") {
                if let Some(tap) = Self::extract_quoted_value(line) {
                    intent.taps.insert(tap);
                }
            }
            // TODO: Add support for mas apps when needed
        }

        Ok(intent)
    }

    fn extract_quoted_value(line: &str) -> Option<String> {
        let start = line.find('"')?;
        let end = line[start + 1..].find('"')?;
        Some(line[start + 1..start + 1 + end].to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_parse_brewfile() {
        let temp_dir = TempDir::new().unwrap();
        let brewfile_path = temp_dir.path().join("Brewfile");

        let brewfile_content = r#"
# Created by `nix-darwin`'s `homebrew` module

# Taps
tap "homebrew/bundle"
tap "homebrew/core"

# Brews
brew "wget"
brew "curl"

# Casks
cask "firefox"
cask "visual-studio-code"
"#;

        fs::write(&brewfile_path, brewfile_content).unwrap();

        let intent = HomebrewIntent::parse_brewfile(&brewfile_path).unwrap();

        assert_eq!(intent.brews.len(), 2);
        assert!(intent.brews.contains("wget"));
        assert!(intent.brews.contains("curl"));

        assert_eq!(intent.casks.len(), 2);
        assert!(intent.casks.contains("firefox"));
        assert!(intent.casks.contains("visual-studio-code"));

        assert_eq!(intent.taps.len(), 2);
        assert!(intent.taps.contains("homebrew/bundle"));
        assert!(intent.taps.contains("homebrew/core"));
    }

    #[test]
    fn test_extract_quoted_value() {
        assert_eq!(
            HomebrewIntent::extract_quoted_value("brew \"wget\""),
            Some("wget".to_string())
        );
        assert_eq!(
            HomebrewIntent::extract_quoted_value("cask \"visual-studio-code\""),
            Some("visual-studio-code".to_string())
        );
        assert_eq!(HomebrewIntent::extract_quoted_value("no quotes here"), None);
    }

    #[test]
    fn test_extract_from_activation_script() {
        let temp_dir = TempDir::new().unwrap();
        let activate_path = temp_dir.path().join("activate");
        let brewfile_path = temp_dir.path().join("Brewfile");

        // Create a minimal activation script
        let activate_content = format!(
            r#"#!/bin/sh
echo "Setting up Homebrew..."
brew bundle --file='{}' --no-upgrade
echo "Done"
"#,
            brewfile_path.display()
        );

        fs::write(&activate_path, activate_content).unwrap();

        // Create the referenced Brewfile
        let brewfile_content = r#"brew "git""#;
        fs::write(&brewfile_path, brewfile_content).unwrap();

        let intent = HomebrewIntent::extract(temp_dir.path()).unwrap();
        assert!(intent.brews.contains("git"));
    }
}
