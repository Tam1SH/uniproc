pub fn get_tab_name_by_index(index: i32) -> &'static str {
    match index {
        0 => "Processes",
        1 => "Performance",
        2 => "Network",
        _ => "Unknown",
    }
}
