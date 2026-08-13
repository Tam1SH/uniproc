pub fn bytes(v: u64) -> String {
    const KIB: f64 = 1024.0;
    let f = v as f64;
    if f >= KIB.powi(3) {
        format!("{:.1} GiB", f / KIB.powi(3))
    } else if f >= KIB.powi(2) {
        format!("{:.1} MiB", f / KIB.powi(2))
    } else if f >= KIB {
        format!("{:.0} KiB", f / KIB)
    } else {
        format!("{v} B")
    }
}
