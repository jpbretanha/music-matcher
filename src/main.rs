use axum::{
    extract::Multipart,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tracing::{info, error};

mod audio;
mod database;
mod fingerprint;

use database::Database;

#[derive(Serialize, Deserialize)]
struct MatchResponse {
    matched: bool,
    song_id: Option<i64>,
    title: Option<String>,
    artist: Option<String>,
    confidence: Option<f64>,
}

#[derive(Clone)]
struct AppState {
    db: Database,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let db = Database::new("songs.db").await?;
    db.init().await?;

    let state = AppState { db };

    let app = Router::new()
        .route("/", get(health_check))
        .route("/match", post(match_audio))
        .route("/add-song", post(add_song))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("Server running on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> &'static str {
    "Audio matching service is running"
}

async fn match_audio(
    axum::extract::State(state): axum::extract::State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<MatchResponse>, StatusCode> {
    while let Some(field) = multipart.next_field().await.map_err(|_| StatusCode::BAD_REQUEST)? {
        if field.name() == Some("audio") {
            let data = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;
            
            match process_audio_match(&state.db, &data).await {
                Ok(response) => return Ok(Json(response)),
                Err(e) => {
                    error!("Audio processing error: {}", e);
                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                }
            }
        }
    }
    
    Err(StatusCode::BAD_REQUEST)
}

async fn add_song(
    axum::extract::State(state): axum::extract::State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut audio_data = None;
    let mut title = None;
    let mut artist = None;

    while let Some(field) = multipart.next_field().await.map_err(|_| StatusCode::BAD_REQUEST)? {
        match field.name() {
            Some("audio") => {
                audio_data = Some(field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?);
            }
            Some("title") => {
                title = Some(field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?);
            }
            Some("artist") => {
                artist = Some(field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?);
            }
            _ => {}
        }
    }

    let audio_data = audio_data.ok_or(StatusCode::BAD_REQUEST)?;
    let title = title.ok_or(StatusCode::BAD_REQUEST)?;
    let artist = artist.ok_or(StatusCode::BAD_REQUEST)?;

    match process_add_song(&state.db, &audio_data, &title, &artist).await {
        Ok(song_id) => Ok(Json(serde_json::json!({
            "success": true,
            "song_id": song_id
        }))),
        Err(e) => {
            error!("Add song error: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn process_audio_match(db: &Database, audio_data: &[u8]) -> anyhow::Result<MatchResponse> {
    let audio_samples = audio::decode_audio(audio_data)?;
    let fingerprint = fingerprint::generate_fingerprint(&audio_samples)?;
    
    if let Some((song_id, title, artist, confidence)) = db.find_match(&fingerprint).await? {
        Ok(MatchResponse {
            matched: true,
            song_id: Some(song_id),
            title: Some(title),
            artist: Some(artist),
            confidence: Some(confidence),
        })
    } else {
        Ok(MatchResponse {
            matched: false,
            song_id: None,
            title: None,
            artist: None,
            confidence: None,
        })
    }
}

async fn process_add_song(
    db: &Database,
    audio_data: &[u8],
    title: &str,
    artist: &str,
) -> anyhow::Result<i64> {
    let audio_samples = audio::decode_audio(audio_data)?;
    let fingerprint = fingerprint::generate_fingerprint(&audio_samples)?;
    
    let song_id = db.add_song(title, artist, &fingerprint).await?;
    Ok(song_id)
}