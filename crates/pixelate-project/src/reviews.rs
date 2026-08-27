use std::fs;

use pixelate_core::stable_json;

use crate::{
    ProjectError, ProjectStore, REVIEW_SCHEMA, ReviewActorKind, ReviewDecision, ReviewEvent,
    ReviewRecord,
    persistence::{atomic_write, ensure_schema, io_at, now_unix_ms, read_json},
};

impl ProjectStore {
    /// Loads a revision's durable review record, if review has begun.
    ///
    /// # Errors
    ///
    /// Returns a [`ProjectError`] when identities, schemas, or review JSON are invalid.
    pub fn review(
        &self,
        asset_id: &str,
        revision: &str,
    ) -> Result<Option<ReviewRecord>, ProjectError> {
        self.revision(asset_id, revision)?;
        let path = self.review_path(asset_id, revision);
        if !path.is_file() {
            return Ok(None);
        }
        let record: ReviewRecord = read_json(&path)?;
        validate_review_record(&record, asset_id, revision)?;
        Ok(Some(record))
    }

    /// Atomically appends an explicit human or agent review event.
    ///
    /// # Errors
    ///
    /// Returns a [`ProjectError`] when the revision, actor, record, lock, or
    /// durable write is invalid.
    pub fn record_review(
        &self,
        asset_id: &str,
        revision: &str,
        actor: &str,
        actor_kind: ReviewActorKind,
        decision: ReviewDecision,
        note: &str,
    ) -> Result<ReviewRecord, ProjectError> {
        if actor.trim().is_empty() {
            return Err(ProjectError::EmptyReviewActor);
        }
        self.revision(asset_id, revision)?;
        let lock = self.lock()?;
        let path = self.review_path(asset_id, revision);
        let mut record = if path.is_file() {
            let record: ReviewRecord = read_json(&path)?;
            validate_review_record(&record, asset_id, revision)?;
            record
        } else {
            ReviewRecord {
                schema: REVIEW_SCHEMA.to_owned(),
                asset: asset_id.to_owned(),
                revision: revision.to_owned(),
                events: Vec::new(),
            }
        };
        let sequence = u64::try_from(record.events.len())
            .map_err(|_| ProjectError::Clock)?
            .checked_add(1)
            .ok_or(ProjectError::Clock)?;
        record.events.push(ReviewEvent {
            sequence,
            created_unix_ms: now_unix_ms()?,
            actor: actor.to_owned(),
            actor_kind,
            decision,
            note: note.to_owned(),
        });
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| io_at(parent, source))?;
        }
        atomic_write(&path, &stable_json(&record)?)?;
        drop(lock);
        Ok(record)
    }
}

fn validate_review_record(
    record: &ReviewRecord,
    asset_id: &str,
    revision: &str,
) -> Result<(), ProjectError> {
    ensure_schema(&record.schema, REVIEW_SCHEMA)?;
    let events_valid = record.events.iter().enumerate().all(|(index, event)| {
        u64::try_from(index)
            .ok()
            .and_then(|index| index.checked_add(1))
            == Some(event.sequence)
            && !event.actor.trim().is_empty()
    });
    if record.asset == asset_id && record.revision == revision && events_valid {
        Ok(())
    } else {
        Err(ProjectError::InvalidReviewRecord)
    }
}
