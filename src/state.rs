use crate::error::{Error, Result};
use std::collections::{HashMap, HashSet};
use std::process::Command;

/// What's actually installed via Homebrew right now
#[derive(Debug, Clone, Default)]
pub struct HomebrewState {
    pub installed_brews: HashMap<String, String>, // name -> version
    pub installed_casks: HashMap<String, String>, // name -> version
    pub installed_taps: HashSet<String>,
    pub installed_mas_apps: HashSet<String>, // Store as "name (id)" for display
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
            installed_mas_apps: Self::get_mas_apps()?,
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
        // Use 'brew leaves' to get only user-installed formulae (not dependencies)
        // This avoids showing confusing removals for dependencies like pcre2 that
        // are only installed because they're required by other formulae.
        // Users typically only care about the top-level packages they explicitly installed.
        let leaves_output = Command::new(Self::get_brew_command())
            .args(["leaves"])
            .output()
            .map_err(|e| Error::CommandFailed(format!("brew leaves failed: {}", e)))?;

        if !leaves_output.status.success() {
            return Ok(HashMap::new());
        }

        let leaves_str = String::from_utf8(leaves_output.stdout)?;
        let leaves: Vec<String> = leaves_str.lines().map(|s| s.to_string()).collect();

        if leaves.is_empty() {
            return Ok(HashMap::new());
        }

        // Get versions for the leaves
        let mut args = vec!["list", "--versions"];
        for leaf in &leaves {
            args.push(leaf);
        }

        let versions_output = Command::new(Self::get_brew_command())
            .args(&args)
            .output()
            .map_err(|e| Error::CommandFailed(format!("brew list --versions failed: {}", e)))?;

        if !versions_output.status.success() {
            return Ok(HashMap::new());
        }

        Self::parse_list_versions_output(&versions_output.stdout)
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

    fn get_mas_apps() -> Result<HashSet<String>> {
        // Check if mas is installed
        let mas_check = Command::new("which")
            .arg("mas")
            .output()
            .map_err(|e| Error::CommandFailed(format!("which mas failed: {}", e)))?;

        if !mas_check.status.success() {
            // mas not installed, no MAS apps
            return Ok(HashSet::new());
        }

        let output = Command::new("mas")
            .arg("list")
            .output()
            .map_err(|e| Error::CommandFailed(format!("mas list failed: {}", e)))?;

        if !output.status.success() {
            return Ok(HashSet::new());
        }

        let content = String::from_utf8(output.stdout)?;
        let mut apps = HashSet::new();

        // Parse output format: "1234567890  App Name     (1.2.3)"
        for line in content.lines() {
            // Split on whitespace and filter out empty strings
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let id = parts[0];
                // Find where the version starts (last item in parentheses)
                let version_start = parts.iter().rposition(|p| p.starts_with('('));
                let name_parts = if let Some(idx) = version_start {
                    &parts[1..idx]
                } else {
                    &parts[1..]
                };
                let name = name_parts.join(" ");
                // Store as "App Name (id)" to match intent format
                apps.insert(format!("{} ({})", name, id));
            }
        }

        Ok(apps)
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
