use colored::*;

pub fn format_bytes(bytes: usize) -> ColoredString {
    let bytes = bytes as f64;
    let kilobytes = bytes / 1024f64;
    let megabytes = kilobytes / 1024f64;
    let gigabytes = megabytes / 1024f64;
    let terabytes = gigabytes / 1024f64;

    if terabytes >= 1f64 {
        format!("{:6.2} TB", terabytes)
            .bold()
            .bright_white()
            .on_red()
    } else if gigabytes >= 5f64 {
        format!("{:6.2} GB", gigabytes)
            .bold()
            .bright_white()
            .on_red()
    } else if gigabytes >= 2f64 {
        format!("{:6.2} GB", gigabytes).bright_red()
    } else if gigabytes >= 1f64 {
        format!("{:6.2} GB", gigabytes).yellow()
    } else if megabytes >= 200f64 {
        format!("{:6.1} MB", megabytes).yellow()
    } else if megabytes >= 1f64 {
        format!("{:6.1} MB", megabytes).normal()
    } else if kilobytes >= 1f64 {
        format!("{:6.1} KB", kilobytes).normal()
    } else if bytes >= 1f64 {
        format!("{:6.1} B", bytes).normal()
    } else {
        format!("{:>6}", "-").normal()
    }
}

pub fn format_seconds(sec: f64) -> String {
    let sec = sec as usize;
    let minutes = sec / 60_usize;
    let hours = minutes / 60_usize;
    let days = hours / 24_usize;

    if days >= 1 {
        format!(
            "{:.0}d {:.0}h:{:.0}m.{:.0}s",
            days,
            hours % 24,
            minutes % 60,
            sec % 60
        )
    } else if hours >= 1 {
        format!("{:.0}h:{:.0}m.{:.0}s", hours, minutes % 60, sec % 60)
    } else if minutes >= 1 {
        format!("{:.0}m.{:.1}s", minutes, sec % 60)
    } else {
        format!("{:.2}s", sec)
    }
}
