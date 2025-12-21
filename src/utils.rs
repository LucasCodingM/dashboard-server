pub fn human_readable_bytes(bytes: u64) -> String {
        const KB: f64 = 1024.0;
        const MB: f64 = KB * 1024.0;
        const GB: f64 = MB * 1024.0;
        
        let b = bytes as f64;
        if b >= GB {
            format!("{:.2} GiB", b / GB)
        } else if b >= MB {
            format!("{:.2} MiB", b / MB)
        } else if b >= KB {
            format!("{:.2} KiB", b / KB)
        } else {
            format!("{} B", bytes)
        }
}