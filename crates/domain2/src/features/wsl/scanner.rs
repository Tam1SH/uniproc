use app_contracts2::features::wsl::{AgentPresence, DistroRow};
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn scan_distros() -> anyhow::Result<Vec<DistroRow>> {
    let mut command = Command::new("wsl.exe");
    command.args(["-l", "-v"]);

    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);

    let output = command.output()?;

    let text = decode_utf16le(&output.stdout);

    Ok(parse_wsl_output(&text)
        .into_iter()
        .map(|(name, running)| DistroRow {
            name,
            running,
            agent: AgentPresence::NotChecked,
        })
        .collect())
}

fn decode_utf16le(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    String::from_utf16_lossy(&units)
}

pub fn parse_wsl_output(output: &str) -> Vec<(String, bool)> {
    let clean = output.replace('\0', "");

    clean
        .lines()
        .skip(1)
        .filter_map(|line| {
            let parts: Vec<&str> = line.trim().split_whitespace().collect();

            let (name, state) = match parts.as_slice() {
                ["*", name, state, ..] => (name, state),
                [name, state, ..] => (name, state),
                _ => return None,
            };

            Some((
                (*name).to_string(),
                state.eq_ignore_ascii_case("running"),
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_names_and_running_state() {
        let output = "  NAME      STATE           VERSION\n\
                      * Ubuntu    Running         2\n\
                        Debian    Stopped         2\n";

        assert_eq!(
            parse_wsl_output(output),
            vec![
                ("Ubuntu".to_string(), true),
                ("Debian".to_string(), false),
            ]
        );
    }

    #[test]
    fn the_default_marker_does_not_shift_the_columns() {
        let starred = parse_wsl_output("HEADER\n* Ubuntu Running 2\n");
        let plain = parse_wsl_output("HEADER\nUbuntu Running 2\n");

        assert_eq!(starred, plain);
    }

    #[test]
    fn utf16_padding_and_blank_lines_are_ignored() {
        let output = "N\0A\0M\0E\0\n\0*\0 \0U\0b\0u\0n\0t\0u\0 \0R\0u\0n\0n\0i\0n\0g\0 \02\0\n\n";

        assert_eq!(parse_wsl_output(output), vec![("Ubuntu".to_string(), true)]);
    }

    #[test]
    fn a_message_instead_of_a_table_yields_nothing_usable() {
        let output = "Windows Subsystem for Linux has no installed distributions.\n";

        assert!(parse_wsl_output(output).is_empty());
    }

    #[test]
    fn decodes_what_wsl_actually_writes() {
        let utf16: Vec<u8> = "Ubuntu"
            .encode_utf16()
            .flat_map(|u| u.to_le_bytes())
            .collect();

        assert_eq!(decode_utf16le(&utf16), "Ubuntu");
    }
}
