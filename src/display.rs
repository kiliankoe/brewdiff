use crate::diff::HomebrewDiffData;
use crate::error::Result;
use owo_colors::OwoColorize;
use std::io::Write;
use std::path::Path;

/// Write the diff output with header, returns number of lines written
/// Matches dix's format exactly
pub fn write_diff_with_header<W: Write>(
    writer: &mut W,
    current_profile: &Path,
    new_profile: &Path,
    diff_data: &HomebrewDiffData,
) -> Result<usize> {
    let mut lines_written = 0;

    // Header like dix
    writeln!(writer, "<<< {}", current_profile.display())?;
    writeln!(writer, ">>> {}", new_profile.display())?;
    writeln!(writer)?;
    lines_written += 3;

    let inner_lines = write_diff(writer, diff_data)?;
    lines_written += inner_lines;

    Ok(lines_written)
}

/// Write the diff output, returns number of lines written
pub fn write_diff<W: Write>(writer: &mut W, diff_data: &HomebrewDiffData) -> Result<usize> {
    let mut lines_written = 0;

    if !diff_data.has_changes() {
        return Ok(0);
    }

    // Added section
    if !diff_data.brews.added.is_empty()
        || !diff_data.casks.added.is_empty()
        || !diff_data.taps.added.is_empty()
        || !diff_data.mas_apps.added.is_empty()
    {
        writeln!(writer, "ADDED")?;
        lines_written += 1;

        if !diff_data.taps.added.is_empty() {
            writeln!(writer, "Taps")?;
            lines_written += 1;
            for tap in &diff_data.taps.added {
                writeln!(writer, "[{}] {}", "A".green().bold(), tap)?;
                lines_written += 1;
            }
        }

        if !diff_data.brews.added.is_empty() {
            writeln!(writer, "Formulae")?;
            lines_written += 1;
            for pkg in &diff_data.brews.added {
                writeln!(writer, "[{}] {}", "A".green().bold(), pkg)?;
                lines_written += 1;
            }
        }

        if !diff_data.casks.added.is_empty() {
            writeln!(writer, "Casks")?;
            lines_written += 1;
            for pkg in &diff_data.casks.added {
                writeln!(writer, "[{}] {}", "A".green().bold(), pkg)?;
                lines_written += 1;
            }
        }

        if !diff_data.mas_apps.added.is_empty() {
            writeln!(writer, "App Store")?;
            lines_written += 1;
            for app in &diff_data.mas_apps.added {
                writeln!(writer, "[{}] {}", "A".green().bold(), app)?;
                lines_written += 1;
            }
        }

        if !diff_data.brews.removed.is_empty()
            || !diff_data.casks.removed.is_empty()
            || !diff_data.taps.removed.is_empty()
            || !diff_data.mas_apps.removed.is_empty()
        {
            writeln!(writer)?;
            lines_written += 1;
        }
    }

    // Removed section
    if !diff_data.brews.removed.is_empty()
        || !diff_data.casks.removed.is_empty()
        || !diff_data.taps.removed.is_empty()
        || !diff_data.mas_apps.removed.is_empty()
    {
        writeln!(writer, "REMOVED")?;
        lines_written += 1;

        if !diff_data.taps.removed.is_empty() {
            writeln!(writer, "Taps")?;
            lines_written += 1;
            for tap in &diff_data.taps.removed {
                writeln!(writer, "[{}] {}", "R".red().bold(), tap)?;
                lines_written += 1;
            }
        }

        if !diff_data.brews.removed.is_empty() {
            writeln!(writer, "Formulae")?;
            lines_written += 1;
            for pkg in &diff_data.brews.removed {
                writeln!(writer, "[{}] {}", "R".red().bold(), pkg)?;
                lines_written += 1;
            }
        }

        if !diff_data.casks.removed.is_empty() {
            writeln!(writer, "Casks")?;
            lines_written += 1;
            for pkg in &diff_data.casks.removed {
                writeln!(writer, "[{}] {}", "R".red().bold(), pkg)?;
                lines_written += 1;
            }
        }

        if !diff_data.mas_apps.removed.is_empty() {
            writeln!(writer, "App Store")?;
            lines_written += 1;
            for app in &diff_data.mas_apps.removed {
                writeln!(writer, "[{}] {}", "R".red().bold(), app)?;
                lines_written += 1;
            }
        }
    }

    Ok(lines_written)
}

/// Write statistics about the diff (optional, for detailed summaries)
pub fn write_stats<W: Write>(writer: &mut W, diff_data: &HomebrewDiffData) -> Result<()> {
    if !diff_data.has_changes() {
        return Ok(());
    }

    let total_added =
        diff_data.brews.added.len() + diff_data.casks.added.len() + diff_data.taps.added.len();
    let total_removed = diff_data.brews.removed.len()
        + diff_data.casks.removed.len()
        + diff_data.taps.removed.len();

    writeln!(writer)?;
    writeln!(
        writer,
        "HOMEBREW: {} added, {} removed",
        total_added.green(),
        total_removed.red()
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strip_ansi_codes(s: &str) -> String {
        // Simple regex to strip ANSI color codes
        let re = regex::Regex::new(r"\x1b\[[0-9;]*m").unwrap();
        re.replace_all(s, "").to_string()
    }

    #[test]
    fn test_write_diff_no_changes() {
        let diff = HomebrewDiffData::default();
        let mut output = Vec::new();

        let lines = write_diff(&mut output, &diff).unwrap();

        assert_eq!(lines, 0); // No output for no changes
        assert!(output.is_empty());
    }

    #[test]
    fn test_write_diff_with_changes() {
        let mut diff = HomebrewDiffData::default();
        diff.brews.added = vec!["wget".to_string(), "curl".to_string()];
        diff.brews.removed = vec!["git".to_string()];

        let mut output = Vec::new();
        let lines = write_diff(&mut output, &diff).unwrap();

        // ADDED header + Formulae header + 2 brews + blank line + REMOVED header + Formulae header + 1 brew = 8 lines
        assert_eq!(lines, 8);
        let output_str = String::from_utf8(output).unwrap();
        // Strip color codes for testing
        let clean = strip_ansi_codes(&output_str);
        assert!(clean.contains("ADDED"));
        assert!(clean.contains("Formulae"));
        assert!(clean.contains("[A] wget"));
        assert!(clean.contains("[A] curl"));
        assert!(clean.contains("REMOVED"));
        assert!(clean.contains("[R] git"));
    }

    #[test]
    fn test_write_stats() {
        let mut diff = HomebrewDiffData::default();
        diff.brews.added = vec!["wget".to_string()];
        diff.casks.removed = vec!["firefox".to_string()];

        let mut output = Vec::new();
        write_stats(&mut output, &diff).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        let clean_output = strip_ansi_codes(&output_str);
        assert!(clean_output.contains("HOMEBREW: 1 added, 1 removed"));
    }
}
