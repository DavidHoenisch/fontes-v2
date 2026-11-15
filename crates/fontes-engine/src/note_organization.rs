//! Additive user-owned folder/tag metadata; published user-v1 bytes stay unchanged.
use crate::*;
use schemars::JsonSchema;
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NoteOrganization {
    pub folders: Vec<NoteFolder>,
    pub entries: Vec<NoteOrganizationEntry>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NoteFolder {
    pub id: String,
    pub name: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NoteOrganizationEntry {
    pub note_id: String,
    pub folder_id: Option<String>,
    pub tags: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NoteOrganizationChange {
    pub note_id: String,
    pub folder_id: Option<String>,
    pub tags: Vec<String>,
}
impl Engine {
    pub(crate) fn note_organization(&self) -> Result<NoteOrganization> {
        let folders = self
            .user
            .prepare("SELECT id,name FROM fontes_note_folders_v1 ORDER BY name COLLATE NOCASE,id")?
            .query_map([], |r| {
                Ok(NoteFolder {
                    id: r.get(0)?,
                    name: r.get(1)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let entries = self.user.prepare("SELECT note_id,folder_id,tags_json FROM fontes_note_organization_v1 ORDER BY note_id")?.query_map([], |r| Ok((r.get::<_,String>(0)?, r.get::<_,Option<String>>(1)?, r.get::<_,String>(2)?)))?.collect::<std::result::Result<Vec<_>,_>>()?.into_iter().map(|(note_id,folder_id,tags)|Ok(NoteOrganizationEntry{note_id,folder_id,tags:serde_json::from_str(&tags)?})).collect::<Result<Vec<_>>>()?;
        Ok(NoteOrganization { folders, entries })
    }
    pub(crate) fn save_note_folder(&mut self, id: Option<String>, name: String) -> Result<Data> {
        let name = name.trim();
        if name.is_empty() || name.len() > 128 {
            return Err(Error::invalid("Folder names must contain 1–128 bytes."));
        }
        let id = match id {
            Some(id) => {
                if !self.user.query_row(
                    "SELECT EXISTS(SELECT 1 FROM fontes_note_folders_v1 WHERE id=?1)",
                    [&id],
                    |r| r.get::<_, bool>(0),
                )? {
                    return Err(Error::missing("Folder no longer exists."));
                }
                id
            }
            None => uuid::Uuid::new_v4().to_string(),
        };
        if self.user.query_row("SELECT EXISTS(SELECT 1 FROM fontes_note_folders_v1 WHERE lower(name)=lower(?1) AND id<>?2)",params![name,id],|r|r.get::<_,bool>(0))? {return Err(Error::invalid("A folder with that name already exists."));}
        self.user.execute("INSERT INTO fontes_note_folders_v1(id,name) VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET name=excluded.name",params![id,name])?;
        Ok(Data::Workspace)
    }
    pub(crate) fn delete_note_folder(&mut self, id: String) -> Result<Data> {
        self.user
            .execute("DELETE FROM fontes_note_folders_v1 WHERE id=?1", [id])?;
        Ok(Data::Workspace)
    }
    pub(crate) fn organize_notes(&mut self, changes: Vec<NoteOrganizationChange>) -> Result<Data> {
        if changes.is_empty() || changes.len() > 200 {
            return Err(Error::invalid(
                "Organize between 1 and 200 notes at a time.",
            ));
        }
        let tx = self.user.transaction()?;
        for mut change in changes {
            if !tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM annotations WHERE id=?1 AND type='note')",
                [&change.note_id],
                |r| r.get::<_, bool>(0),
            )? {
                return Err(Error::missing("Note no longer exists."));
            }
            if let Some(folder) = &change.folder_id
                && !tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM fontes_note_folders_v1 WHERE id=?1)",
                    [folder],
                    |r| r.get::<_, bool>(0),
                )?
            {
                return Err(Error::missing("Folder no longer exists."));
            }
            change.tags = change
                .tags
                .into_iter()
                .map(|t| t.trim().trim_start_matches('#').to_lowercase())
                .filter(|t| !t.is_empty())
                .collect();
            change.tags.sort();
            change.tags.dedup();
            if change.tags.len() > 20
                || change
                    .tags
                    .iter()
                    .any(|t| t.len() > 64 || t.contains(['\n', '\r']))
            {
                return Err(Error::invalid("Use up to 20 tags, each at most 64 bytes."));
            }
            tx.execute("INSERT INTO fontes_note_organization_v1(note_id,folder_id,tags_json) VALUES(?1,?2,?3) ON CONFLICT(note_id) DO UPDATE SET folder_id=excluded.folder_id,tags_json=excluded.tags_json",params![change.note_id,change.folder_id,serde_json::to_string(&change.tags)?])?;
        }
        tx.commit()?;
        Ok(Data::Workspace)
    }
}
