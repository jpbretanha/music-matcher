use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool, sqlite::SqliteConnectOptions};
use std::str::FromStr;

use crate::fingerprint::{AudioFingerprint, calculate_similarity};

#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

#[derive(Debug, Serialize, Deserialize)]
struct SongRecord {
    id: i64,
    title: String,
    artist: String,
    fingerprint_data: String,
    duration: f64,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        let options = SqliteConnectOptions::from_str(database_url)?
            .create_if_missing(true);
        
        let pool = SqlitePool::connect_with(options).await?;
        
        Ok(Database { pool })
    }

    pub async fn init(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS songs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                artist TEXT NOT NULL,
                fingerprint_data TEXT NOT NULL,
                duration REAL NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_songs_artist ON songs(artist);
            CREATE INDEX IF NOT EXISTS idx_songs_title ON songs(title);
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn add_song(
        &self,
        title: &str,
        artist: &str,
        fingerprint: &AudioFingerprint,
    ) -> Result<i64> {
        let fingerprint_json = serde_json::to_string(fingerprint)?;

        let result = sqlx::query(
            r#"
            INSERT INTO songs (title, artist, fingerprint_data, duration)
            VALUES (?1, ?2, ?3, ?4)
            "#,
        )
        .bind(title)
        .bind(artist)
        .bind(&fingerprint_json)
        .bind(fingerprint.duration)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn find_match(
        &self,
        query_fingerprint: &AudioFingerprint,
    ) -> Result<Option<(i64, String, String, f64)>> {
        let rows = sqlx::query(
            r#"
            SELECT id, title, artist, fingerprint_data, duration
            FROM songs
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut best_match = None;
        let mut best_similarity = 0.0;

        for row in rows {
            let id: i64 = row.get("id");
            let title: String = row.get("title");
            let artist: String = row.get("artist");
            let fingerprint_data: String = row.get("fingerprint_data");

            if let Ok(stored_fingerprint) = serde_json::from_str::<AudioFingerprint>(&fingerprint_data) {
                let similarity = calculate_similarity(query_fingerprint, &stored_fingerprint);
                
                if similarity > best_similarity && similarity > 0.3 {
                    best_similarity = similarity;
                    best_match = Some((id, title, artist, similarity));
                }
            }
        }

        Ok(best_match)
    }

    pub async fn find_all_matches(
        &self,
        query_fingerprint: &AudioFingerprint,
    ) -> Result<Vec<(i64, String, String, f64)>> {
        let rows = sqlx::query(
            r#"
            SELECT id, title, artist, fingerprint_data, duration
            FROM songs
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut matches = Vec::new();

        for row in rows {
            let id: i64 = row.get("id");
            let title: String = row.get("title");
            let artist: String = row.get("artist");
            let fingerprint_data: String = row.get("fingerprint_data");

            if let Ok(stored_fingerprint) = serde_json::from_str::<AudioFingerprint>(&fingerprint_data) {
                let similarity = calculate_similarity(query_fingerprint, &stored_fingerprint);
                
                if similarity > 0.3 {
                    matches.push((id, title, artist, similarity));
                }
            }
        }

        // Sort by similarity in descending order
        matches.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));

        Ok(matches)
    }

    pub async fn get_all_songs(&self) -> Result<Vec<(i64, String, String)>> {
        let rows = sqlx::query(
            r#"
            SELECT id, title, artist
            FROM songs
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let songs = rows
            .into_iter()
            .map(|row| {
                let id: i64 = row.get("id");
                let title: String = row.get("title");
                let artist: String = row.get("artist");
                (id, title, artist)
            })
            .collect();

        Ok(songs)
    }

    pub async fn delete_song(&self, song_id: i64) -> Result<bool> {
        let result = sqlx::query(
            r#"
            DELETE FROM songs WHERE id = ?1
            "#,
        )
        .bind(song_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}