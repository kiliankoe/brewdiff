use brewdiff;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Brewdiff Example - Comparing Homebrew state with nix-darwin profile\n");

    // Check current system profile
    let current_profile = Path::new("/run/current-system");

    if !current_profile.exists() {
        eprintln!("No nix-darwin system found at /run/current-system");
        eprintln!("This example requires a nix-darwin system with Homebrew configuration");
        return Ok(());
    }

    // Get current Homebrew state
    println!("📊 Current Homebrew State:");
    match brewdiff::get_current_homebrew_state() {
        Ok(state) => {
            println!("  Formulae: {} installed", state.installed_brews.len());
            println!("  Casks: {} installed", state.installed_casks.len());
            println!("  Taps: {} configured", state.installed_taps.len());
        }
        Err(e) => {
            println!("  Error detecting Homebrew state: {}", e);
        }
    }

    // Extract nix-darwin intent
    println!("\n📋 Nix-Darwin Intent:");
    match brewdiff::extract_nix_darwin_intent(current_profile) {
        Ok(intent) => {
            println!("  Formulae: {} declared", intent.brews.len());
            println!("  Casks: {} declared", intent.casks.len());
            println!("  Taps: {} declared", intent.taps.len());
        }
        Err(e) => {
            println!("  Error extracting intent: {}", e);
            return Ok(());
        }
    }

    // Show the diff
    println!("\n🔄 Differences (current vs intended):");
    let lines = brewdiff::write_homebrew_diffln(
        &mut std::io::stdout(),
        // Current system as "old"
        Path::new("/run/current-system"),
        // Same profile as "new" to show drift
        current_profile,
    )?;

    if lines == 3 {
        println!("  Your Homebrew installation matches the nix-darwin configuration!");
    }

    Ok(())
}
