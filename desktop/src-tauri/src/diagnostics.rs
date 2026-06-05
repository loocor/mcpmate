use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::OnceLock,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use mcpmate::common::MCPMatePaths;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const FRONTEND_DIAGNOSTICS_FILE_NAME: &str = "frontend-diagnostics.jsonl";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DiagnosticEventPayload {
    level: String,
    source: String,
    message: String,
    #[serde(default)]
    data: Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DiagnosticsExportResponse {
    pub export_path: String,
    pub file_count: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredDiagnosticEvent<'a> {
    recorded_at_unix_ms: u128,
    level: &'a str,
    source: &'a str,
    message: &'a str,
    data: &'a Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticsManifestEntry {
    source_path: String,
    included_as: String,
    bytes: u64,
}

pub(crate) fn record_frontend_diagnostic_event(
    paths: &MCPMatePaths,
    payload: &DiagnosticEventPayload,
) -> Result<PathBuf> {
    let logs_dir = paths.logs_dir();
    fs::create_dir_all(&logs_dir).with_context(|| {
        format!(
            "failed to create diagnostics logs dir {}",
            logs_dir.display()
        )
    })?;
    let path = logs_dir.join(FRONTEND_DIAGNOSTICS_FILE_NAME);
    let redactor = DiagnosticsRedactor::new(paths);
    let data = redactor.redact_json(payload.data.clone());
    let level = redactor.redact_text(payload.level.as_str());
    let source = redactor.redact_text(payload.source.as_str());
    let message = redactor.redact_text(payload.message.as_str());
    let event = StoredDiagnosticEvent {
        recorded_at_unix_ms: unix_millis()?,
        level: level.as_str(),
        source: source.as_str(),
        message: message.as_str(),
        data: &data,
    };
    let line =
        serde_json::to_string(&event).context("failed to serialize frontend diagnostic event")?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("failed to open frontend diagnostics log {}", path.display()))?;
    let mut entry = line.into_bytes();
    entry.push(b'\n');
    file.write_all(&entry).with_context(|| {
        format!(
            "failed to append frontend diagnostics log {}",
            path.display()
        )
    })?;
    Ok(path)
}

pub(crate) fn export_diagnostics_bundle(
    paths: &MCPMatePaths,
    destination_root: &Path,
    runtime_metadata: Value,
    snapshot: Value,
) -> Result<DiagnosticsExportResponse> {
    let timestamp = current_timestamp()?;
    let export_dir = destination_root.join(diagnostics_export_dir_name(&timestamp));
    let logs_export_dir = export_dir.join("logs");
    fs::create_dir_all(&logs_export_dir).with_context(|| {
        format!(
            "failed to create diagnostics export dir {}",
            logs_export_dir.display()
        )
    })?;

    let redactor = DiagnosticsRedactor::new(paths);
    let mut entries = Vec::new();
    for source in collect_diagnostic_sources(&paths.logs_dir())? {
        let file_name = source
            .file_name()
            .context("diagnostic source path has no file name")?;
        let target = logs_export_dir.join(file_name);
        let included_name = file_name.to_string_lossy();
        let content = fs::read(&source)
            .with_context(|| format!("failed to read diagnostic source {}", source.display()))?;
        let redacted = redactor.redact_text(&String::from_utf8_lossy(&content));
        fs::write(&target, redacted.as_bytes())
            .with_context(|| format!("failed to write diagnostic source {}", target.display()))?;
        let bytes = redacted.len() as u64;
        entries.push(DiagnosticsManifestEntry {
            source_path: format!("[mcpmate-logs-dir]/{included_name}"),
            included_as: format!("logs/{included_name}"),
            bytes,
        });
    }

    let runtime_path = export_dir.join("runtime.json");
    let runtime_metadata = redactor.redact_json(runtime_metadata);
    fs::write(
        &runtime_path,
        serde_json::to_vec_pretty(&runtime_metadata)
            .context("failed to serialize runtime diagnostics")?,
    )
    .with_context(|| {
        format!(
            "failed to write runtime diagnostics {}",
            runtime_path.display()
        )
    })?;

    let snapshot_path = export_dir.join("snapshot.json");
    let snapshot = redactor.redact_json(snapshot);
    fs::write(
        &snapshot_path,
        serde_json::to_vec_pretty(&snapshot).context("failed to serialize diagnostics snapshot")?,
    )
    .with_context(|| {
        format!(
            "failed to write diagnostics snapshot {}",
            snapshot_path.display()
        )
    })?;

    let manifest = json!({
        "version": 1,
        "generatedAt": timestamp,
        "baseDir": "[mcpmate-data-dir]",
        "logsDir": "[mcpmate-logs-dir]",
        "snapshot": "snapshot.json",
        "runtime": "runtime.json",
        "files": entries,
    });
    let manifest_path = export_dir.join("manifest.json");
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).context("failed to serialize diagnostics manifest")?,
    )
    .with_context(|| {
        format!(
            "failed to write diagnostics manifest {}",
            manifest_path.display()
        )
    })?;

    Ok(DiagnosticsExportResponse {
        export_path: export_dir.display().to_string(),
        file_count: entries.len() + 3,
    })
}

fn collect_diagnostic_sources(logs_dir: &Path) -> Result<Vec<PathBuf>> {
    if !logs_dir.exists() {
        return Ok(Vec::new());
    }

    let canonical_logs_dir = logs_dir.canonicalize().with_context(|| {
        format!(
            "failed to resolve diagnostics logs dir {}",
            logs_dir.display()
        )
    })?;
    let mut sources = Vec::new();
    for entry in fs::read_dir(logs_dir)
        .with_context(|| format!("failed to read diagnostics logs dir {}", logs_dir.display()))?
    {
        let entry = entry.with_context(|| {
            format!(
                "failed to read diagnostics logs dir entry {}",
                logs_dir.display()
            )
        })?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .with_context(|| format!("failed to inspect diagnostic source {}", path.display()))?;
        if !metadata.file_type().is_file() {
            continue;
        }
        let canonical_path = path
            .canonicalize()
            .with_context(|| format!("failed to resolve diagnostic source {}", path.display()))?;
        if canonical_path.starts_with(&canonical_logs_dir) && is_diagnostic_source(&path) {
            sources.push(path);
        }
    }
    sources.sort();
    Ok(sources)
}

fn is_diagnostic_source(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    file_name == FRONTEND_DIAGNOSTICS_FILE_NAME
        || file_name.starts_with("desktop-shell") && file_name.ends_with(".log")
        || file_name.starts_with("desktop-core.") && file_name.ends_with(".log")
}

fn diagnostics_export_dir_name(timestamp: &str) -> String {
    format!("mcpmate-diagnostics-{timestamp}")
}

struct DiagnosticsRedactor {
    replacements: Vec<(String, &'static str)>,
}

impl DiagnosticsRedactor {
    fn new(paths: &MCPMatePaths) -> Self {
        let mut replacements = vec![
            (paths.logs_dir().display().to_string(), "[mcpmate-logs-dir]"),
            (paths.base_dir().display().to_string(), "[mcpmate-data-dir]"),
        ];
        if let Some(home_dir) = std::env::var_os("HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
        {
            replacements.push((home_dir.display().to_string(), "[user-home]"));
        }
        Self { replacements }
    }

    fn redact_json(&self, value: Value) -> Value {
        match value {
            Value::String(text) => Value::String(self.redact_text(&text)),
            Value::Array(items) => Value::Array(
                items
                    .into_iter()
                    .map(|item| self.redact_json(item))
                    .collect(),
            ),
            Value::Object(map) => Value::Object(
                map.into_iter()
                    .map(|(key, value)| {
                        if is_sensitive_key(&key) {
                            (key, Value::String("[redacted-secret]".to_string()))
                        } else {
                            (key, self.redact_json(value))
                        }
                    })
                    .collect(),
            ),
            other => other,
        }
    }

    fn redact_text(&self, input: &str) -> String {
        let mut output = input.to_string();
        for (needle, replacement) in &self.replacements {
            if !needle.is_empty() {
                output = output.replace(needle, replacement);
            }
        }
        output = redacted_user_home_path_regex()
            .replace_all(&output, "[user-home]/[redacted-path]")
            .to_string();
        output = users_path_with_tail_regex()
            .replace_all(&output, "[user-home]/[redacted-path]")
            .to_string();
        output = users_path_regex()
            .replace_all(&output, "[user-home]")
            .to_string();
        output = windows_users_path_with_tail_regex()
            .replace_all(&output, "[user-home]\\[redacted-path]")
            .to_string();
        output = windows_users_path_regex()
            .replace_all(&output, "[user-home]")
            .to_string();
        output = escaped_windows_users_path_with_tail_regex()
            .replace_all(&output, "[user-home]\\\\[redacted-path]")
            .to_string();
        output = escaped_windows_users_path_regex()
            .replace_all(&output, "[user-home]")
            .to_string();
        output = unix_home_path_with_tail_regex()
            .replace_all(&output, "[user-home]/[redacted-path]")
            .to_string();
        output = unix_home_path_regex()
            .replace_all(&output, "[user-home]")
            .to_string();
        output = volumes_path_regex()
            .replace_all(&output, "[external-volume]/[redacted-path]")
            .to_string();
        output = bearer_regex()
            .replace_all(&output, "${prefix}[redacted-secret]")
            .to_string();
        output = cookie_header_regex()
            .replace_all(&output, "${prefix}[redacted-secret]")
            .to_string();
        output = cli_secret_arg_regex()
            .replace_all(&output, "${prefix}[redacted-secret]")
            .to_string();
        output = assignment_secret_regex()
            .replace_all(&output, "${prefix}[redacted-secret]")
            .to_string();
        output = json_secret_regex()
            .replace_all(&output, "${prefix}\"[redacted-secret]\"")
            .to_string();
        output = url_regex()
            .replace_all(&output, |captures: &regex::Captures<'_>| {
                format!("{}://[redacted]", &captures["scheme"])
            })
            .to_string();
        output
    }
}

fn is_sensitive_key(key: &str) -> bool {
    let normalized = key.replace(['-', '_'], "").to_ascii_lowercase();
    [
        "authorization",
        "apikey",
        "authtoken",
        "accesstoken",
        "refreshtoken",
        "clientsecret",
        "password",
        "secret",
        "token",
        "cookie",
        "credential",
        "databaseurl",
        "connectionstring",
        "session",
        "jwt",
        "dsn",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn redacted_user_home_path_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#"\[user-home\](?:[/\\][^\r\n"']+)+"#)
            .expect("compile redacted user home path regex")
    })
}

fn users_path_with_tail_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#"/Users/[^/\r\n"']+(?:/[^\r\n"']+)+"#)
            .expect("compile users path with tail regex")
    })
}

fn users_path_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r#"/Users/[^/\r\n"']+"#).expect("compile users path regex"))
}

fn windows_users_path_with_tail_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#"(?i)[a-z]:\\Users\\[^\\\r\n"']+(?:\\[^\\\r\n"']+)+"#)
            .expect("compile windows users path with tail regex")
    })
}

fn windows_users_path_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#"(?i)[a-z]:\\Users\\[^\\\r\n"']+"#).expect("compile windows users path regex")
    })
}

fn escaped_windows_users_path_with_tail_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#"(?i)[a-z]:(?:\\\\)+Users(?:\\\\)+[^\\\r\n"']+(?:(?:\\\\)+[^\\\r\n"']+)+"#)
            .expect("compile escaped windows users path with tail regex")
    })
}

fn escaped_windows_users_path_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#"(?i)[a-z]:(?:\\\\)+Users(?:\\\\)+[^\\\r\n"']+"#)
            .expect("compile escaped windows users path regex")
    })
}

fn unix_home_path_with_tail_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#"/home/[^/\r\n"']+(?:/[^\r\n"']+)+"#)
            .expect("compile unix home path with tail regex")
    })
}

fn unix_home_path_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r#"/home/[^/\r\n"']+"#).expect("compile unix home path regex"))
}

fn volumes_path_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#"/Volumes/[^/\r\n"']+(?:/[^\r\n"']+)*"#).expect("compile volumes path regex")
    })
}

fn bearer_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)(?P<prefix>\bauthorization\s*[:=]\s*)(?:bearer\s+)?[^\s,;]+")
            .expect("compile bearer regex")
    })
}

fn cookie_header_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)(?P<prefix>\b(?:set-cookie|cookie)\s*[:=]\s*)[^\r\n]+")
            .expect("compile cookie header regex")
    })
}

fn cli_secret_arg_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)(?P<prefix>(?:^|[\s,;])--?[a-z0-9_-]*(?:api[_-]?key|auth[_-]?token|access[_-]?token|refresh[_-]?token|client[_-]?secret|authorization|password|secret|token|cookie|credential|database[_-]?url|connection[_-]?string|session|jwt|dsn)[a-z0-9_-]*(?:\s+|=))(?:[^\s,;]+)")
            .expect("compile cli secret argument regex")
    })
}

fn assignment_secret_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)(?P<prefix>\b[a-z0-9_-]*(?:api[_-]?key|auth[_-]?token|access[_-]?token|refresh[_-]?token|client[_-]?secret|authorization|password|secret|token|cookie|credential|database[_-]?url|connection[_-]?string|session|jwt|dsn)[a-z0-9_-]*\s*[:=]\s*)[^\s,;]+")
        .expect("compile assignment secret regex")
    })
}

fn json_secret_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#"(?i)(?P<prefix>"[^"]*(?:api[_-]?key|auth[_-]?token|access[_-]?token|refresh[_-]?token|client[_-]?secret|authorization|password|secret|token|cookie|credential|database[_-]?url|connection[_-]?string|session|jwt|dsn)[^"]*"\s*:\s*)"[^"]*""#)
        .expect("compile json secret regex")
    })
}

fn url_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#"(?i)\b(?P<scheme>[a-z][a-z0-9+.-]*)://[^\s"']+"#).expect("compile url regex")
    })
}

fn current_timestamp() -> Result<String> {
    Ok(format!("{}Z", unix_millis()?))
}

fn unix_millis() -> Result<u128> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before Unix epoch")?
        .as_millis())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use mcpmate::common::MCPMatePaths;
    use serde_json::json;

    #[test]
    fn diagnostics_export_dir_name_uses_timestamp() {
        assert_eq!(
            super::diagnostics_export_dir_name("20260604T120000Z"),
            "mcpmate-diagnostics-20260604T120000Z"
        );
    }

    #[test]
    fn diagnostic_source_filter_includes_runtime_logs_and_frontend_events() {
        assert!(super::is_diagnostic_source(Path::new("desktop-shell.log")));
        assert!(super::is_diagnostic_source(Path::new(
            "desktop-shell.2026-06-04.log"
        )));
        assert!(super::is_diagnostic_source(Path::new(
            "desktop-core.startup-123.log"
        )));
        assert!(super::is_diagnostic_source(Path::new(
            "frontend-diagnostics.jsonl"
        )));
        assert!(!super::is_diagnostic_source(Path::new("notes.txt")));
    }

    #[test]
    fn diagnostics_export_copies_logs_and_writes_runtime_files() {
        let test_dir = std::env::temp_dir().join(format!(
            "mcpmate-diagnostics-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let base_dir = test_dir.join("base");
        let destination = test_dir.join("export");
        let logs_dir = base_dir.join("logs");
        std::fs::create_dir_all(&logs_dir).expect("create logs dir");
        std::fs::create_dir_all(&destination).expect("create export dir");
        std::fs::write(logs_dir.join("desktop-shell.log"), "shell log").expect("write shell log");
        std::fs::write(logs_dir.join("frontend-diagnostics.jsonl"), "{}\n")
            .expect("write frontend log");
        std::fs::write(logs_dir.join("notes.txt"), "ignore").expect("write ignored file");

        let paths = MCPMatePaths::from_base_dir(&base_dir).expect("paths");
        let response = super::export_diagnostics_bundle(
            &paths,
            &destination,
            json!({ "selectedSource": "localhost" }),
            json!({ "backend": { "status": "ok" } }),
        )
        .expect("export diagnostics");
        let export_path = Path::new(&response.export_path);

        assert!(export_path.join("manifest.json").exists());
        assert!(export_path.join("runtime.json").exists());
        assert!(export_path.join("snapshot.json").exists());
        assert!(export_path.join("logs/desktop-shell.log").exists());
        assert!(export_path.join("logs/frontend-diagnostics.jsonl").exists());
        assert!(!export_path.join("logs/notes.txt").exists());
        assert_eq!(response.file_count, 5);

        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn diagnostics_export_redacts_sensitive_log_content_and_private_paths() {
        let test_dir = std::env::temp_dir().join(format!(
            "mcpmate-diagnostics-redaction-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let base_dir = test_dir.join("Users").join("loocor").join(".mcpmate");
        let destination = test_dir.join("export");
        let logs_dir = base_dir.join("logs");
        std::fs::create_dir_all(&logs_dir).expect("create logs dir");
        std::fs::create_dir_all(&destination).expect("create export dir");
        std::fs::write(
            logs_dir.join("desktop-shell.log"),
            format!(
                "HOME=/Users/loocor\nAuthorization: Bearer secret-token\nbase={}\nclientPath=/Users/loocor/Documents/SecretClient/project.json\nvolumePath=/Volumes/SecretDrive/project/config.json",
                base_dir.display()
            ),
        )
        .expect("write shell log");

        let paths = MCPMatePaths::from_base_dir(&base_dir).expect("paths");
        let response = super::export_diagnostics_bundle(
            &paths,
            &destination,
            json!({
                "dataDir": base_dir.display().to_string(),
                "token": "secret-token",
                "remoteUrl": "https://user:pass@example.com/private?token=secret-token",
                "workspacePath": "/Users/loocor/Documents/SecretClient/project.json",
                "externalPath": "/Volumes/SecretDrive/project/config.json",
            }),
            json!({
                "backend": {
                    "apiBaseUrl": "https://user:pass@example.com/private?token=secret-token",
                    "workspacePath": "/Users/loocor/Documents/SecretClient/project.json",
                    "databaseUrl": "postgres://user:pass@db.example/app"
                }
            }),
        )
        .expect("export diagnostics");
        let export_path = Path::new(&response.export_path);
        let exported_log =
            std::fs::read_to_string(export_path.join("logs/desktop-shell.log")).expect("read log");
        let manifest =
            std::fs::read_to_string(export_path.join("manifest.json")).expect("read manifest");
        let runtime =
            std::fs::read_to_string(export_path.join("runtime.json")).expect("read runtime");
        let snapshot =
            std::fs::read_to_string(export_path.join("snapshot.json")).expect("read snapshot");

        assert!(!exported_log.contains("/Users/loocor"));
        assert!(!exported_log.contains("SecretClient"));
        assert!(!exported_log.contains("SecretDrive"));
        assert!(!exported_log.contains("secret-token"));
        assert!(exported_log.contains("[user-home]"));
        assert!(exported_log.contains("[external-volume]/[redacted-path]"));
        assert!(exported_log.contains("[redacted-secret]"));
        assert!(!manifest.contains(base_dir.to_string_lossy().as_ref()));
        assert!(!runtime.contains(base_dir.to_string_lossy().as_ref()));
        assert!(!runtime.contains("SecretClient"));
        assert!(!runtime.contains("SecretDrive"));
        assert!(!runtime.contains("secret-token"));
        assert!(!runtime.contains("https://user:pass@example.com"));
        assert!(!snapshot.contains("SecretClient"));
        assert!(!snapshot.contains("secret-token"));
        assert!(!snapshot.contains("https://user:pass@example.com"));
        assert!(!snapshot.contains("postgres://user:pass@db.example"));

        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn frontend_diagnostic_events_are_redacted_before_persisting() {
        let test_dir = std::env::temp_dir().join(format!(
            "mcpmate-frontend-diagnostics-redaction-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let base_dir = test_dir.join("base");
        let paths = MCPMatePaths::from_base_dir(&base_dir).expect("paths");
        let payload = super::DiagnosticEventPayload {
            level: "info".to_string(),
            source: "test".to_string(),
            message: "Authorization: Bearer secret-token".to_string(),
            data: json!({
                "token": "secret-token",
                "url": "https://user:pass@example.com/private?token=secret-token",
            }),
        };

        let path = super::record_frontend_diagnostic_event(&paths, &payload)
            .expect("record frontend diagnostic");
        let content = std::fs::read_to_string(path).expect("read frontend diagnostics");

        assert!(!content.contains("secret-token"));
        assert!(!content.contains("user:pass@example.com"));
        assert!(content.contains("[redacted-secret]"));
        assert!(content.contains("https://[redacted]"));

        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn diagnostics_redaction_catches_prefixed_secret_keys_and_windows_paths() {
        let test_dir = std::env::temp_dir().join(format!(
            "mcpmate-diagnostics-prefixed-secret-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let paths = MCPMatePaths::from_base_dir(test_dir.join("base")).expect("paths");
        let redactor = super::DiagnosticsRedactor::new(&paths);
        let redacted = redactor.redact_text(
            "OPENAI_API_KEY=sk-secret ANTHROPIC_AUTH_TOKEN=anthropic-secret x-api-key: x-secret --api-key sk-cli --token token-cli Cookie: sid=cookie-secret; theme=dark\nSet-Cookie: refresh=refresh-secret\nDATABASE_URL=postgres://user:pass@db.example/app connection_string=mysql://user:pass@db.example/app C:\\Users\\loocor\\AppData\\Local",
        );
        let redacted_json = redactor.redact_json(json!({
            "openai_api_key": "sk-secret",
            "githubToken": "gh-secret",
            "sessionId": "session-secret",
            "cookie": "cookie-secret",
            "databaseUrl": "postgres://user:pass@db.example/app",
            "dsn": "postgres://user:pass@db.example/app",
            "profilePath": "C:\\Users\\loocor\\AppData\\Local\\MCPMate",
        }));

        assert!(!redacted.contains("sk-secret"));
        assert!(!redacted.contains("anthropic-secret"));
        assert!(!redacted.contains("x-secret"));
        assert!(!redacted.contains("sk-cli"));
        assert!(!redacted.contains("token-cli"));
        assert!(!redacted.contains("cookie-secret"));
        assert!(!redacted.contains("refresh-secret"));
        assert!(!redacted.contains("postgres://user:pass@db.example"));
        assert!(!redacted.contains("mysql://user:pass@db.example"));
        assert!(!redacted.contains("C:\\Users\\loocor"));
        let redacted_json_text = redacted_json.to_string();
        assert!(!redacted_json_text.contains("sk-secret"));
        assert!(!redacted_json_text.contains("gh-secret"));
        assert!(!redacted_json_text.contains("session-secret"));
        assert!(!redacted_json_text.contains("cookie-secret"));
        assert!(!redacted_json_text.contains("postgres://user:pass@db.example"));
        assert!(!redacted_json_text.contains("C:\\Users\\loocor"));
    }

    #[test]
    fn diagnostics_redaction_catches_paths_with_spaces_and_escaped_windows_paths() {
        let test_dir = std::env::temp_dir().join(format!(
            "mcpmate-diagnostics-path-space-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let paths = MCPMatePaths::from_base_dir(test_dir.join("base")).expect("paths");
        let redactor = super::DiagnosticsRedactor::new(&paths);
        let redacted = redactor.redact_text(
            r#"configPath="/Users/loocor/Secret Project/config.json"
volumePath="/Volumes/Secret Drive/client config.json"
escapedWindowsPath="C:\\Users\\loocor\\Secret Project\\config.json""#,
        );

        assert!(!redacted.contains("Secret Project"));
        assert!(!redacted.contains("Secret Drive"));
        assert!(!redacted.contains(r#"C:\\Users\\loocor"#));
        assert!(redacted.contains("[user-home]/[redacted-path]"));
        assert!(redacted.contains("[external-volume]/[redacted-path]"));
        assert!(redacted.contains(r#"[user-home]\\[redacted-path]"#));
    }

    #[cfg(unix)]
    #[test]
    fn diagnostics_export_skips_log_symlinks() {
        use std::os::unix::fs::symlink;

        let test_dir = std::env::temp_dir().join(format!(
            "mcpmate-diagnostics-symlink-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let base_dir = test_dir.join("base");
        let destination = test_dir.join("export");
        let logs_dir = base_dir.join("logs");
        std::fs::create_dir_all(&logs_dir).expect("create logs dir");
        std::fs::create_dir_all(&destination).expect("create export dir");
        let outside = test_dir.join("outside-secret.txt");
        std::fs::write(&outside, "outside secret").expect("write outside file");
        symlink(&outside, logs_dir.join("desktop-shell.log")).expect("create log symlink");

        let paths = MCPMatePaths::from_base_dir(&base_dir).expect("paths");
        let response = super::export_diagnostics_bundle(&paths, &destination, json!({}), json!({}))
            .expect("export diagnostics");
        let export_path = Path::new(&response.export_path);

        assert!(!export_path.join("logs/desktop-shell.log").exists());

        let _ = std::fs::remove_dir_all(&test_dir);
    }
}
