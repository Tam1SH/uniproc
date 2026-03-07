pub fn parse_wsl_output(output: &str) -> Vec<(String, bool)> {
    let clean_output = output.replace('\0', "");

    clean_output
        .lines()
        .skip(1)
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }

            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.is_empty() {
                return None;
            }

            if parts[0] == "*" {
                if parts.len() >= 3 {
                    Some((
                        parts[1].to_string(),
                        parts[2].to_ascii_lowercase().contains("running"),
                    ))
                } else {
                    None
                }
            } else if parts.len() >= 2 {
                Some((
                    parts[0].to_string(),
                    parts[1].to_ascii_lowercase().contains("running"),
                ))
            } else {
                None
            }
        })
        .collect()
}
