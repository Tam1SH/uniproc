pub fn get_tab_name_by_index(index: i32) -> &'static str {
    match index {
        0 => "Processes",
        1 => "Performance",
        2 => "Disk",
        3 => "Statistics",
        4 => "Startup apps",
        5 => "Users",
        6 => "Services",
        _ => "Unknown",
    }
}
