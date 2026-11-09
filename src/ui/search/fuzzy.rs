#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match {
    pub score: i64,
    pub byte_positions: Vec<usize>,
}

/// Matches `query` against `candidate` as a case-insensitive subsequence.
///
/// Substrings beat scattered letters, and matches at word starts or at the
/// beginning of the candidate score higher.
pub fn fuzzy_match(query: &str, candidate: &str) -> Option<Match> {
    let query: Vec<char> = query
        .chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect();
    if query.is_empty() {
        return Some(Match {
            score: 0,
            byte_positions: Vec::new(),
        });
    }
    let chars: Vec<(usize, char)> = candidate.char_indices().collect();
    let lower: Vec<char> = chars
        .iter()
        .map(|&(_, c)| c.to_lowercase().next().unwrap_or(c))
        .collect();

    if let Some(start) = find_run(&lower, &query) {
        let word_start = is_boundary(&chars, start);
        let bonus = match (start, word_start) {
            (0, _) => 300,
            (_, true) => 150,
            _ => 0,
        };
        return Some(Match {
            score: 1000 + bonus - (chars.len() - query.len()) as i64,
            byte_positions: (start..start + query.len()).map(|i| chars[i].0).collect(),
        });
    }

    let mut positions = Vec::with_capacity(query.len());
    let mut score = 0;
    let mut next = 0;
    let mut previous: Option<usize> = None;
    for &wanted in &query {
        let first = (next..lower.len()).find(|&i| lower[i] == wanted)?;
        let follows_previous = previous.is_some_and(|p| first == p + 1);
        let index = if follows_previous {
            first
        } else {
            (first..lower.len())
                .find(|&i| lower[i] == wanted && is_boundary(&chars, i))
                .unwrap_or(first)
        };
        score += 10;
        if is_boundary(&chars, index) {
            score += 40;
        }
        score += match previous {
            Some(p) if index == p + 1 => 30,
            Some(p) => -((index - p) as i64),
            None => -(index as i64),
        };
        positions.push(chars[index].0);
        previous = Some(index);
        next = index + 1;
    }
    Some(Match {
        score: score - chars.len() as i64 / 4,
        byte_positions: positions,
    })
}

fn find_run(haystack: &[char], needle: &[char]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn is_boundary(chars: &[(usize, char)], index: usize) -> bool {
    let Some(previous) = index.checked_sub(1).map(|i| chars[i].1) else {
        return true;
    };
    matches!(previous, ' ' | '-' | '_' | '.' | '/')
        || (previous.is_lowercase() && chars[index].1.is_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_query_matches_everything() {
        assert_eq!(
            fuzzy_match("  ", "anything").unwrap().byte_positions,
            Vec::<usize>::new()
        );
    }

    #[test]
    fn substrings_are_highlighted_in_place() {
        assert_eq!(
            fuzzy_match("note", "My notes").unwrap().byte_positions,
            [3, 4, 5, 6]
        );
    }

    #[test]
    fn scattered_letters_prefer_word_starts() {
        assert_eq!(
            fuzzy_match("mn", "My Notes").unwrap().byte_positions,
            [0, 3]
        );
        assert_eq!(
            fuzzy_match("mt", "meetingTodo").unwrap().byte_positions,
            [0, 7]
        );
    }

    #[test]
    fn prefixes_outrank_matches_inside_words() {
        let prefix = fuzzy_match("gro", "Groceries").unwrap().score;
        let inside = fuzzy_match("gro", "Playground").unwrap().score;
        assert!(prefix > inside);
    }

    #[test]
    fn missing_letters_do_not_match() {
        assert!(fuzzy_match("xyz", "My Notes").is_none());
        assert!(fuzzy_match("nm", "My Notes").is_none());
    }

    #[test]
    fn positions_are_byte_offsets() {
        assert_eq!(
            fuzzy_match("ca", "Café notes").unwrap().byte_positions,
            [0, 1]
        );
        assert_eq!(
            fuzzy_match("én", "Café notes").unwrap().byte_positions[0],
            3
        );
    }
}
