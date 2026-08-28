use serde::Serialize;

use crate::{CoreError, IndexedSequence};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SequenceMotion {
    pub transitions: Vec<FrameTransition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FrameTransition {
    pub from_frame_id: String,
    pub to_frame_id: String,
    pub changed_pixels: u64,
    pub silhouette_changes: u64,
    pub opaque_color_changes: u64,
    pub opaque_overlap_pixels: u64,
}

/// Measures exact indexed-pixel changes between adjacent frames and loop closure.
///
/// `opaque_color_changes` isolates palette-index churn where both frames remain
/// opaque. A high value paired with few silhouette changes is a useful signal
/// that nominally stationary artwork is changing colour between poses.
///
/// # Errors
/// Returns [`CoreError`] when the sequence is invalid.
pub fn inspect_sequence_motion(sequence: &IndexedSequence) -> Result<SequenceMotion, CoreError> {
    sequence.validate()?;
    if sequence.frames.len() < 2 {
        return Ok(SequenceMotion {
            transitions: Vec::new(),
        });
    }
    let transparent = sequence.palette.transparent_index;
    let transitions = sequence
        .frames
        .iter()
        .zip(sequence.frames.iter().cycle().skip(1))
        .take(sequence.frames.len())
        .map(|(from, to)| {
            let mut changed_pixels = 0;
            let mut silhouette_changes = 0;
            let mut opaque_color_changes = 0;
            let mut opaque_overlap_pixels = 0;
            for (&before, &after) in from.pixels.iter().zip(&to.pixels) {
                let before_opaque = before != transparent;
                let after_opaque = after != transparent;
                changed_pixels += u64::from(before != after);
                silhouette_changes += u64::from(before_opaque != after_opaque);
                opaque_overlap_pixels += u64::from(before_opaque && after_opaque);
                opaque_color_changes += u64::from(before_opaque && after_opaque && before != after);
            }
            FrameTransition {
                from_frame_id: from.id.clone(),
                to_frame_id: to.id.clone(),
                changed_pixels,
                silhouette_changes,
                opaque_color_changes,
                opaque_overlap_pixels,
            }
        })
        .collect();
    Ok(SequenceMotion { transitions })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::{IndexedFrame, Palette, SEQUENCE_SCHEMA};

    use super::*;

    #[test]
    fn reports_colour_churn_separately_from_silhouette_motion() {
        let sequence = IndexedSequence {
            schema: SEQUENCE_SCHEMA.into(),
            width: 3,
            height: 1,
            palette: Palette::new(
                "motion",
                0,
                vec![[0, 0, 0, 0], [1, 1, 1, 255], [2, 2, 2, 255]],
            ),
            frames: vec![
                IndexedFrame {
                    id: "a".into(),
                    name: None,
                    duration_ms: 100,
                    pixels: vec![0, 1, 1],
                },
                IndexedFrame {
                    id: "b".into(),
                    name: None,
                    duration_ms: 100,
                    pixels: vec![1, 2, 1],
                },
            ],
            pivot: None,
            metadata: BTreeMap::new(),
        };
        let motion = inspect_sequence_motion(&sequence).unwrap();
        assert_eq!(motion.transitions.len(), 2);
        assert_eq!(motion.transitions[0].changed_pixels, 2);
        assert_eq!(motion.transitions[0].silhouette_changes, 1);
        assert_eq!(motion.transitions[0].opaque_color_changes, 1);
        assert_eq!(motion.transitions[0].opaque_overlap_pixels, 2);
    }
}
