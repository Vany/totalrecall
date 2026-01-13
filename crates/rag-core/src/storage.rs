use crate::{Memory, MemoryScope};
use anyhow::{Context, Result};
use sled::Db;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

pub struct MemoryStore {
    session: HashMap<String, Memory>,
    global_db: Option<Db>,
    project_dbs: HashMap<PathBuf, Db>,
    global_db_path: PathBuf,
}

impl MemoryStore {
    pub fn new(global_db_path: PathBuf) -> Result<Self> {
        let global_db = if global_db_path.exists() || global_db_path.parent().map(|p| p.exists()).unwrap_or(false) {
            if let Some(parent) = global_db_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            Some(sled::open(&global_db_path).context("Failed to open global database")?)
        } else {
            None
        };

        info!("Initialized MemoryStore with global DB at {:?}", global_db_path);

        Ok(Self {
            session: HashMap::new(),
            global_db,
            project_dbs: HashMap::new(),
            global_db_path,
        })
    }

    pub fn store(&mut self, memory: Memory) -> Result<()> {
        debug!("Storing memory: id={}, scope={:?}", memory.id, memory.scope);

        match &memory.scope {
            MemoryScope::Session => {
                self.session.insert(memory.id.clone(), memory);
            }
            MemoryScope::Global => {
                let db = self.get_or_create_global_db()?;
                let serialized = bincode::serialize(&memory)?;
                db.insert(memory.id.as_bytes(), serialized)?;
                db.flush()?;
            }
            MemoryScope::Project { path } => {
                let db = self.get_or_create_project_db(path)?;
                let serialized = bincode::serialize(&memory)?;
                db.insert(memory.id.as_bytes(), serialized)?;
                db.flush()?;
            }
        }

        Ok(())
    }

    pub fn get(&self, id: &str, scope: &MemoryScope) -> Result<Option<Memory>> {
        match scope {
            MemoryScope::Session => Ok(self.session.get(id).cloned()),
            MemoryScope::Global => {
                if let Some(db) = &self.global_db {
                    if let Some(bytes) = db.get(id.as_bytes())? {
                        let memory: Memory = bincode::deserialize(&bytes)?;
                        Ok(Some(memory))
                    } else {
                        Ok(None)
                    }
                } else {
                    Ok(None)
                }
            }
            MemoryScope::Project { path } => {
                if let Some(db) = self.project_dbs.get(path) {
                    if let Some(bytes) = db.get(id.as_bytes())? {
                        let memory: Memory = bincode::deserialize(&bytes)?;
                        Ok(Some(memory))
                    } else {
                        Ok(None)
                    }
                } else {
                    Ok(None)
                }
            }
        }
    }

    pub fn delete(&mut self, id: &str, scope: &MemoryScope) -> Result<bool> {
        match scope {
            MemoryScope::Session => Ok(self.session.remove(id).is_some()),
            MemoryScope::Global => {
                if let Some(db) = &self.global_db {
                    Ok(db.remove(id.as_bytes())?.is_some())
                } else {
                    Ok(false)
                }
            }
            MemoryScope::Project { path } => {
                if let Some(db) = self.project_dbs.get(path) {
                    Ok(db.remove(id.as_bytes())?.is_some())
                } else {
                    Ok(false)
                }
            }
        }
    }

    pub fn list(&self, scope: &MemoryScope, limit: usize, offset: usize) -> Result<Vec<Memory>> {
        let mut memories = Vec::new();

        match scope {
            MemoryScope::Session => {
                memories.extend(self.session.values().cloned());
            }
            MemoryScope::Global => {
                if let Some(db) = &self.global_db {
                    for item in db.iter() {
                        let (_, value) = item?;
                        let memory: Memory = bincode::deserialize(&value)?;
                        memories.push(memory);
                    }
                }
            }
            MemoryScope::Project { path } => {
                if let Some(db) = self.project_dbs.get(path) {
                    for item in db.iter() {
                        let (_, value) = item?;
                        let memory: Memory = bincode::deserialize(&value)?;
                        memories.push(memory);
                    }
                }
            }
        }

        memories.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        Ok(memories.into_iter().skip(offset).take(limit).collect())
    }

    pub fn list_all(&self, scope: &MemoryScope) -> Result<Vec<Memory>> {
        self.list(scope, usize::MAX, 0)
    }

    pub fn clear_session(&mut self) {
        info!("Clearing session memories");
        self.session.clear();
    }

    pub fn stats(&self, scope: &MemoryScope) -> Result<MemoryStats> {
        let count = match scope {
            MemoryScope::Session => self.session.len(),
            MemoryScope::Global => {
                if let Some(db) = &self.global_db {
                    db.len()
                } else {
                    0
                }
            }
            MemoryScope::Project { path } => {
                if let Some(db) = self.project_dbs.get(path) {
                    db.len()
                } else {
                    0
                }
            }
        };

        Ok(MemoryStats {
            total_memories: count,
            scope: scope.clone(),
        })
    }

    fn get_or_create_global_db(&mut self) -> Result<&Db> {
        if self.global_db.is_none() {
            if let Some(parent) = self.global_db_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            self.global_db = Some(sled::open(&self.global_db_path)?);
        }
        Ok(self.global_db.as_ref().unwrap())
    }

    fn get_or_create_project_db(&mut self, path: &Path) -> Result<&Db> {
        if !self.project_dbs.contains_key(path) {
            let db_path = path.join(".rag-mcp").join("data.db");
            if let Some(parent) = db_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let db = sled::open(&db_path)?;
            self.project_dbs.insert(path.to_path_buf(), db);
        }
        Ok(self.project_dbs.get(path).unwrap())
    }
}

#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub total_memories: usize,
    pub scope: MemoryScope,
}
