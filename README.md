# brewdiff

A Rust crate that provides diff functionality for Homebrew packages managed through nix-darwin. It compares what Homebrew actually has installed on the system versus what a nix-darwin configuration declares should be installed.

## Purpose

This crate is designed to show users what Homebrew changes will occur when they activate a new nix-darwin configuration, similar to how the `dix` crate shows Nix store diffs.

## Features

- Extract Homebrew intent from nix-darwin profiles
- Query current Homebrew state (installed formulae, casks, and taps)
- Compute differences between current state and intended state
- Colorized output with clear add/remove indicators
- Thread-based async processing (mirrors dix pattern)

## Example Output

```
<<< /run/current-system
>>> /nix/var/nix/profiles/system-123-link

ADDED
Formulae
[A] curl
Casks
[A] firefox
[A] visual-studio-code

REMOVED
Formulae
[R] wget
Casks
[R] slack
```

## How It Works

1. Reads the nix-darwin activation script to find the Brewfile path, then parses that Brewfile to extract Homebrew intent
2. Queries Homebrew directly using `brew list` commands for current state
3. Diffs current state with intended state to find additions and removals
4. Formats the diff with colors and emoji indicators

## API

```rust
use brewdiff;
use std::path::Path;

// Primary use case: compare current state with new nix-darwin profile
let new_profile = Path::new("/nix/var/nix/profiles/system-123-link");

// Async diff computation
let handle = brewdiff::spawn_homebrew_diff(new_profile.to_path_buf());
if let Ok(diff_data) = handle.join() {
    let diff = diff_data?;
    println!("Changes: {} additions, {} removals",
             diff.brews.added.len(),
             diff.brews.removed.len());
}

// Or synchronous diff with output
let lines_written = brewdiff::write_homebrew_diffln(
    &mut std::io::stdout(),
    new_profile,
)?;
```
