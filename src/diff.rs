use crate::intent::HomebrewIntent;
use crate::state::HomebrewState;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Default)]
pub struct HomebrewDiffData {
    pub brews: PackageDiff,
    pub casks: PackageDiff,
    pub taps: SetDiff,
    pub mas_apps: SetDiff,
}

#[derive(Debug, Clone, Default)]
pub struct PackageDiff {
    pub added: Vec<String>,
    pub removed: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SetDiff {
    pub added: Vec<String>,
    pub removed: Vec<String>,
}

impl HomebrewDiffData {
    pub fn compute(current_state: &HomebrewState, nix_intent: &HomebrewIntent) -> Self {
        Self {
            brews: Self::compute_package_diff(&current_state.installed_brews, &nix_intent.brews),
            casks: Self::compute_package_diff(&current_state.installed_casks, &nix_intent.casks),
            taps: Self::compute_set_diff(&current_state.installed_taps, &nix_intent.taps),
            mas_apps: Self::compute_set_diff(&current_state.installed_mas_apps, &nix_intent.mas_apps),
        }
    }

    fn compute_package_diff(
        installed: &HashMap<String, String>, // name -> version
        intended: &HashSet<String>,          // just names
    ) -> PackageDiff {
        let mut added = Vec::new();
        let mut removed = Vec::new();

        // Find packages to add
        for pkg in intended {
            if !installed.contains_key(pkg) {
                added.push(pkg.clone());
            }
        }

        // Find packages to remove
        for pkg in installed.keys() {
            if !intended.contains(pkg) {
                removed.push(pkg.clone());
            }
        }

        // Sort for consistent output
        added.sort();
        removed.sort();

        PackageDiff { added, removed }
    }

    fn compute_set_diff(current: &HashSet<String>, intended: &HashSet<String>) -> SetDiff {
        let mut added: Vec<String> = intended.difference(current).cloned().collect();
        let mut removed: Vec<String> = current.difference(intended).cloned().collect();

        added.sort();
        removed.sort();

        SetDiff { added, removed }
    }

    /// Check if there are any changes
    pub fn has_changes(&self) -> bool {
        !self.brews.added.is_empty()
            || !self.brews.removed.is_empty()
            || !self.casks.added.is_empty()
            || !self.casks.removed.is_empty()
            || !self.taps.added.is_empty()
            || !self.taps.removed.is_empty()
            || !self.mas_apps.added.is_empty()
            || !self.mas_apps.removed.is_empty()
    }

    /// Get total count of changes
    pub fn total_changes(&self) -> usize {
        self.brews.added.len()
            + self.brews.removed.len()
            + self.casks.added.len()
            + self.casks.removed.len()
            + self.taps.added.len()
            + self.taps.removed.len()
            + self.mas_apps.added.len()
            + self.mas_apps.removed.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_package_diff_additions() {
        let mut installed = HashMap::new();
        installed.insert("wget".to_string(), "1.21.3".to_string());

        let mut intended = HashSet::new();
        intended.insert("wget".to_string());
        intended.insert("curl".to_string());

        let diff = HomebrewDiffData::compute_package_diff(&installed, &intended);

        assert_eq!(diff.added, vec!["curl"]);
        assert!(diff.removed.is_empty());
    }

    #[test]
    fn test_compute_package_diff_removals() {
        let mut installed = HashMap::new();
        installed.insert("wget".to_string(), "1.21.3".to_string());
        installed.insert("curl".to_string(), "8.4.0".to_string());

        let mut intended = HashSet::new();
        intended.insert("wget".to_string());

        let diff = HomebrewDiffData::compute_package_diff(&installed, &intended);

        assert!(diff.added.is_empty());
        assert_eq!(diff.removed, vec!["curl"]);
    }

    #[test]
    fn test_compute_set_diff() {
        let mut current = HashSet::new();
        current.insert("homebrew/core".to_string());

        let mut intended = HashSet::new();
        intended.insert("homebrew/core".to_string());
        intended.insert("homebrew/cask".to_string());

        let diff = HomebrewDiffData::compute_set_diff(&current, &intended);

        assert_eq!(diff.added, vec!["homebrew/cask"]);
        assert!(diff.removed.is_empty());
    }

    #[test]
    fn test_has_changes() {
        let state = HomebrewState::default();
        let intent = HomebrewIntent::default();
        let diff = HomebrewDiffData::compute(&state, &intent);
        assert!(!diff.has_changes());

        let mut intent_with_brew = HomebrewIntent::default();
        intent_with_brew.brews.insert("git".to_string());
        let diff_with_changes = HomebrewDiffData::compute(&state, &intent_with_brew);
        assert!(diff_with_changes.has_changes());
    }
}
