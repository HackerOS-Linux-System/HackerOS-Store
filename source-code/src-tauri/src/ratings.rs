use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::RatingInfo;
use crate::security::validate_pkg_token;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LocalReview {
    pub stars: u8,
    pub comment: Option<String>,
    pub timestamp: String,
}

/// On-disk shape: `"source:package_id" -> [reviews...]`. A person can
/// re-rate the same package; we keep the full small history (capped) so a
/// changed mind is visible, and always aggregate over all stored reviews
/// for that key.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
struct RatingsStore(HashMap<String, Vec<LocalReview>>);

/// Cap per-package review history so a person spamming the submit button
/// can't grow the file unboundedly; keeps only the most recent entries.
const MAX_REVIEWS_PER_PACKAGE: usize = 50;

fn ratings_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
    PathBuf::from(format!("{home}/.hackeros/store/ratings.json"))
}

fn load() -> RatingsStore {
    std::fs::read_to_string(ratings_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save(store: &RatingsStore) -> Result<(), String> {
    let path = ratings_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

fn key(source: &str, package_id: &str) -> String {
    format!("{source}:{package_id}")
}

fn aggregate(reviews: &[LocalReview]) -> Option<RatingInfo> {
    if reviews.is_empty() {
        return None;
    }
    let count = reviews.len() as u32;
    let sum: u32 = reviews.iter().map(|r| r.stars as u32).sum();
    Some(RatingInfo {
        average: sum as f32 / count as f32,
        count,
    })
}

fn now_iso() -> String {
    // Lightweight RFC3339-ish timestamp without pulling in a datetime crate:
    // seconds-since-epoch is enough for display/sorting purposes here.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("epoch:{secs}")
}

/// Returns the aggregated local rating for a package, if any reviews exist.
/// Best-effort / read-only: never errors, just returns `None` on any
/// problem (missing file, corrupt JSON, unknown key).
pub fn get_local_rating(source: &str, package_id: &str) -> Option<RatingInfo> {
    let package_id = validate_pkg_token(package_id).ok()?;
    let store = load();
    store.0.get(&key(source, &package_id)).and_then(|r| aggregate(r))
}

/// Submits (adds) a 1-5 star rating with an optional short comment for a
/// package under a given source, and returns the freshly recomputed
/// aggregate so the UI can update immediately without a second round trip.
pub fn submit_rating(
    source: &str,
    package_id: &str,
    stars: u8,
    comment: Option<String>,
) -> Result<RatingInfo, String> {
    let package_id = validate_pkg_token(package_id)?;
    if !(1..=5).contains(&stars) {
        return Err("Rating must be between 1 and 5 stars.".to_string());
    }
    // Keep comments short and newline-free in storage; the UI is a plain
    // text field, not rich text, so this is just hygiene, not a security
    // boundary (the value is only ever displayed as text, never executed).
    let comment = comment.map(|c| {
        let trimmed = c.trim();
        let clipped: String = trimmed.chars().take(500).collect();
        clipped
    }).filter(|c| !c.is_empty());

    let mut store = load();
    let k = key(source, &package_id);
    let entry = store.0.entry(k).or_default();
    entry.push(LocalReview { stars, comment, timestamp: now_iso() });
    if entry.len() > MAX_REVIEWS_PER_PACKAGE {
        let drop = entry.len() - MAX_REVIEWS_PER_PACKAGE;
        entry.drain(0..drop);
    }
    let result = aggregate(entry).unwrap_or(RatingInfo { average: stars as f32, count: 1 });
    save(&store)?;
    Ok(result)
}

/// Returns the raw review list (most recent first) for the detail view's
/// "recent reviews" section.
pub fn get_reviews(source: &str, package_id: &str) -> Vec<LocalReview> {
    let Ok(package_id) = validate_pkg_token(package_id) else { return vec![] };
    let store = load();
    let mut reviews = store.0.get(&key(source, &package_id)).cloned().unwrap_or_default();
    reviews.reverse();
    reviews
}
