//! Replication-safe File Tunnel upload job model.
//!
//! The optional `opto` feature uses the pinned `opto-sync-client` submodule.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
    #[error("failed to serialize replication-safe upload job")]
    Serialize,
    #[error("opto-sync refused the upload job mutation")]
    Queue,
}

impl UploadJob {
    pub fn validate(&self) -> Result<(), Error> {
        if self.bytes_transferred > self.size_bytes {
            return Err(Error::InvalidProgress);
        }
        if self.name.is_empty() || self.media_type.is_empty() {
            return Err(Error::MissingMetadata);
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
            updated_at: "1722276000000-0-device".to_owned(),
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
}
