use std::path::Path;
use std::time::{Duration, SystemTime};

pub fn relative_time(time: SystemTime) -> String {
    let seconds = SystemTime::now()
        .duration_since(time)
        .unwrap_or(Duration::ZERO)
        .as_secs();
    match seconds {
        0..60 => "just now".into(),
        60..3_600 => format!("{}m ago", seconds / 60),
        3_600..86_400 => format!("{}h ago", seconds / 3_600),
        86_400..2_592_000 => format!("{}d ago", seconds / 86_400),
        _ => format!("{}mo ago", seconds / 2_592_000),
    }
}

pub fn display_path(path: &Path) -> String {
    let home = dirs::home_dir().unwrap_or_default();
    match path.strip_prefix(&home) {
        Ok(rest) if !home.as_os_str().is_empty() => Path::new("~").join(rest).display().to_string(),
        _ => path.display().to_string(),
    }
}

pub fn word_count<'a>(chunks: impl IntoIterator<Item = &'a str>) -> usize {
    let mut count = 0;
    let mut in_word = false;
    for c in chunks.into_iter().flat_map(str::chars) {
        let word_char = !c.is_whitespace();
        if word_char && !in_word {
            count += 1;
        }
        in_word = word_char;
    }
    count
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use super::{display_path, relative_time, word_count};

    #[test]
    fn paths_under_home_are_shortened() {
        assert_eq!(
            display_path(
                &dirs::home_dir()
                    .unwrap_or_default()
                    .join("Documents/Noties")
            ),
            "~/Documents/Noties"
        );
        assert_eq!(
            display_path(std::path::Path::new("/srv/notes")),
            "/srv/notes"
        );
    }

    #[test]
    fn words_are_whitespace_separated() {
        assert_eq!(word_count(["# Title\n\nsome *notes* here"]), 5);
        assert_eq!(word_count(["   "]), 0);
    }

    #[test]
    fn words_split_across_chunks_count_once() {
        assert_eq!(word_count(["hel", "lo wo", "rld", " "]), 2);
        assert_eq!(word_count(["one ", " two"]), 2);
    }

    #[test]
    fn relative_times_round_down_to_the_largest_unit() {
        let now = SystemTime::now();
        assert_eq!(relative_time(now), "just now");
        assert_eq!(relative_time(now - Duration::from_secs(5 * 60)), "5m ago");
        assert_eq!(
            relative_time(now - Duration::from_secs(3 * 3_600)),
            "3h ago"
        );
        assert_eq!(
            relative_time(now - Duration::from_secs(2 * 86_400)),
            "2d ago"
        );
    }
}
