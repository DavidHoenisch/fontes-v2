//! Shared build/client contracts. Signature verification and installation are separate layers.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const CONTENT_SCHEMA_VERSION: u32 = 1;
pub const MANIFEST_VERSION: u32 = 1;
pub const UPDATE_PROTOCOL_VERSION: u32 = 1;
pub const SQLITE_APPLICATION_ID: u32 = 0x464E5453;
pub const CANONICAL_REGISTRY_JSON: &str = include_str!("../../../data/canonical/registry-v1.json");
pub const CONTENT_SCHEMA_SQL: &str = include_str!("../../../data/schema/content-v1.sql");
pub const USER_SCHEMA_SQL: &str = include_str!("../../../data/schema/user-v1.sql");

/// Book IDs are permanent registry IDs, independent of localized names and display order.
pub fn canonical_ref_id(book_id: u16, chapter: u16, verse: u16) -> Result<i64, String> {
    if !(1..=999).contains(&book_id) || !(1..=999).contains(&chapter) || !(1..=999).contains(&verse)
    {
        return Err("reference components must be in 1..=999".into());
    }
    Ok(i64::from(book_id) * 1_000_000 + i64::from(chapter) * 1_000 + i64::from(verse))
}

fn identifier(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 100
        && s.bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
}

fn sha256(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    Stable,
    Preview,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    Reading,
    StrongsLexicon,
    GreekMorphology,
    HebrewMorphology,
    Commentary,
    Topics,
    CrossReferences,
    Genealogies,
    BookOverviews,
}

impl Channel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Preview => "preview",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceSnapshot {
    pub source_id: String,
    pub sha256: String,
    pub adapter_version: String,
    pub rights_review_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AllowedRights {
    PublicDomain,
    PublicDomainWithRegionalException,
    #[serde(rename = "CC0-1.0")]
    Cc0,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LockedSource {
    pub source_id: String,
    pub url: String,
    pub sha256: String,
    pub bytes: u64,
    pub rights_class: AllowedRights,
    pub rights_review_id: String,
    pub rights_evidence_url: String,
    pub adapter: String,
    pub upstream_revision: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceLock {
    pub lock_version: u32,
    pub canonical_registry_version: u32,
    pub sources: Vec<LockedSource>,
}

impl SourceLock {
    pub fn validate(&self) -> Result<(), String> {
        if self.lock_version != 1 || self.canonical_registry_version != 1 || self.sources.is_empty()
        {
            return Err("unsupported or empty source lock".into());
        }
        let mut ids = BTreeSet::new();
        for source in &self.sources {
            if !identifier(&source.source_id)
                || !ids.insert(source.source_id.as_str())
                || !identifier(&source.adapter)
                || !sha256(&source.sha256)
                || source.bytes == 0
                || !source.url.starts_with("https://")
                || !source.rights_evidence_url.starts_with("https://")
                || source.rights_review_id.is_empty()
            {
                return Err("invalid or duplicate locked source".into());
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileArtifact {
    pub profile: String,
    pub capabilities: Vec<Capability>,
    pub asset: String,
    pub compression: String,
    pub compressed_bytes: u64,
    pub compressed_sha256: String,
    pub sqlite_bytes: u64,
    pub sqlite_sha256: String,
    pub content_id: String,
    pub source_ids: Vec<String>,
    pub canonical_ref_count: u32,
    pub verse_text_count: u64,
    pub resource_count: u64,
}

/// This describes the bytes being signed, not a trusted object by itself.
/// Clients verify a detached signature over the original bytes before parsing.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseManifest {
    pub manifest_version: u32,
    pub update_protocol_version: u32,
    pub content_schema_version: u32,
    pub canonical_registry_version: u32,
    pub repository: String,
    pub release_tag: String,
    pub channel: Channel,
    pub sequence: u64,
    pub published_at: String,
    pub signing_key_id: String,
    pub builder_commit: String,
    pub source_lock_sha256: String,
    pub profiles: Vec<ProfileArtifact>,
    pub sources: Vec<SourceSnapshot>,
}

impl ReleaseManifest {
    /// Structural and cross-field checks only. Never substitutes for signature verification.
    pub fn validate(&self) -> Result<(), String> {
        if self.manifest_version != MANIFEST_VERSION
            || self.update_protocol_version != UPDATE_PROTOCOL_VERSION
            || self.content_schema_version != CONTENT_SCHEMA_VERSION
            || self.canonical_registry_version != 1
        {
            return Err("unsupported manifest, protocol, schema, or reference registry".into());
        }
        let parts: Vec<_> = self.repository.split('/').collect();
        if parts.len() != 2
            || parts.iter().any(|s| {
                s.is_empty()
                    || *s == "."
                    || *s == ".."
                    || !s
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || b"-_.".contains(&c))
            })
        {
            return Err("repository must be owner/name".into());
        }
        let tag = format!(
            "data-v{}-{}-{}",
            self.content_schema_version,
            self.channel.as_str(),
            self.sequence
        );
        if self.sequence == 0 || self.sequence > i64::MAX as u64 || self.release_tag != tag {
            return Err("release tag does not match schema, channel, and sequence".into());
        }
        if !identifier(&self.signing_key_id)
            || self.published_at.is_empty()
            || self.builder_commit.len() != 40
            || !self.builder_commit.bytes().all(|c| c.is_ascii_hexdigit())
            || !sha256(&self.source_lock_sha256)
        {
            return Err("invalid release provenance".into());
        }
        let mut sources = BTreeSet::new();
        for source in &self.sources {
            if !identifier(&source.source_id)
                || !sha256(&source.sha256)
                || source.adapter_version.is_empty()
                || source.rights_review_id.is_empty()
                || !sources.insert(source.source_id.as_str())
            {
                return Err("invalid or duplicate source snapshot".into());
            }
        }
        if self.profiles.is_empty() || sources.is_empty() {
            return Err("release must contain profiles and sources".into());
        }
        let mut profiles = BTreeSet::new();
        let mut assets = BTreeSet::new();
        for profile in &self.profiles {
            if !identifier(&profile.profile)
                || !profiles.insert(profile.profile.as_str())
                || !assets.insert(profile.asset.as_str())
                || profile.asset != format!("fontes-{}.sqlite.zst", profile.profile)
                || profile.compression != "zstd"
                || profile.compressed_bytes == 0
                || profile.sqlite_bytes < 4096
                || !sha256(&profile.compressed_sha256)
                || !sha256(&profile.sqlite_sha256)
                || !sha256(&profile.content_id)
                || profile.canonical_ref_count == 0
                || profile.source_ids.is_empty()
                || !profile.capabilities.contains(&Capability::Reading)
                || profile.capabilities.iter().collect::<BTreeSet<_>>().len()
                    != profile.capabilities.len()
            {
                return Err("invalid or duplicate profile artifact".into());
            }
            let members: BTreeSet<_> = profile.source_ids.iter().map(String::as_str).collect();
            if members.len() != profile.source_ids.len()
                || members.iter().any(|s| !sources.contains(s))
            {
                return Err("profile references missing or duplicate source IDs".into());
            }
        }
        Ok(())
    }

    /// The configured repository must also match; clients must not accept a manifest-chosen host.
    pub fn asset_url(&self, profile: &str) -> Option<String> {
        self.validate().ok()?;
        let artifact = self.profiles.iter().find(|p| p.profile == profile)?;
        Some(format!(
            "https://github.com/{}/releases/download/{}/{}",
            self.repository, self.release_tag, artifact.asset
        ))
    }
}
