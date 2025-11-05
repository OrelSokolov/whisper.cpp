use crate::types::Segment;

/// Merges segments that form incomplete sentences
pub fn merge_segments(segments: Vec<Segment>) -> Vec<Segment> {
    if segments.is_empty() {
        return segments;
    }

    let mut merged = Vec::new();
    let mut current = segments[0].clone();

    for next in segments.into_iter().skip(1) {
        // Check if current segment ends with sentence terminator
        let current_text = current.text.trim();
        let ends_with_terminator = current_text.ends_with('.')
            || current_text.ends_with('!')
            || current_text.ends_with('?')
            || current_text.ends_with('。')  // Chinese/Japanese period
            || current_text.ends_with('！')  // Chinese/Japanese exclamation
            || current_text.ends_with('？'); // Chinese/Japanese question

        // Check time gap
        let time_gap = next.start - current.end;
        let large_gap = time_gap > 1.5;

        // Check if next segment starts with capital letter (might be new sentence)
        let next_text = next.text.trim();
        let starts_with_capital = next_text.chars().next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false);

        // Decide whether to merge
        let should_merge = !ends_with_terminator 
            && !large_gap 
            && !starts_with_capital;

        if should_merge {
            // Merge with current
            let separator = if current_text.ends_with(',') 
                || current_text.ends_with('，')  // Chinese comma
            {
                " "
            } else {
                " "
            };
            
            current.text = format!("{}{}{}", current.text.trim(), separator, next.text.trim());
            current.end = next.end;
        } else {
            // Push current and start new
            merged.push(current);
            current = next;
        }
    }

    // Don't forget the last segment
    merged.push(current);

    merged
}

/// Merges segments within a chunk if needed
pub fn merge_chunk_segments(segments: Vec<Segment>) -> Vec<Segment> {
    merge_segments(segments)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_incomplete_sentences() {
        let segments = vec![
            Segment {
                index: 0,
                text: "Hello".to_string(),
                start: 0.0,
                end: 0.5,
                confidence: None,
            },
            Segment {
                index: 1,
                text: ", world".to_string(),
                start: 0.5,
                end: 1.0,
                confidence: None,
            },
            Segment {
                index: 2,
                text: "!".to_string(),
                start: 1.0,
                end: 1.2,
                confidence: None,
            },
        ];

        let merged = merge_segments(segments);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].text.trim(), "Hello , world !");
        assert_eq!(merged[0].start, 0.0);
        assert_eq!(merged[0].end, 1.2);
    }

    #[test]
    fn test_keep_complete_sentences() {
        let segments = vec![
            Segment {
                index: 0,
                text: "First sentence.".to_string(),
                start: 0.0,
                end: 1.0,
                confidence: None,
            },
            Segment {
                index: 1,
                text: "Second sentence.".to_string(),
                start: 1.5,
                end: 2.5,
                confidence: None,
            },
        ];

        let merged = merge_segments(segments);
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn test_merge_comma_separated() {
        let segments = vec![
            Segment {
                index: 0,
                text: "Hello,".to_string(),
                start: 0.0,
                end: 0.5,
                confidence: None,
            },
            Segment {
                index: 1,
                text: "world".to_string(),
                start: 0.5,
                end: 1.0,
                confidence: None,
            },
        ];

        let merged = merge_segments(segments);
        assert_eq!(merged.len(), 1);
        assert!(merged[0].text.contains("Hello"));
        assert!(merged[0].text.contains("world"));
    }
}

