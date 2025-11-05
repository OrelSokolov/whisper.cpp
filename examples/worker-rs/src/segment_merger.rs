use log::{info, debug};
use crate::params::SegmentMessage;

/// Segment merger for combining incomplete sentences
pub struct SegmentMerger {
    buffer: Vec<SegmentMessage>,
    enabled: bool,
}

impl SegmentMerger {
    pub fn new(enabled: bool) -> Self {
        Self {
            buffer: Vec::new(),
            enabled,
        }
    }

    /// Process a new segment. Returns merged segments if ready to send.
    pub fn process_segment(&mut self, segment: SegmentMessage, is_final: bool) -> Vec<SegmentMessage> {
        if !self.enabled {
            // Merge disabled, pass through
            return vec![segment];
        }

        // Add to buffer
        self.buffer.push(segment);

        // Check if we should flush the buffer
        if self.should_flush(is_final) {
            let merged = self.merge_buffer();
            self.buffer.clear();
            merged
        } else {
            // Not ready yet, wait for more segments
            Vec::new()
        }
    }

    /// Flush any remaining segments (call at end of stream)
    pub fn flush(&mut self) -> Vec<SegmentMessage> {
        if self.buffer.is_empty() {
            return Vec::new();
        }

        let merged = self.merge_buffer();
        self.buffer.clear();
        merged
    }

    /// Determine if buffer should be flushed
    fn should_flush(&self, is_final: bool) -> bool {
        if self.buffer.is_empty() {
            return false;
        }

        // Always flush if this is the final segment
        if is_final {
            return true;
        }

        // Check if last segment ends with sentence terminator
        if let Some(last) = self.buffer.last() {
            let text = last.text.trim();
            let ends_with_terminator = text.ends_with('.')
                || text.ends_with('!')
                || text.ends_with('?')
                || text.ends_with('。')  // Chinese/Japanese period
                || text.ends_with('！')  // Chinese/Japanese exclamation
                || text.ends_with('？'); // Chinese/Japanese question

            if ends_with_terminator {
                debug!("Flushing buffer: sentence complete");
                return true;
            }
        }

        // Don't flush - waiting for more segments
        false
    }

    /// Merge segments in buffer into complete sentences
    fn merge_buffer(&self) -> Vec<SegmentMessage> {
        if self.buffer.is_empty() {
            return Vec::new();
        }

        if self.buffer.len() == 1 {
            return self.buffer.clone();
        }

        let mut merged = Vec::new();
        let mut current = self.buffer[0].clone();

        for next in self.buffer.iter().skip(1) {
            // Check if current segment ends with sentence terminator
            let current_text = current.text.trim();
            let ends_with_terminator = current_text.ends_with('.')
                || current_text.ends_with('!')
                || current_text.ends_with('?')
                || current_text.ends_with('。')
                || current_text.ends_with('！')
                || current_text.ends_with('？');

            // Check time gap
            let time_gap = if let (Some(current_end), Some(next_start)) = (current.end, next.start) {
                next_start - current_end
            } else {
                0.0
            };
            let large_gap = time_gap > 1.5;

            // Check if next segment starts with capital letter
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
                let separator = if current_text.ends_with(',') || current_text.ends_with('，') {
                    " "
                } else if current_text.is_empty() {
                    ""
                } else {
                    " "
                };
                
                current.text = format!("{}{}{}", current.text.trim(), separator, next.text.trim());
                
                // Update end time
                if let Some(next_end) = next.end {
                    current.end = Some(next_end);
                }
                
                // Update progress and eta if available
                if next.progress.is_some() {
                    current.progress = next.progress;
                }
                if next.eta.is_some() {
                    current.eta = next.eta;
                }
            } else {
                // Push current and start new
                merged.push(current);
                current = next.clone();
            }
        }

        // Don't forget the last segment
        merged.push(current);

        info!("Merged {} segments into {} complete sentence(s)", 
              self.buffer.len(), merged.len());

        merged
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_incomplete_sentences() {
        let mut merger = SegmentMerger::new(true);

        let seg1 = SegmentMessage {
            msg_type: "segment".to_string(),
            index: 0,
            text: "Hello".to_string(),
            start: Some(0.0),
            end: Some(0.5),
            progress: None,
            eta: None,
            speaker: None,
        };

        let seg2 = SegmentMessage {
            msg_type: "segment".to_string(),
            index: 1,
            text: ", world!".to_string(),
            start: Some(0.5),
            end: Some(1.0),
            progress: None,
            eta: None,
            speaker: None,
        };

        // Process first segment (incomplete)
        let result1 = merger.process_segment(seg1, false);
        assert!(result1.is_empty()); // Not flushed yet

        // Process second segment (complete with !)
        let result2 = merger.process_segment(seg2, false);
        assert_eq!(result2.len(), 1);
        assert!(result2[0].text.contains("Hello"));
        assert!(result2[0].text.contains("world"));
    }

    #[test]
    fn test_passthrough_when_disabled() {
        let mut merger = SegmentMerger::new(false);

        let seg = SegmentMessage {
            msg_type: "segment".to_string(),
            index: 0,
            text: "Test".to_string(),
            start: Some(0.0),
            end: Some(0.5),
            progress: None,
            eta: None,
            speaker: None,
        };

        let result = merger.process_segment(seg.clone(), false);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].text, seg.text);
    }

    #[test]
    fn test_flush_final() {
        let mut merger = SegmentMerger::new(true);

        let seg = SegmentMessage {
            msg_type: "segment".to_string(),
            index: 0,
            text: "Incomplete sentence".to_string(),
            start: Some(0.0),
            end: Some(1.0),
            progress: None,
            eta: None,
            speaker: None,
        };

        // Process as final segment
        let result = merger.process_segment(seg, true);
        assert_eq!(result.len(), 1);
    }
}

