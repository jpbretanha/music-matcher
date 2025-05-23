# Rust Music Matching Service

A Rust-based HTTP service that receives audio files and matches them against a SQLite vector database of known songs using audio fingerprinting.

## Features

- HTTP API for audio file upload and matching
- Audio fingerprinting using spectral peak analysis
- SQLite database with vector similarity matching
- Support for WAV audio files
- RESTful endpoints for adding and matching songs

## API Endpoints

### `GET /`
Health check endpoint.

### `POST /match`
Match an uploaded audio file against the database.
- Content-Type: `multipart/form-data`
- Field: `audio` (WAV file)
- Returns: JSON with match results including confidence score

### `POST /add-song`
Add a new song to the database.
- Content-Type: `multipart/form-data`
- Fields: 
  - `audio` (WAV file)
  - `title` (string)
  - `artist` (string)
- Returns: JSON with success status and song ID

## Usage

1. Build and run:
```bash
cargo run
```

2. The service runs on `http://127.0.0.1:3000`

3. Add a song:
```bash
curl -X POST http://127.0.0.1:3000/add-song \
  -F "audio=@song.wav" \
  -F "title=Song Title" \
  -F "artist=Artist Name"
```

4. Match an audio file:
```bash
curl -X POST http://127.0.0.1:3000/match \
  -F "audio=@query.wav"
```

## Technical Details

- Uses spectral peak analysis for audio fingerprinting
- Generates hash-based fingerprints for efficient matching
- Stores fingerprints as JSON in SQLite database
- Similarity threshold of 0.3 for matches
- Supports downsampling to 11kHz for processing