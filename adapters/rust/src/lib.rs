//! Replication-safe File Tunnel upload job model.
//!
//! The optional `opto` feature uses the pinned `opto-sync-client` submodule.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "opto")]
use opto_sync_client::clock::{format_hlc, HlcParts};
#[cfg(feature = "opto")]
use opto_sync_client::sqlite::{SqliteProtocolStore, SqliteStoreError};
#[cfg(feature = "opto")]
use rusqlite::{OptionalExtension, TransactionBehavior};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UploadStatus {
    Queued,
    Declaring,
    Uploading,
    Paused,
    Available,
    Imported,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UploadJob {
    pub id: Uuid,
    pub tunnel_id: Uuid,
    pub file_id: Option<Uuid>,
    pub name: String,
    pub media_type: String,
    pub size_bytes: u64,
    pub bytes_transferred: u64,
    pub status: UploadStatus,
    pub attempt: u16,
    pub reason_code: Option<ReasonCode>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "syncedAt")]
    pub synced_at: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReasonCode {
    NetworkUnavailable,
    PermissionRequired,
    SourceMissing,
    TunnelExpired,
    FileRejected,
    UploadInterrupted,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    #[error("bytes transferred exceeds declared file size")]
    InvalidProgress,
    #[error("file name or media type is empty")]
    MissingMetadata,
    #[error("file name or media type exceeds the replication contract")]
    MetadataTooLarge,
    #[error("declared file size exceeds the replication contract")]
    FileTooLarge,
    #[error("attempt count exceeds the replication contract")]
    AttemptLimit,
    #[error("updatedAt is not a canonical opto-sync HLC timestamp")]
    InvalidTimestamp,
    #[error("failed to serialize replication-safe upload job")]
    Serialize,
    #[error("opto-sync refused the upload job mutation")]
    Queue,
    #[cfg(feature = "opto")]
    #[error("durable opto-sync state failed: {0}")]
    Durable(String),
    #[cfg(feature = "opto")]
    #[error("durable upload state is invalid")]
    Deserialize,
    #[cfg(feature = "opto")]
    #[error("the durable writer id must be non-empty and contain no '-'")]
    InvalidWriterId,
}

#[must_use]
pub const fn progress_is_valid(size_bytes: u64, bytes_transferred: u64) -> bool {
    bytes_transferred <= size_bytes
}

impl UploadJob {
    pub fn validate(&self) -> Result<(), Error> {
        if !progress_is_valid(self.size_bytes, self.bytes_transferred) {
            return Err(Error::InvalidProgress);
        }
        if self.name.is_empty() || self.media_type.is_empty() {
            return Err(Error::MissingMetadata);
        }
        if self.name.chars().count() > 255 || self.media_type.chars().count() > 128 {
            return Err(Error::MetadataTooLarge);
        }
        if self.size_bytes > 5_368_709_120 {
            return Err(Error::FileTooLarge);
        }
        if self.attempt > 100 {
            return Err(Error::AttemptLimit);
        }
        if !valid_hlc(&self.updated_at) {
            return Err(Error::InvalidTimestamp);
        }
        Ok(())
    }

    pub fn replication_json(&self) -> Result<String, Error> {
        self.validate()?;
        serde_json::to_string(self).map_err(|_| Error::Serialize)
    }
}

#[cfg(feature = "opto")]
pub fn queue_with_opto<S: opto_sync_client::MutationStore>(
    client: &mut opto_sync_client::OptoSyncClient<S>,
    job: &UploadJob,
) -> Result<u64, Error> {
    let payload = job.replication_json()?;
    client.queue_mutation(payload).map_err(|_| Error::Queue)
}

/// Protocol-v1 SQLite queue for upload metadata.
///
/// File bytes, filesystem paths, capabilities, and pairing secrets never enter
/// this store. Every job upsert and protocol mutation is committed atomically by
/// [`SqliteProtocolStore`], so a crash cannot leave an optimistic record without
/// the mutation that will reconcile it.
#[cfg(feature = "opto")]
pub struct DurableUploadQueue {
    store: SqliteProtocolStore,
    writer_id: String,
}

#[cfg(feature = "opto")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueuedUpload {
    pub mutation_id: String,
    pub job: UploadJob,
}

#[cfg(feature = "opto")]
impl DurableUploadQueue {
    pub fn open(
        path: impl AsRef<std::path::Path>,
        writer_id: impl Into<String>,
    ) -> Result<Self, Error> {
        let writer_id = validate_writer_id(writer_id.into())?;
        let store = SqliteProtocolStore::open(path, writer_id.clone()).map_err(durable)?;
        let mut queue = Self { store, writer_id };
        queue.initialize_clock()?;
        Ok(queue)
    }

    pub fn open_in_memory(writer_id: impl Into<String>) -> Result<Self, Error> {
        let writer_id = validate_writer_id(writer_id.into())?;
        let store = SqliteProtocolStore::open_in_memory(writer_id.clone()).map_err(durable)?;
        let mut queue = Self { store, writer_id };
        queue.initialize_clock()?;
        Ok(queue)
    }

    /// Queue one replication-safe upload job and materialize the local view in
    /// the same SQLite transaction. `updatedAt` is replaced with a durable HLC
    /// timestamp; callers never supply a wall-clock ordering value.
    pub fn queue(&mut self, mut job: UploadJob) -> Result<QueuedUpload, Error> {
        job.updated_at = self.next_timestamp()?;
        job.validate()?;
        let payload = serde_json::to_value(&job).map_err(|_| Error::Serialize)?;
        let mutation_id = self
            .store
            .queue_upsert_record("ftnl_upload_jobs", job.id.to_string(), payload, None, false)
            .map_err(durable)?;
        Ok(QueuedUpload { mutation_id, job })
    }

    pub fn load(&self, job_id: Uuid) -> Result<Option<UploadJob>, Error> {
        self.store
            .local_record("ftnl_upload_jobs", &job_id.to_string())
            .map_err(durable)?
            .map(|record| {
                let job = serde_json::from_value::<UploadJob>(record.record)
                    .map_err(|_| Error::Deserialize)?;
                job.validate()?;
                Ok(job)
            })
            .transpose()
    }

    pub fn pending_count(&self) -> Result<usize, Error> {
        Ok(self.store.load_queue().map_err(durable)?.pending().count())
    }

    fn initialize_clock(&mut self) -> Result<(), Error> {
        self.store
            .connection_mut()
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS _ftnl_sync_clock (
                   singleton INTEGER PRIMARY KEY NOT NULL CHECK (singleton = 1),
                   millis INTEGER NOT NULL CHECK (millis >= 0),
                   counter INTEGER NOT NULL CHECK (counter >= 0 AND counter <= 65535)
                 ) STRICT;
                 INSERT OR IGNORE INTO _ftnl_sync_clock(singleton, millis, counter)
                 VALUES (1, 0, 0);",
            )
            .map_err(|error| durable(SqliteStoreError::Sqlite(error)))?;
        Ok(())
    }

    fn next_timestamp(&mut self) -> Result<String, Error> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| i64::try_from(duration.as_millis()).unwrap_or(i64::MAX))
            .unwrap_or(0);
        let transaction = self
            .store
            .connection_mut()
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| durable(SqliteStoreError::Sqlite(error)))?;
        let previous = transaction
            .query_row(
                "SELECT millis, counter FROM _ftnl_sync_clock WHERE singleton = 1",
                [],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
            )
            .optional()
            .map_err(|error| durable(SqliteStoreError::Sqlite(error)))?
            .unwrap_or((0, 0));
        let (millis, counter) = if now > previous.0 {
            (now, 0)
        } else if previous.1 < 0xffff {
            (previous.0, previous.1 + 1)
        } else {
            (
                previous
                    .0
                    .checked_add(1)
                    .ok_or_else(|| Error::Durable("hybrid clock exhausted".to_owned()))?,
                0,
            )
        };
        transaction
            .execute(
                "UPDATE _ftnl_sync_clock SET millis = ?1, counter = ?2 WHERE singleton = 1",
                (millis, counter),
            )
            .map_err(|error| durable(SqliteStoreError::Sqlite(error)))?;
        transaction
            .commit()
            .map_err(|error| durable(SqliteStoreError::Sqlite(error)))?;
        Ok(format_hlc(&HlcParts {
            millis: millis as u64,
            counter: counter as u32,
            node_id: self.writer_id.clone(),
        }))
    }
}

#[cfg(feature = "opto")]
fn validate_writer_id(writer_id: String) -> Result<String, Error> {
    if writer_id.is_empty() || writer_id.contains('-') {
        return Err(Error::InvalidWriterId);
    }
    Ok(writer_id)
}

#[cfg(feature = "opto")]
fn durable(error: SqliteStoreError) -> Error {
    Error::Durable(error.to_string())
}

fn valid_hlc(timestamp: &str) -> bool {
    let mut fields = timestamp.splitn(3, '-');
    let (Some(millis), Some(counter), Some(node_id)) =
        (fields.next(), fields.next(), fields.next())
    else {
        return false;
    };
    millis.len() == 13
        && millis.bytes().all(|byte| byte.is_ascii_digit())
        && counter.len() == 4
        && counter.bytes().all(|byte| byte.is_ascii_hexdigit())
        && !node_id.is_empty()
        && !node_id.contains('-')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job() -> UploadJob {
        UploadJob {
            id: Uuid::nil(),
            tunnel_id: Uuid::nil(),
            file_id: None,
            name: "photo.jpg".to_owned(),
            media_type: "image/jpeg".to_owned(),
            size_bytes: 100,
            bytes_transferred: 25,
            status: UploadStatus::Uploading,
            attempt: 1,
            reason_code: None,
            updated_at: "1722276000000-0000-device".to_owned(),
            synced_at: None,
        }
    }

    #[test]
    fn serialized_record_has_no_secret_or_content_fields() {
        let json = job().replication_json().unwrap();
        for forbidden in [
            "capability",
            "pairing_secret",
            "event_ticket",
            "local_ref",
            "presigned_url",
            "content",
        ] {
            assert!(!json.contains(forbidden));
        }
    }

    #[test]
    fn progress_cannot_exceed_declared_size() {
        let mut invalid = job();
        invalid.bytes_transferred = 101;
        assert_eq!(invalid.validate(), Err(Error::InvalidProgress));
    }

    #[cfg(feature = "opto")]
    #[test]
    fn durable_queue_atomically_persists_the_job_and_protocol_mutation() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("upload-state.sqlite3");
        let first_timestamp;
        {
            let mut queue = DurableUploadQueue::open(&path, "device01.cli01").unwrap();
            let queued = queue.queue(job()).unwrap();
            first_timestamp = queued.job.updated_at.clone();
            assert_eq!(queued.mutation_id, "1");
            assert_eq!(queue.pending_count().unwrap(), 1);
            assert_eq!(queue.load(Uuid::nil()).unwrap(), Some(queued.job));
        }

        let mut reopened = DurableUploadQueue::open(&path, "device01.cli01").unwrap();
        let mut changed = reopened.load(Uuid::nil()).unwrap().unwrap();
        changed.status = UploadStatus::Available;
        changed.bytes_transferred = changed.size_bytes;
        let queued = reopened.queue(changed).unwrap();
        assert!(queued.job.updated_at > first_timestamp);
        assert_eq!(queued.mutation_id, "2");
        assert_eq!(reopened.pending_count().unwrap(), 2);
    }

    #[cfg(feature = "opto")]
    #[test]
    fn durable_queue_rejects_an_ambiguous_hlc_writer_id() {
        assert!(matches!(
            DurableUploadQueue::open_in_memory("has-a-dash"),
            Err(Error::InvalidWriterId)
        ));
    }

    proptest::proptest! {
        #[test]
        fn progress_validation_matches_the_declared_bound(
            size_bytes in proptest::prelude::any::<u64>(),
            bytes_transferred in proptest::prelude::any::<u64>(),
        ) {
            proptest::prop_assert_eq!(
                progress_is_valid(size_bytes, bytes_transferred),
                bytes_transferred <= size_bytes
            );
        }
    }
}

#[cfg(kani)]
mod verification {
    use super::progress_is_valid;

    #[kani::proof]
    fn persisted_progress_never_exceeds_declared_size() {
        let size_bytes = kani::any::<u64>();
        let bytes_transferred = kani::any::<u64>();
        assert_eq!(
            progress_is_valid(size_bytes, bytes_transferred),
            bytes_transferred <= size_bytes
        );
    }
}
