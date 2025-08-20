# brewdiff - Homebrew Diff Crate Design Document

## Executive Summary

`brewdiff` is a Rust crate that provides diff functionality for Homebrew packages managed through nix-darwin. It compares what Homebrew actually has installed on the system versus what a nix-darwin configuration declares should be installed, showing users what changes will occur when they activate a new configuration.

## Motivation

- **User Clarity**: Show users what Homebrew changes will happen before activating a nix-darwin configuration
- **Integration with NH**: Provide Homebrew diff alongside the existing Nix store diff from `dix`
- **Safety**: Let users see if packages will be removed or upgraded before applying changes
- **Separation of Concerns**: Keep Homebrew-specific logic in a dedicated crate

## Interface Design

The core API focuses on comparing current Homebrew state with nix-darwin intent:

```rust
// Primary API - compare current Homebrew state with new nix-darwin config
pub fn spawn_homebrew_diff(
    new_profile: PathBuf  // The new nix-darwin profile to be activated
) -> JoinHandle<Result<HomebrewDiffData>>;

pub fn write_homebrew_diffln<W: Write>(
    writer: &mut W,
    new_profile: &Path,
) -> Result<usize>;

pub fn write_homebrew_stats<W: Write>(
    writer: &mut W,
    diff_data: &HomebrewDiffData,
) -> Result<()>;

// Core extraction functions
pub fn get_current_homebrew_state() -> Result<HomebrewState>;
pub fn extract_nix_darwin_intent(profile: &Path) -> Result<HomebrewIntent>;

// Alternative API for comparing two profiles (less common use case)
pub fn diff_profiles(
    old_profile: &Path,
    new_profile: &Path,
) -> Result<HomebrewDiffData>;
```

## Core Data Structures

```rust
// What's actually installed via Homebrew right now
#[derive(Debug, Clone)]
pub struct HomebrewState {
    pub installed_brews: HashMap<String, String>, // name -> version
    pub installed_casks: HashMap<String, String>, // name -> version
    pub installed_taps: HashSet<String>,
    pub installed_mas_apps: HashMap<String, u64>, // name -> app_id
}

// What nix-darwin wants to be installed
#[derive(Debug, Clone, PartialEq)]
pub struct HomebrewIntent {
    pub brews: HashSet<String>,
    pub casks: HashSet<String>,
    pub taps: HashSet<String>,
    pub mas_apps: HashMap<String, u64>, // name -> app_id
    pub cleanup_on_activation: bool,
    pub upgrade_on_activation: bool,
}

#[derive(Debug)]
pub struct HomebrewDiffData {
    pub brews: PackageDiff,
    pub casks: PackageDiff,
    pub taps: SetDiff,
    pub mas_apps: MapDiff,
    pub config_changes: ConfigChanges,
}

#[derive(Debug)]
pub struct PackageDiff {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub upgraded: Vec<(String, String, String)>, // (name, old_version, new_version)
}

#[derive(Debug)]
pub struct SetDiff {
    pub added: Vec<String>,
    pub removed: Vec<String>,
}

#[derive(Debug)]
pub struct MapDiff {
    pub added: Vec<(String, u64)>,
    pub removed: Vec<(String, u64)>,
}

#[derive(Debug)]
pub struct ConfigChanges {
    pub cleanup_changed: Option<(bool, bool)>,
    pub upgrade_changed: Option<(bool, bool)>,
}
```

## Implementation Strategy

### Phase 1: Get Current Homebrew State

This is straightforward - query Homebrew directly for what's installed:

```rust
impl HomebrewState {
    pub fn detect() -> Result<Self> {
        if !homebrew_installed() {
            return Ok(Self::empty());
        }
        
        Ok(Self {
            installed_brews: Self::get_installed_formulae()?,
            installed_casks: Self::get_installed_casks()?,
            installed_taps: Self::get_taps()?,
            installed_mas_apps: Self::get_mas_apps()?,
        })
    }
    
    fn get_installed_formulae() -> Result<HashMap<String, String>> {
        // Option 1: Simple list with versions
        let output = Command::new("brew")
            .args(["list", "--formula", "--versions"])
            .output()?;
        parse_brew_list_versions(&output.stdout)
    }
    
    fn get_installed_formulae_json() -> Result<HashMap<String, String>> {
        // Option 2: Detailed JSON (slower but more info)
        let output = Command::new("brew")
            .args(["info", "--installed", "--json=v2"])
            .output()?;
        parse_brew_json(&output.stdout)
    }
}
```

### Phase 2: Extract Intent from Nix-Darwin Profile

**DISCOVERED: How nix-darwin actually manages Homebrew!**

Based on investigation of a real nix-darwin system, here's what happens:

1. **Nix-darwin generates a Brewfile** in the nix store
2. **The activation script runs `brew bundle`** with that Brewfile
3. **The Brewfile path is embedded in the activation script**

Here's the actual implementation:

```rust
impl HomebrewIntent {
    pub fn extract(profile: &Path) -> Result<Self> {
        // The activation script contains a brew bundle command with the Brewfile path
        Self::extract_brewfile_from_activation_script(profile)
    }
    
    fn extract_brewfile_from_activation_script(profile: &Path) -> Result<Self> {
        let activate_path = profile.join("activate");
        if !activate_path.exists() {
            return Err(Error::NoActivationScript);
        }
        
        let content = std::fs::read_to_string(activate_path)?;
        
        // Look for the brew bundle command
        // Example from real system:
        // brew bundle --file='/nix/store/amsfmcij84r55d8m1zyyywqlia67x3z5-Brewfile' --no-upgrade
        
        let brewfile_regex = regex::Regex::new(
            r"brew bundle --file='(/nix/store/[^']+Brewfile)'"
        )?;
        
        if let Some(captures) = brewfile_regex.captures(&content) {
            let brewfile_path = captures.get(1).unwrap().as_str();
            return parse_brewfile(Path::new(brewfile_path));
        }
        
        Err(Error::BrewfileNotFound)
    }
}

// The generated Brewfile has a simple format:
// # Created by `nix-darwin`'s `homebrew` module
// tap "homebrew/bundle"
// brew "gh"
// cask "firefox"
// mas "Xcode", id: 497799835

fn parse_brewfile(path: &Path) -> Result<HomebrewIntent> {
    let content = std::fs::read_to_string(path)?;
    let mut intent = HomebrewIntent::default();
    
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        
        if line.starts_with("brew \"") {
            let formula = extract_quoted_value(line)?;
            intent.brews.insert(formula);
        } else if line.starts_with("cask \"") {
            let cask = extract_quoted_value(line)?;
            intent.casks.insert(cask);
        } else if line.starts_with("tap \"") {
            let tap = extract_quoted_value(line)?;
            intent.taps.insert(tap);
        } else if line.starts_with("mas \"") {
            // Note: mas lines aren't present in the example system
            // but the format would be: mas "App Name", id: 1234567890
            let (name, id) = parse_mas_line(line)?;
            intent.mas_apps.insert(name, id);
        }
    }
    
    Ok(intent)
}
```

### Phase 3: Diff Computation

```rust
impl HomebrewDiffData {
    pub fn compute(
        current_state: &HomebrewState,
        nix_intent: &HomebrewIntent,
    ) -> Self {
        let brews = Self::compute_brew_diff(
            &current_state.installed_brews,
            &nix_intent.brews,
        );
        
        let casks = Self::compute_cask_diff(
            &current_state.installed_casks,
            &nix_intent.casks,
        );
        
        let taps = Self::compute_tap_diff(
            &current_state.installed_taps,
            &nix_intent.taps,
        );
        
        let mas_apps = Self::compute_mas_diff(
            &current_state.installed_mas_apps,
            &nix_intent.mas_apps,
        );
        
        Self {
            brews,
            casks,
            taps,
            mas_apps,
            config_changes: ConfigChanges {
                cleanup_changed: if nix_intent.cleanup_on_activation {
                    Some((false, true))
                } else {
                    None
                },
                upgrade_changed: if nix_intent.upgrade_on_activation {
                    Some((false, true))
                } else {
                    None
                },
            },
        }
    }
    
    fn compute_brew_diff(
        installed: &HashMap<String, String>,  // name -> version
        intended: &HashSet<String>,           // just names
    ) -> PackageDiff {
        let mut added = Vec::new();
        let mut removed = Vec::new();
        let mut upgraded = Vec::new();
        
        // Find packages to add
        for pkg in intended {
            if !installed.contains_key(pkg) {
                added.push(pkg.clone());
            }
        }
        
        // Find packages to remove (if cleanup is enabled)
        for (pkg, version) in installed {
            if !intended.contains(pkg) {
                removed.push(format!("{} ({})", pkg, version));
            }
        }
        
        // Note: Version upgrades would happen automatically
        // if upgrade_on_activation is true
        
        PackageDiff { added, removed, upgraded }
    }
}
```


### Phase 4: Output Formatting

```rust
pub fn write_homebrew_diffln<W: Write>(
    writer: &mut W,
    old_profile: &Path,
    new_profile: &Path,
) -> Result<usize> {
    let mut lines_written = 0;
    
    // Header (similar to dix)
    writeln!(writer, "<<< {}", old_profile.display())?;
    writeln!(writer, ">>> {}", new_profile.display())?;
    lines_written += 2;
    
    // Get diff data (from spawned thread or compute directly)
    let diff_data = compute_diff(old_profile, new_profile)?;
    
    // Format output with colors
    if !diff_data.brews.added.is_empty() {
        writeln!(writer, "\n📦 Homebrew Formulae:")?;
        for pkg in &diff_data.brews.added {
            writeln!(writer, "  {} {}", green("+"), pkg)?;
        }
        lines_written += diff_data.brews.added.len() + 1;
    }
    
    if !diff_data.brews.removed.is_empty() {
        for pkg in &diff_data.brews.removed {
            writeln!(writer, "  {} {}", red("-"), pkg)?;
        }
        lines_written += diff_data.brews.removed.len();
    }
    
    // Similar for casks, taps, etc.
    
    Ok(lines_written)
}
```

## Integration with NH

NH would use brewdiff when switching/building darwin configurations:

In NH's `src/darwin.rs`:

```rust
// After the existing dix diff
if matches!(self.common.diff, DiffType::Never) {
    debug!("Not running dix as the --diff flag is set to never.");
} else {
    debug!("Comparing with target profile: {}", target_profile.display());
    let _ = print_dix_diff(&PathBuf::from(CURRENT_PROFILE), &target_profile);
    
    // NEW: Add Homebrew diff
    // Compare current Homebrew state with what the new profile wants
    if homebrew_managed_by_nix() {
        let _ = print_homebrew_diff(&target_profile);
    }
}
```

In NH's `src/util.rs`:

```rust
pub fn print_homebrew_diff(new_profile: &Path) -> Result<()> {
    let mut out = WriteFmt(io::stdout());
    
    // Spawn thread for async processing (like dix)
    let homebrew_handle = brewdiff::spawn_homebrew_diff(
        new_profile.to_path_buf()
    );
    
    // Write the diff header
    writeln!(&mut out, "Homebrew changes:")?;
    
    // Write the diff
    let wrote = brewdiff::write_homebrew_diffln(
        &mut out, 
        new_profile
    ).unwrap_or_default();
    
    // Handle stats if available
    if let Ok(diff_data) = homebrew_handle.join()? {
        if wrote > 0 {
            println!();
        }
        brewdiff::write_homebrew_stats(&mut out, &diff_data)?;
    }
    
    Ok(())
}
```

The key difference from dix is that we're comparing:
- **Current state**: What Homebrew actually has installed
- **New intent**: What the new nix-darwin profile wants installed

Rather than comparing two nix profiles like dix does.

## Error Handling

The crate should handle these scenarios gracefully:

1. **Homebrew not installed**: Return empty state, no diff
2. **Configuration not found**: Assume no Homebrew management
3. **Parse errors**: Log and continue with partial data
4. **Command failures**: Graceful degradation

```rust
#[derive(Debug, thiserror::Error)]
pub enum BrewdiffError {
    #[error("Homebrew not installed")]
    HomebrewNotFound,
    
    #[error("Failed to extract configuration: {0}")]
    ConfigExtraction(String),
    
    #[error("Failed to parse Homebrew output: {0}")]
    ParseError(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_brew_list() {
        let input = "wget 2.1.2\ncurl 7.68.0\n";
        let result = parse_brew_list_output(input.as_bytes()).unwrap();
        assert_eq!(result.get("wget"), Some(&"2.1.2".to_string()));
    }
    
    #[test]
    fn test_compute_diff() {
        let old = HomebrewConfig {
            brews: ["wget", "curl"].into_iter().map(String::from).collect(),
            ..Default::default()
        };
        let new = HomebrewConfig {
            brews: ["curl", "git"].into_iter().map(String::from).collect(),
            ..Default::default()
        };
        
        let diff = HomebrewDiffData::compute(&old, &new, None);
        assert_eq!(diff.brews.removed, vec!["wget"]);
        assert_eq!(diff.brews.added, vec!["git"]);
    }
}
```

### Integration Tests

```rust
#[test]
#[cfg(target_os = "macos")]
fn test_real_homebrew_detection() {
    // Only run if Homebrew is installed
    if homebrew_installed() {
        let state = HomebrewState::detect().unwrap();
        assert!(!state.installed_brews.is_empty() || !state.installed_casks.is_empty());
    }
}
```

## Performance Considerations

1. **Async Processing**: Use threads like `dix` for expensive operations
2. **Caching**: Cache Homebrew state queries (they're slow)
3. **Lazy Evaluation**: Only query what's needed
4. **Batch Operations**: Group Homebrew commands when possible

## Project Structure

```
brewdiff/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs           # Public API
│   ├── config.rs        # Configuration extraction
│   ├── state.rs         # Current state detection
│   ├── diff.rs          # Diff computation
│   ├── display.rs       # Output formatting
│   ├── error.rs         # Error types
│   └── homebrew.rs      # Homebrew command interface
└── tests/
    ├── fixtures/        # Test data
    └── integration.rs   # Integration tests
```

## Cargo.toml

```toml
[package]
name = "brewdiff"
version = "0.1.0"
edition = "2021"
description = "Homebrew diff functionality for nix-darwin configurations"
license = "MIT OR Apache-2.0"

[dependencies]
thiserror = "1.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
regex = "1.8"
owo-colors = "4.0"
# For parsing command output
nom = "7.1"  # Optional: for robust parsing of brew output
# For async operations (matching dix pattern)
tokio = { version = "1.0", features = ["rt", "process"] }  # Optional: if using async

[dev-dependencies]
tempfile = "3.5"
mockall = "0.11"
pretty_assertions = "1.4"  # For better test output
insta = "1.31"  # For snapshot testing of diff outputs
```

## Usage Example

```rust
use brewdiff;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Primary use case: compare current state with new nix-darwin profile
    let new_profile = Path::new("/tmp/new-darwin-system");
    
    // Get current Homebrew state
    let current_state = brewdiff::get_current_homebrew_state()?;
    println!("Currently installed: {} formulae, {} casks", 
             current_state.installed_brews.len(),
             current_state.installed_casks.len());
    
    // Extract what nix-darwin wants
    let nix_intent = brewdiff::extract_nix_darwin_intent(new_profile)?;
    println!("Nix-darwin wants: {} formulae, {} casks",
             nix_intent.brews.len(),
             nix_intent.casks.len());
    
    // Simple synchronous diff
    let diff_count = brewdiff::write_homebrew_diffln(
        &mut std::io::stdout(),
        new_profile,
    )?;
    
    println!("Displayed {} diff lines", diff_count);
    
    // Or async with stats
    let handle = brewdiff::spawn_homebrew_diff(
        new_profile.to_path_buf(),
    );
    
    if let Ok(diff_data) = handle.join() {
        brewdiff::write_homebrew_stats(&mut std::io::stdout(), &diff_data?)?;
    }
    
    Ok(())
}
```

## Development Roadmap

### Phase 1: MVP (Week 1-2)
- Basic configuration extraction from activation scripts
- Simple diff computation (added/removed only)
- Text output without colors

### Phase 2: Enhanced (Week 3-4)
- Version change detection
- Colored output
- Threaded processing
- Better error handling

### Phase 3: Complete (Week 5-6)
- Full nix-darwin integration
- Comprehensive tests
- Documentation
- Performance optimization

## Comparison Strategies

Based on the research, we have several approaches for comparing Homebrew installations:

### 1. Brewfile-based Comparison
- Generate or extract Brewfiles from both profiles
- Use text diff to compare the declarative configuration
- Pros: Simple, human-readable, version-control friendly
- Cons: May not capture version differences unless pinned

### 2. JSON Export Comparison
- Use `brew info --json=v2` for detailed package information
- Enables semantic diffing with version tracking
- Can use tools like `jd` (JSON diff) for structured comparison
- Pros: Rich metadata, version information, dependency tracking
- Cons: More complex parsing, larger data structures

### 3. Hybrid Approach (Recommended)
- Use Brewfile for configuration intent (what should be installed)
- Use JSON export for actual state (what is installed, with versions)
- Combine both for comprehensive diff output

```rust
pub enum DiffStrategy {
    Simple,     // Just package names (add/remove)
    Versions,   // Include version changes
    Full,       // Include all metadata changes
}
```

## Research Findings

Based on investigation of a real nix-darwin system with Homebrew configuration:

### 1. How nix-darwin manages Homebrew

**Discovered:**
- When you specify `homebrew.brews = ["wget" "curl"];` in nix-darwin, it generates a Brewfile
- The Brewfile is stored in the nix store at `/nix/store/[hash]-Brewfile`
- The activation script contains a `brew bundle` command that references this Brewfile
- The exact command is: `brew bundle --file='/nix/store/[hash]-Brewfile' --no-upgrade`

**File locations:**
- Activation script: `/nix/var/nix/profiles/system-XX-link/activate`
- Brewfile: `/nix/store/[hash]-Brewfile` (path embedded in activation script)

### 2. Brewfile Format

The generated Brewfile has a clean, simple format:
```brewfile
# Created by `nix-darwin`'s `homebrew` module

# Taps
tap "homebrew/bundle"
tap "homebrew/cask"

# Brews
brew "gh"
brew "swift-outdated"

# Casks
cask "firefox"
cask "visual-studio-code"

# Mac App Store apps (if configured)
# mas "Xcode", id: 497799835
```

### 3. Extraction Approach

The most reliable approach is:
1. Read the activation script at `profile/activate`
2. Extract the Brewfile path using regex: `brew bundle --file='(/nix/store/[^']+Brewfile)'`
3. Read and parse the Brewfile at that path

This approach works because:
- The activation script always exists for a built profile
- The brew bundle command format is consistent
- The Brewfile in the nix store is immutable and accessible

## Remaining Questions

1. **Cleanup behavior**: Does nix-darwin's homebrew module support cleanup options? 
   - Need to check if `onActivation.cleanup` is exposed in the activation script
2. **Output Format**: Should we match dix's format exactly or have our own style?
3. **Crate Name**: `brewdiff`, `hops`, `homebrew-diff`, or something else?
4. **Version upgrades**: The `--no-upgrade` flag means versions won't change automatically
   - Should we detect available upgrades with `brew outdated`?

## Conclusion

Creating a separate `brewdiff` crate follows the successful pattern established by `dix` and provides a clean, reusable solution for Homebrew diff functionality. The crate would be:

- **Focused**: Single responsibility - Homebrew diffs
- **Reusable**: Other tools can use it
- **Testable**: Easier to test in isolation
- **Maintainable**: Clear boundaries and interfaces

This design provides a solid foundation for implementing Homebrew diff support in NH while maintaining good software engineering practices.