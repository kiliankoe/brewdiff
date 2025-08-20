use crate::diff::HomebrewDiffData;
use crate::error::Result;
use owo_colors::OwoColorize;
use std::io::Write;

/// Write the diff output, returns number of lines written
pub fn write_diff<W: Write>(writer: &mut W, diff_data: &HomebrewDiffData) -> Result<usize> {
    let mut lines_written = 0;

    if !diff_data.has_changes() {
        writeln!(writer, "No Homebrew changes detected")?;
        return Ok(1);
    }

    // Formulae section
    if !diff_data.brews.added.is_empty() || !diff_data.brews.removed.is_empty() {
        writeln!(writer, "\n📦 Homebrew Formulae:")?;
        lines_written += 1;

        for pkg in &diff_data.brews.added {
            writeln!(writer, "  {} {}", "+".green(), pkg)?;
            lines_written += 1;
        }

        for pkg in &diff_data.brews.removed {
            writeln!(writer, "  {} {}", "-".red(), pkg)?;
            lines_written += 1;
        }
    }

    // Casks section
    if !diff_data.casks.added.is_empty() || !diff_data.casks.removed.is_empty() {
        writeln!(writer, "\n🍺 Homebrew Casks:")?;
        lines_written += 1;

        for pkg in &diff_data.casks.added {
            writeln!(writer, "  {} {}", "+".green(), pkg)?;
            lines_written += 1;
        }

        for pkg in &diff_data.casks.removed {
            writeln!(writer, "  {} {}", "-".red(), pkg)?;
            lines_written += 1;
        }
    }

    // Taps section
    if !diff_data.taps.added.is_empty() || !diff_data.taps.removed.is_empty() {
        writeln!(writer, "\n🚰 Homebrew Taps:")?;
        lines_written += 1;

        for tap in &diff_data.taps.added {
            writeln!(writer, "  {} {}", "+".green(), tap)?;
            lines_written += 1;
        }

        for tap in &diff_data.taps.removed {
            writeln!(writer, "  {} {}", "-".red(), tap)?;
            lines_written += 1;
        }
    }

    Ok(lines_written)
}

/// Write statistics about the diff
pub fn write_stats<W: Write>(writer: &mut W, diff_data: &HomebrewDiffData) -> Result<()> {
    if !diff_data.has_changes() {
        return Ok(());
    }

    writeln!(writer, "\nSummary:")?;

    if !diff_data.brews.added.is_empty() || !diff_data.brews.removed.is_empty() {
        writeln!(
            writer,
            "  Formulae: {} added, {} removed",
            diff_data.brews.added.len().green(),
            diff_data.brews.removed.len().red()
        )?;
    }

    if !diff_data.casks.added.is_empty() || !diff_data.casks.removed.is_empty() {
        writeln!(
            writer,
            "  Casks: {} added, {} removed",
            diff_data.casks.added.len().green(),
            diff_data.casks.removed.len().red()
        )?;
    }

    if !diff_data.taps.added.is_empty() || !diff_data.taps.removed.is_empty() {
        writeln!(
            writer,
            "  Taps: {} added, {} removed",
            diff_data.taps.added.len().green(),
            diff_data.taps.removed.len().red()
        )?;
    }

    writeln!(
        writer,
        "  Total changes: {}",
        diff_data.total_changes().yellow()
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
        let output_str = String::from_utf8(output).unwrap();

        assert_eq!(lines, 1);
        assert!(output_str.contains("No Homebrew changes"));
    }

    #[test]
    fn test_write_diff_with_changes() {
        let mut diff = HomebrewDiffData::default();
        diff.brews.added = vec!["wget".to_string(), "curl".to_string()];
        diff.brews.removed = vec!["git".to_string()];

        let mut output = Vec::new();
        let lines = write_diff(&mut output, &diff).unwrap();

        assert_eq!(lines, 4); // Header + 2 additions + 1 removal
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("Homebrew Formulae"));
        assert!(output_str.contains("wget"));
        assert!(output_str.contains("curl"));
        assert!(output_str.contains("git"));
    }

    #[test]
    fn test_write_stats() {
        let mut diff = HomebrewDiffData::default();
        diff.brews.added = vec!["wget".to_string()];
        diff.casks.removed = vec!["firefox".to_string()];

        let mut output = Vec::new();
        write_stats(&mut output, &diff).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        // Remove ANSI color codes for easier testing
        let clean_output = strip_ansi_codes(&output_str);
        assert!(clean_output.contains("Summary"));
        assert!(clean_output.contains("Formulae: 1 added, 0 removed"));
        assert!(clean_output.contains("Casks: 0 added, 1 removed"));
        assert!(clean_output.contains("Total changes: 2"));
    }
}
