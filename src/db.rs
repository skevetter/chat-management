use std::sync::LazyLock;

use chrono::Utc;
use regex::Regex;
use rusqlite::{Connection, Result, params};
use uuid::Uuid;

static MENTION_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"@([a-zA-Z0-9_\-]+)").unwrap());

use crate::models::{
    Channel, ChannelListResult, Mention, MentionListResult, Message, MessageListResult,
};

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;

             CREATE TABLE IF NOT EXISTS channels (
                 id            INTEGER PRIMARY KEY AUTOINCREMENT,
                 name          TEXT    NOT NULL,
                 namespace     TEXT    NOT NULL DEFAULT 'default',
                 purpose       TEXT,
                 created_at    TEXT    NOT NULL,
                 message_count INTEGER NOT NULL DEFAULT 0,
                 UNIQUE (name, namespace)
             );

             CREATE TABLE IF NOT EXISTS messages (
                 id              TEXT PRIMARY KEY,
                 channel_id      INTEGER NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
                 sender          TEXT    NOT NULL,
                 content         TEXT    NOT NULL,
                 timestamp       TEXT    NOT NULL,
                 reply_to        TEXT,
                 idempotency_key TEXT
             );

             CREATE INDEX IF NOT EXISTS idx_messages_channel_ts ON messages (channel_id, timestamp);
             CREATE INDEX IF NOT EXISTS idx_messages_sender ON messages (sender);
             CREATE UNIQUE INDEX IF NOT EXISTS idx_messages_idempotency
                 ON messages (idempotency_key) WHERE idempotency_key IS NOT NULL;

             CREATE TABLE IF NOT EXISTS mentions (
                 id              INTEGER PRIMARY KEY AUTOINCREMENT,
                 message_id      TEXT    NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
                 channel_id      INTEGER NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
                 mentioned_agent TEXT    NOT NULL,
                 created_at      TEXT    NOT NULL
             );

             CREATE INDEX IF NOT EXISTS idx_mentions_agent_channel ON mentions (mentioned_agent, channel_id);
             CREATE INDEX IF NOT EXISTS idx_mentions_message ON mentions (message_id);

             CREATE TABLE IF NOT EXISTS schema_versions (
                 version    INTEGER PRIMARY KEY,
                 applied_at TEXT    NOT NULL
             );

             CREATE TRIGGER IF NOT EXISTS trg_inc_message_count
             AFTER INSERT ON messages
             BEGIN
                 UPDATE channels SET message_count = message_count + 1 WHERE id = NEW.channel_id;
             END;

             CREATE TRIGGER IF NOT EXISTS trg_dec_message_count
             AFTER DELETE ON messages
             BEGIN
                 UPDATE channels SET message_count = message_count - 1 WHERE id = OLD.channel_id;
             END;",
        )?;

        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT OR IGNORE INTO schema_versions (version, applied_at) VALUES (1, ?1)",
            params![now],
        )?;

        Ok(Self { conn })
    }

    pub fn create_channel(
        &self,
        name: &str,
        namespace: &str,
        purpose: Option<&str>,
    ) -> Result<Channel> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO channels (name, namespace, purpose, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![name, namespace, purpose, now],
        )?;
        let id = self.conn.last_insert_rowid();
        Ok(Channel {
            id,
            name: name.to_string(),
            namespace: namespace.to_string(),
            purpose: purpose.map(String::from),
            created_at: now,
            message_count: 0,
        })
    }

    pub fn list_channels(
        &self,
        namespace: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<ChannelListResult> {
        let (count_sql, query_sql, params_vec): (String, String, Vec<Box<dyn rusqlite::types::ToSql>>) =
            match namespace {
                Some(ns) => (
                    "SELECT COUNT(*) FROM channels WHERE namespace = ?1".to_string(),
                    "SELECT id, name, namespace, purpose, created_at, message_count FROM channels WHERE namespace = ?1 ORDER BY created_at DESC LIMIT ?2 OFFSET ?3".to_string(),
                    vec![
                        Box::new(ns.to_string()) as Box<dyn rusqlite::types::ToSql>,
                        Box::new(limit),
                        Box::new(offset),
                    ],
                ),
                None => (
                    "SELECT COUNT(*) FROM channels".to_string(),
                    "SELECT id, name, namespace, purpose, created_at, message_count FROM channels ORDER BY created_at DESC LIMIT ?1 OFFSET ?2".to_string(),
                    vec![
                        Box::new(limit) as Box<dyn rusqlite::types::ToSql>,
                        Box::new(offset),
                    ],
                ),
            };

        let count_params: Vec<&dyn rusqlite::types::ToSql> = match namespace {
            Some(_) => vec![params_vec[0].as_ref()],
            None => vec![],
        };
        let total: i64 = self
            .conn
            .query_row(&count_sql, count_params.as_slice(), |row| row.get(0))?;

        let query_params: Vec<&dyn rusqlite::types::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        let mut stmt = self.conn.prepare(&query_sql)?;
        let rows = stmt.query_map(query_params.as_slice(), |row| {
            Ok(Channel {
                id: row.get(0)?,
                name: row.get(1)?,
                namespace: row.get(2)?,
                purpose: row.get(3)?,
                created_at: row.get(4)?,
                message_count: row.get(5)?,
            })
        })?;

        let mut channels = Vec::new();
        for row in rows {
            channels.push(row?);
        }
        Ok(ChannelListResult {
            channels,
            total,
            limit,
            offset,
        })
    }

    pub fn get_channel(
        &self,
        name_or_id: &str,
        namespace: Option<&str>,
    ) -> Result<Option<Channel>> {
        if let Ok(id) = name_or_id.parse::<i64>() {
            let mut stmt = self.conn.prepare(
                "SELECT id, name, namespace, purpose, created_at, message_count FROM channels WHERE id = ?1",
            )?;
            let mut rows = stmt.query_map(params![id], |row| {
                Ok(Channel {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    namespace: row.get(2)?,
                    purpose: row.get(3)?,
                    created_at: row.get(4)?,
                    message_count: row.get(5)?,
                })
            })?;
            return match rows.next() {
                Some(Ok(ch)) => Ok(Some(ch)),
                Some(Err(e)) => Err(e),
                None => Ok(None),
            };
        }

        let ns = namespace.unwrap_or("default");
        let mut stmt = self.conn.prepare(
            "SELECT id, name, namespace, purpose, created_at, message_count FROM channels WHERE name = ?1 AND namespace = ?2",
        )?;
        let mut rows = stmt.query_map(params![name_or_id, ns], |row| {
            Ok(Channel {
                id: row.get(0)?,
                name: row.get(1)?,
                namespace: row.get(2)?,
                purpose: row.get(3)?,
                created_at: row.get(4)?,
                message_count: row.get(5)?,
            })
        })?;
        match rows.next() {
            Some(Ok(ch)) => Ok(Some(ch)),
            Some(Err(e)) => Err(e),
            None => Ok(None),
        }
    }

    pub fn delete_channel(&self, name_or_id: &str, namespace: Option<&str>) -> Result<Option<i64>> {
        let channel = self.get_channel(name_or_id, namespace)?;
        match channel {
            Some(ch) => {
                self.conn
                    .execute("DELETE FROM channels WHERE id = ?1", params![ch.id])?;
                Ok(Some(ch.id))
            }
            None => Ok(None),
        }
    }

    pub fn post_message(
        &self,
        channel_id: i64,
        sender: &str,
        content: &str,
        reply_to: Option<&str>,
        idempotency_key: Option<&str>,
    ) -> Result<Message> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        match self.conn.execute(
            "INSERT INTO messages (id, channel_id, sender, content, timestamp, reply_to, idempotency_key) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, channel_id, sender, content, now, reply_to, idempotency_key],
        ) {
            Ok(_) => {
                self.extract_and_store_mentions(&id, channel_id, content)?;
                Ok(Message {
                    id,
                    channel_id,
                    sender: sender.to_string(),
                    content: content.to_string(),
                    timestamp: now,
                    reply_to: reply_to.map(String::from),
                    idempotency_key: idempotency_key.map(String::from),
                })
            }
            Err(rusqlite::Error::SqliteFailure(err, _))
                if err.code == rusqlite::ErrorCode::ConstraintViolation
                    && idempotency_key.is_some() =>
            {
                let key = idempotency_key.unwrap();
                let existing = self.conn.query_row(
                    "SELECT id, channel_id, sender, content, timestamp, reply_to, idempotency_key FROM messages WHERE idempotency_key = ?1 AND channel_id = ?2",
                    params![key, channel_id],
                    |row| {
                        Ok(Message {
                            id: row.get(0)?,
                            channel_id: row.get(1)?,
                            sender: row.get(2)?,
                            content: row.get(3)?,
                            timestamp: row.get(4)?,
                            reply_to: row.get(5)?,
                            idempotency_key: row.get(6)?,
                        })
                    },
                )?;
                Ok(existing)
            }
            Err(e) => Err(e),
        }
    }

    pub fn read_messages(
        &self,
        channel_id: i64,
        limit: i64,
        offset: i64,
        since: Option<&str>,
        sender: Option<&str>,
    ) -> Result<MessageListResult> {
        let mut conditions = vec!["channel_id = ?".to_string()];
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(channel_id)];

        if let Some(ts) = since {
            conditions.push("timestamp > ?".to_string());
            param_values.push(Box::new(ts.to_string()));
        }
        if let Some(s) = sender {
            conditions.push("sender = ?".to_string());
            param_values.push(Box::new(s.to_string()));
        }

        let where_clause = conditions.join(" AND ");

        let count_sql = format!("SELECT COUNT(*) FROM messages WHERE {where_clause}");
        let count_params: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();
        let total: i64 = self
            .conn
            .query_row(&count_sql, count_params.as_slice(), |row| row.get(0))?;

        let query_sql = format!(
            "SELECT id, channel_id, sender, content, timestamp, reply_to, idempotency_key FROM messages WHERE {where_clause} ORDER BY timestamp ASC LIMIT ? OFFSET ?"
        );
        param_values.push(Box::new(limit));
        param_values.push(Box::new(offset));

        let query_params: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();
        let mut stmt = self.conn.prepare(&query_sql)?;
        let rows = stmt.query_map(query_params.as_slice(), |row| {
            Ok(Message {
                id: row.get(0)?,
                channel_id: row.get(1)?,
                sender: row.get(2)?,
                content: row.get(3)?,
                timestamp: row.get(4)?,
                reply_to: row.get(5)?,
                idempotency_key: row.get(6)?,
            })
        })?;

        let mut messages = Vec::new();
        for row in rows {
            messages.push(row?);
        }
        Ok(MessageListResult {
            messages,
            total,
            limit,
            offset,
        })
    }

    pub fn inspect_channel(
        &self,
        name_or_id: &str,
        namespace: Option<&str>,
    ) -> Result<Option<Channel>> {
        self.get_channel(name_or_id, namespace)
    }

    pub fn list_mentions(
        &self,
        agent: Option<&str>,
        channel_id: Option<i64>,
        limit: i64,
        offset: i64,
    ) -> Result<MentionListResult> {
        let mut conditions: Vec<String> = Vec::new();
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(a) = agent {
            conditions.push("mentioned_agent = ?".to_string());
            param_values.push(Box::new(a.to_string()));
        }
        if let Some(cid) = channel_id {
            conditions.push("channel_id = ?".to_string());
            param_values.push(Box::new(cid));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", conditions.join(" AND "))
        };

        let count_sql = format!("SELECT COUNT(*) FROM mentions{where_clause}");
        let count_params: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();
        let total: i64 = self
            .conn
            .query_row(&count_sql, count_params.as_slice(), |row| row.get(0))?;

        let query_sql = format!(
            "SELECT id, message_id, channel_id, mentioned_agent, created_at FROM mentions{where_clause} ORDER BY created_at DESC LIMIT ? OFFSET ?"
        );
        param_values.push(Box::new(limit));
        param_values.push(Box::new(offset));

        let query_params: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();
        let mut stmt = self.conn.prepare(&query_sql)?;
        let rows = stmt.query_map(query_params.as_slice(), |row| {
            Ok(Mention {
                id: row.get(0)?,
                message_id: row.get(1)?,
                channel_id: row.get(2)?,
                mentioned_agent: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;

        let mut mentions = Vec::new();
        for row in rows {
            mentions.push(row?);
        }
        Ok(MentionListResult {
            mentions,
            total,
            limit,
            offset,
        })
    }

    pub fn extract_and_store_mentions(
        &self,
        message_id: &str,
        channel_id: i64,
        content: &str,
    ) -> Result<Vec<Mention>> {
        let now = Utc::now().to_rfc3339();

        let mut seen = std::collections::HashSet::new();
        let mut mentions = Vec::new();

        for cap in MENTION_RE.captures_iter(content) {
            let agent = cap[1].to_string();
            if !seen.insert(agent.clone()) {
                continue;
            }
            self.conn.execute(
                "INSERT INTO mentions (message_id, channel_id, mentioned_agent, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![message_id, channel_id, agent, now],
            )?;
            let id = self.conn.last_insert_rowid();
            mentions.push(Mention {
                id,
                message_id: message_id.to_string(),
                channel_id,
                mentioned_agent: agent,
                created_at: now.clone(),
            });
        }
        Ok(mentions)
    }
}
