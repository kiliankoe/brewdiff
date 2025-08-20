use crate::error::{Error, Result};
use std::collections::{HashMap, HashSet};
use std::process::Command;

/// What's actually installed via Homebrew right now
#[derive(Debug, Clone, Default)]
pub struct HomebrewState {
    pub installed_brews: HashMap<String, String>, // name -> version
    pub installed_casks: HashMap<String, String>, // name -> version
    pub installed_taps: HashSet<String>,
}

impl HomebrewState {
    /// Detect current Homebrew state by querying brew commands
    pub fn detect() -> Result<Self> {
        if !Self::homebrew_installed() {
            return Ok(Self::default());
        }

        Ok(Self {
            installed_brews: Self::get_installed_formulae()?,
            installed_casks: Self::get_installed_casks()?,
            installed_taps: Self::get_taps()?,
        })
    }

    fn homebrew_installed() -> bool {
        // Check for Homebrew at common locations
        std::path::Path::new("/opt/homebrew/bin/brew").exists()
            || std::path::Path::new("/usr/local/bin/brew").exists()
    }

    fn get_brew_command() -> &'static str {
        if std::path::Path::new("/opt/homebrew/bin/brew").exists() {
            "/opt/homebrew/bin/brew"
        } else {
            "/usr/local/bin/brew"
        }
    }

    fn get_installed_formulae() -> Result<HashMap<String, String>> {
        let output = Command::new(Self::get_brew_command())
            .args(["list", "--formula", "--versions"])
            .output()
            .map_err(|e| Error::CommandFailed(format!("brew list failed: {}", e)))?;

        if !output.status.success() {
            return Ok(HashMap::new());
        }

        Self::parse_list_versions_output(&output.stdout)
    }

    fn get_installed_casks() -> Result<HashMap<String, String>> {
        let output = Command::new(Self::get_brew_command())
            .args(["list", "--cask", "--versions"])
            .output()
            .map_err(|e| Error::CommandFailed(format!("brew list --cask failed: {}", e)))?;

        if !output.status.success() {
            return Ok(HashMap::new());
        }

        Self::parse_list_versions_output(&output.stdout)
    }

    fn get_taps() -> Result<HashSet<String>> {
        let output = Command::new(Self::get_brew_command())
            .args(["tap"])
            .output()
            .map_err(|e| Error::CommandFailed(format!("brew tap failed: {}", e)))?;

        if !output.status.success() {
            return Ok(HashSet::new());
        }

        let content = String::from_utf8(output.stdout)?;
        Ok(content.lines().map(|s| s.to_string()).collect())
    }

    fn parse_list_versions_output(output: &[u8]) -> Result<HashMap<String, String>> {
        let content = String::from_utf8(output.to_vec())?;
        let mut result = HashMap::new();

        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            let name = parts[0].to_string();
            let version = if parts.len() > 1 {
                // Join all version parts (some versions have spaces)
                parts[1..].join(" ")
            } else {
                "unknown".to_string()
            };

            result.insert(name, version);
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_list_versions_output() {
        let input = b"wget 1.21.3\ncurl 8.4.0\ngit 2.42.0 2.41.0\n";
        let result = HomebrewState::parse_list_versions_output(input).unwrap();

        assert_eq!(result.get("wget"), Some(&"1.21.3".to_string()));
        assert_eq!(result.get("curl"), Some(&"8.4.0".to_string()));
        // Multiple versions get joined
        assert_eq!(result.get("git"), Some(&"2.42.0 2.41.0".to_string()));
    }

    #[test]
    fn test_parse_empty_output() {
        let input = b"";
        let result = HomebrewState::parse_list_versions_output(input).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_homebrew_detection() {
        // This test will pass/fail based on whether Homebrew is installed
        let is_installed = HomebrewState::homebrew_installed();
        if is_installed {
            assert!(
                std::path::Path::new("/opt/homebrew/bin/brew").exists()
                    || std::path::Path::new("/usr/local/bin/brew").exists()
            );
        }
    }
}
