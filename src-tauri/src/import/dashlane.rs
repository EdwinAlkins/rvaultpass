//! Dashlane credentials.csv importer
//!
//! Expects the credentials export from a Dashlane ZIP (first header column: `username`).
//! Columns: username, username2, username3, title, password, note, url, category, otpUrl[, otpSecret]

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::db::operations;

/// Result of a Dashlane CSV import run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportSummary {
    pub imported: u32,
    pub skipped: u32,
    pub errors: Vec<String>,
}

/// One parsed credentials row (before DB insert)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DashlaneCredential {
    pub title: String,
    pub url: Option<String>,
    pub username: Option<String>,
    pub username2: Option<String>,
    pub username3: Option<String>,
    pub password: String,
    pub notes: Option<String>,
    pub totp_secret: Option<String>,
    pub category: Option<String>,
}

fn empty_to_none(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

fn get_field(record: &csv::StringRecord, headers: &csv::StringRecord, name: &str) -> String {
    headers
        .iter()
        .position(|h| h.trim() == name)
        .and_then(|i| record.get(i).map(|s| s.to_string()))
        .unwrap_or_default()
}

fn resolve_title(title: &str, url: &Option<String>) -> String {
    if let Some(t) = empty_to_none(title) {
        return t;
    }
    if let Some(u) = url {
        return u.clone();
    }
    "Untitled".to_string()
}

/// Prefer otpUrl, fall back to otpSecret (both Dashlane columns).
fn resolve_totp(otp_url: &str, otp_secret: &str) -> Option<String> {
    empty_to_none(otp_url).or_else(|| empty_to_none(otp_secret))
}

/// Parse Dashlane credentials CSV content into credential records.
///
/// Validates that the first header column is `username` (Dashlane credentials format).
pub fn parse_credentials_csv(content: &str) -> Result<Vec<DashlaneCredential>, String> {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(content.as_bytes());

    let headers = reader
        .headers()
        .map_err(|e| format!("Failed to read CSV headers: {}", e))?
        .clone();

    if headers.is_empty() {
        return Err("CSV file has no headers".to_string());
    }

    let first = headers.get(0).unwrap_or("").trim();
    if first != "username" {
        return Err(format!(
            "Not a Dashlane credentials.csv (expected first column 'username', got '{}'). \
             Extract credentials.csv from the Dashlane export ZIP.",
            first
        ));
    }

    let mut credentials = Vec::new();

    for (row_idx, result) in reader.records().enumerate() {
        let record = match result {
            Ok(r) => r,
            Err(e) => {
                return Err(format!("Failed to parse CSV row {}: {}", row_idx + 2, e));
            }
        };

        let username = empty_to_none(&get_field(&record, &headers, "username"));
        let username2 = empty_to_none(&get_field(&record, &headers, "username2"));
        let username3 = empty_to_none(&get_field(&record, &headers, "username3"));
        let password = get_field(&record, &headers, "password");
        let url = empty_to_none(&get_field(&record, &headers, "url"));
        let title_raw = get_field(&record, &headers, "title");
        let note = get_field(&record, &headers, "note");
        let category = empty_to_none(&get_field(&record, &headers, "category"));
        let otp_url = get_field(&record, &headers, "otpUrl");
        let otp_secret = get_field(&record, &headers, "otpSecret");

        credentials.push(DashlaneCredential {
            title: resolve_title(&title_raw, &url),
            url,
            username,
            username2,
            username3,
            password,
            notes: empty_to_none(&note),
            totp_secret: resolve_totp(&otp_url, &otp_secret),
            category,
        });
    }

    Ok(credentials)
}

/// Import a Dashlane credentials.csv file into the currently open vault.
/// The caller supplies the transaction. Any row failure aborts the whole import.
pub fn import_dashlane_csv(conn: &rusqlite::Connection, path: &Path) -> Result<ImportSummary, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read CSV file: {}", e))?;

    let credentials = parse_credentials_csv(&content)?;

    let mut folder_cache: HashMap<String, String> = HashMap::new();
    if let Ok(existing) = operations::list_folders(conn) {
        for folder in existing {
            folder_cache.insert(folder.name.clone(), folder.id);
        }
    }

    let mut imported: u32 = 0;
    let errors: Vec<String> = Vec::new();

    for (i, cred) in credentials.iter().enumerate() {
        let row_num = i + 2;

        let folder_id = if let Some(ref cat) = cred.category {
            if let Some(id) = folder_cache.get(cat) {
                Some(id.clone())
            } else {
                let folder = operations::create_folder(conn, cat.clone(), None).map_err(|e| {
                    format!("Row {row_num}: failed to create folder '{cat}': {e}")
                })?;
                folder_cache.insert(cat.clone(), folder.id.clone());
                Some(folder.id)
            }
        } else {
            None
        };

        operations::create_entry(
            conn,
            folder_id,
            cred.title.clone(),
            cred.url.clone(),
            cred.username.clone(),
            cred.username2.clone(),
            cred.username3.clone(),
            cred.password.clone(),
            cred.notes.clone(),
            cred.totp_secret.clone(),
            false,
            None,
        )
        .map_err(|e| format!("Row {row_num}: failed to create entry '{}': {e}", cred.title))?;
        imported += 1;
    }

    Ok(ImportSummary {
        imported,
        skipped: 0,
        errors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_CSV: &str = "\
username,username2,username3,title,password,note,url,category,otpUrl
alice@example.com,,,Gmail,secret123,Personal note,https://mail.google.com,Email,
bob,,,GitHub,gh-pass,,https://github.com,Dev,otpauth://totp/GitHub?secret=JBSWY3DPEHPK3PXP
,,,,pass4,,https://notitle.example,,
user2@x.com,alt1,alt2,MultiUser,pw,Main note,https://multi.example,Work,
";

    #[test]
    fn parse_credentials_maps_fields() {
        let creds = parse_credentials_csv(SAMPLE_CSV).expect("parse ok");
        assert_eq!(creds.len(), 4);

        assert_eq!(creds[0].title, "Gmail");
        assert_eq!(creds[0].username.as_deref(), Some("alice@example.com"));
        assert!(creds[0].username2.is_none());
        assert_eq!(creds[0].password, "secret123");
        assert_eq!(creds[0].notes.as_deref(), Some("Personal note"));
        assert_eq!(creds[0].url.as_deref(), Some("https://mail.google.com"));
        assert_eq!(creds[0].category.as_deref(), Some("Email"));
        assert!(creds[0].totp_secret.is_none());

        assert_eq!(creds[1].title, "GitHub");
        assert_eq!(
            creds[1].totp_secret.as_deref(),
            Some("otpauth://totp/GitHub?secret=JBSWY3DPEHPK3PXP")
        );
        assert_eq!(creds[1].category.as_deref(), Some("Dev"));

        assert_eq!(creds[2].title, "https://notitle.example");
        assert!(creds[2].category.is_none());

        assert_eq!(creds[3].title, "MultiUser");
        assert_eq!(creds[3].username2.as_deref(), Some("alt1"));
        assert_eq!(creds[3].username3.as_deref(), Some("alt2"));
        assert_eq!(creds[3].notes.as_deref(), Some("Main note"));
    }

    #[test]
    fn parse_prefers_otp_url_over_secret() {
        let csv = "\
username,title,password,note,url,category,otpSecret,otpUrl
user,App,pass,,,email,SECRET,otpauth://totp/App?secret=SECRET
";
        let creds = parse_credentials_csv(csv).unwrap();
        assert_eq!(
            creds[0].totp_secret.as_deref(),
            Some("otpauth://totp/App?secret=SECRET")
        );
    }

    #[test]
    fn parse_rejects_non_credentials_csv() {
        let csv = "title,note\nMy note,hello\n";
        let err = parse_credentials_csv(csv).unwrap_err();
        assert!(err.contains("credentials.csv"));
    }

    #[test]
    fn untitled_when_no_title_or_url() {
        let csv = "username,title,password,note,url,category\nuser,,,note,,\n";
        let creds = parse_credentials_csv(csv).unwrap();
        assert_eq!(creds[0].title, "Untitled");
    }
}
