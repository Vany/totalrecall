use crate::{Memory, MemoryScope};
use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tracing::{debug, info};

pub struct MemoryStore {
    session: HashMap<String, Memory>,
    global_db: Option<Arc<Mutex<Connection>>>,
    project_dbs: HashMap<PathBuf, Arc<Mutex<Connection>>>,
    global_db_path: PathBuf,
}

impl MemoryStore {
    pub fn new(global_db_path: PathBuf) -> Result<Self> {
        let global_db = if global_db_path.exists()
            || global_db_path.parent().map(|p| p.exists()).unwrap_or(false)
        {
            if let Some(parent) = global_db_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let conn =
                Connection::open(&global_db_path).context("Failed to open global database")?;

            // Enable WAL mode for concurrent access
            conn.execute("PRAGMA journal_mode=WAL", [])?;
            conn.execute("PRAGMA synchronous=NORMAL", [])?;

            // Create table if it doesn't exist
            conn.execute(
                "CREATE TABLE IF NOT EXISTS memories (
                    id TEXT PRIMARY KEY,
                    content TEXT NOT NULL,
                    scope TEXT NOT NULL,
                    metadata TEXT NOT NULL,
                    created_at INTEGER NOT NULL,
                    updated_at INTEGER NOT NULL
                )",
                [],
            )?;

            Some(Arc::new(Mutex::new(conn)))
        } else {
            None
        };

        info!(
            "Initialized MemoryStore with global DB at {:?}",
            global_db_path
        );

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
                let conn = db.lock().unwrap();
                let metadata_json = serde_json::to_string(&memory.metadata)?;

                conn.execute(
                    "INSERT OR REPLACE INTO memories (id, content, scope, metadata, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        memory.id,
                        memory.content,
                        "global",
                        metadata_json,
                        memory.created_at.timestamp(),
                        memory.updated_at.timestamp(),
                    ],
                )?;
            }
            MemoryScope::Project { path } => {
                let db = self.get_or_create_project_db(path)?;
                let conn = db.lock().unwrap();
                let metadata_json = serde_json::to_string(&memory.metadata)?;
                let path_str = path.to_string_lossy();

                conn.execute(
                    "INSERT OR REPLACE INTO memories (id, content, scope, metadata, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        memory.id,
                        memory.content,
                        path_str.as_ref(),
                        metadata_json,
                        memory.created_at.timestamp(),
                        memory.updated_at.timestamp(),
                    ],
                )?;
            }
        }

        Ok(())
    }

    pub fn get(&self, id: &str, scope: &MemoryScope) -> Result<Option<Memory>> {
        match scope {
            MemoryScope::Session => Ok(self.session.get(id).cloned()),
            MemoryScope::Global => {
                if let Some(db) = &self.global_db {
                    let conn = db.lock().unwrap();
                    let mut stmt = conn.prepare(
                        "SELECT id, content, scope, metadata, created_at, updated_at
                         FROM memories WHERE id = ?1",
                    )?;

                    let memory = stmt
                        .query_row([id], |row| {
                            Ok(Memory {
                                id: row.get(0)?,
                                content: row.get(1)?,
                                scope: MemoryScope::Global,
                                metadata: serde_json::from_str(&row.get::<_, String>(3)?)
                                    .unwrap_or_default(),
                                created_at: chrono::DateTime::from_timestamp(row.get(4)?, 0)
                                    .unwrap(),
                                updated_at: chrono::DateTime::from_timestamp(row.get(5)?, 0)
                                    .unwrap(),
                            })
                        })
                        .optional()?;

                    Ok(memory)
                } else {
                    Ok(None)
                }
            }
            MemoryScope::Project { path } => {
                if let Some(db) = self.project_dbs.get(path) {
                    let conn = db.lock().unwrap();
                    let mut stmt = conn.prepare(
                        "SELECT id, content, scope, metadata, created_at, updated_at
                         FROM memories WHERE id = ?1",
                    )?;

                    let memory = stmt
                        .query_row([id], |row| {
                            Ok(Memory {
                                id: row.get(0)?,
                                content: row.get(1)?,
                                scope: MemoryScope::Project { path: path.clone() },
                                metadata: serde_json::from_str(&row.get::<_, String>(3)?)
                                    .unwrap_or_default(),
                                created_at: chrono::DateTime::from_timestamp(row.get(4)?, 0)
                                    .unwrap(),
                                updated_at: chrono::DateTime::from_timestamp(row.get(5)?, 0)
                                    .unwrap(),
                            })
                        })
                        .optional()?;

                    Ok(memory)
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
                    let conn = db.lock().unwrap();
                    let affected = conn.execute("DELETE FROM memories WHERE id = ?1", [id])?;
                    Ok(affected > 0)
                } else {
                    Ok(false)
                }
            }
            MemoryScope::Project { path } => {
                if let Some(db) = self.project_dbs.get(path) {
                    let conn = db.lock().unwrap();
                    let affected = conn.execute("DELETE FROM memories WHERE id = ?1", [id])?;
                    Ok(affected > 0)
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
                    let conn = db.lock().unwrap();
                    let mut stmt = conn.prepare(
                        "SELECT id, content, scope, metadata, created_at, updated_at
                         FROM memories ORDER BY created_at DESC LIMIT ?1 OFFSET ?2",
                    )?;

                    let rows = stmt.query_map(params![limit, offset], |row| {
                        Ok(Memory {
                            id: row.get(0)?,
                            content: row.get(1)?,
                            scope: MemoryScope::Global,
                            metadata: serde_json::from_str(&row.get::<_, String>(3)?)
                                .unwrap_or_default(),
                            created_at: chrono::DateTime::from_timestamp(row.get(4)?, 0).unwrap(),
                            updated_at: chrono::DateTime::from_timestamp(row.get(5)?, 0).unwrap(),
                        })
                    })?;

                    for row in rows {
                        memories.push(row?);
                    }
                }
            }
            MemoryScope::Project { path } => {
                if let Some(db) = self.project_dbs.get(path) {
                    let conn = db.lock().unwrap();
                    let mut stmt = conn.prepare(
                        "SELECT id, content, scope, metadata, created_at, updated_at
                         FROM memories ORDER BY created_at DESC LIMIT ?1 OFFSET ?2",
                    )?;

                    let rows = stmt.query_map(params![limit, offset], |row| {
                        Ok(Memory {
                            id: row.get(0)?,
                            content: row.get(1)?,
                            scope: MemoryScope::Project { path: path.clone() },
                            metadata: serde_json::from_str(&row.get::<_, String>(3)?)
                                .unwrap_or_default(),
                            created_at: chrono::DateTime::from_timestamp(row.get(4)?, 0).unwrap(),
                            updated_at: chrono::DateTime::from_timestamp(row.get(5)?, 0).unwrap(),
                        })
                    })?;

                    for row in rows {
                        memories.push(row?);
                    }
                }
            }
        }

        Ok(memories)
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
                    let conn = db.lock().unwrap();
                    let count: i64 =
                        conn.query_row("SELECT COUNT(*) FROM memories", [], |row| row.get(0))?;
                    count as usize
                } else {
                    0
                }
            }
            MemoryScope::Project { path } => {
                if let Some(db) = self.project_dbs.get(path) {
                    let conn = db.lock().unwrap();
                    let count: i64 =
                        conn.query_row("SELECT COUNT(*) FROM memories", [], |row| row.get(0))?;
                    count as usize
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

    fn get_or_create_global_db(&mut self) -> Result<&Arc<Mutex<Connection>>> {
        if self.global_db.is_none() {
            if let Some(parent) = self.global_db_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let conn = Connection::open(&self.global_db_path)?;
            conn.execute("PRAGMA journal_mode=WAL", [])?;
            conn.execute("PRAGMA synchronous=NORMAL", [])?;
            conn.execute(
                "CREATE TABLE IF NOT EXISTS memories (
                    id TEXT PRIMARY KEY,
                    content TEXT NOT NULL,
                    scope TEXT NOT NULL,
                    metadata TEXT NOT NULL,
                    created_at INTEGER NOT NULL,
                    updated_at INTEGER NOT NULL
                )",
                [],
            )?;

            self.global_db = Some(Arc::new(Mutex::new(conn)));
        }
        Ok(self.global_db.as_ref().unwrap())
    }

    fn get_or_create_project_db(&mut self, path: &Path) -> Result<&Arc<Mutex<Connection>>> {
        if !self.project_dbs.contains_key(path) {
            let db_path = path.join(".rag-mcp").join("data.db");
            if let Some(parent) = db_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let conn = Connection::open(&db_path)?;
            conn.execute("PRAGMA journal_mode=WAL", [])?;
            conn.execute("PRAGMA synchronous=NORMAL", [])?;
            conn.execute(
                "CREATE TABLE IF NOT EXISTS memories (
                    id TEXT PRIMARY KEY,
                    content TEXT NOT NULL,
                    scope TEXT NOT NULL,
                    metadata TEXT NOT NULL,
                    created_at INTEGER NOT NULL,
                    updated_at INTEGER NOT NULL
                )",
                [],
            )?;

            self.project_dbs
                .insert(path.to_path_buf(), Arc::new(Mutex::new(conn)));
        }
        Ok(self.project_dbs.get(path).unwrap())
    }
}

#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub total_memories: usize,
    pub scope: MemoryScope,
}
