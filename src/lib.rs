pub mod diff;
pub mod display;
pub mod error;
pub mod intent;
pub mod state;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::thread::{self, JoinHandle};

pub use diff::{HomebrewDiffData, PackageDiff};
pub use error::{Error, Result};
pub use intent::HomebrewIntent;
pub use state::HomebrewState;

/// Primary API - compare current Homebrew state with new nix-darwin config
/// Mirrors dix's spawn pattern for async processing
pub fn spawn_homebrew_diff(new_profile: PathBuf) -> JoinHandle<Result<HomebrewDiffData>> {
    thread::spawn(move || {
        let current_state = HomebrewState::detect()?;
        let nix_intent = HomebrewIntent::extract(&new_profile)?;
        Ok(HomebrewDiffData::compute(&current_state, &nix_intent))
    })
}

/// Write homebrew diff output, returns number of lines written
/// Mirrors dix's write pattern
pub fn write_homebrew_diffln<W: Write>(writer: &mut W, new_profile: &Path) -> Result<usize> {
    let current_state = HomebrewState::detect()?;
    let nix_intent = HomebrewIntent::extract(new_profile)?;
    let diff_data = HomebrewDiffData::compute(&current_state, &nix_intent);

    display::write_diff(writer, &diff_data)
}

/// Write homebrew diff statistics
pub fn write_homebrew_stats<W: Write>(writer: &mut W, diff_data: &HomebrewDiffData) -> Result<()> {
    display::write_stats(writer, diff_data)
}

/// Get current Homebrew state
pub fn get_current_homebrew_state() -> Result<HomebrewState> {
    HomebrewState::detect()
}

/// Extract nix-darwin intent from a built profile
pub fn extract_nix_darwin_intent(profile: &Path) -> Result<HomebrewIntent> {
    HomebrewIntent::extract(profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_public_api_exists() {
        // Just verify the public API compiles
        let _ = get_current_homebrew_state;
        let _ = extract_nix_darwin_intent;
        let _ = spawn_homebrew_diff;
        let _ = write_homebrew_diffln::<Vec<u8>>;
        let _ = write_homebrew_stats::<Vec<u8>>;
    }
}
