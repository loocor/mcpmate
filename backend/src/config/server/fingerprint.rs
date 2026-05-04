use url::Url;

// Public entrypoints
pub(crate) fn fingerprint_for_stdio(
    command: &str,
    args: &[String],
) -> String {
    let signature = parse_command_signature(command, args);

    match signature.kind {
        SignatureKind::NodePackage => {
            let pkg = signature.primary.unwrap_or_default();
            let tail = if signature.remainder.is_empty() {
                String::new()
            } else {
                signature.remainder.join(" ")
            };
            format!("node-pkg:{}:{}", pkg, tail)
        }
        SignatureKind::PythonModule => {
            let module = signature.primary.unwrap_or_default();
            format!("python-module:{}:{}", module, signature.remainder.join(" "))
        }
        SignatureKind::PythonPackage => {
            let pkg = signature.primary.unwrap_or_default();
            format!("python-pkg:{}:{}", pkg, signature.remainder.join(" "))
        }
        SignatureKind::PythonScript => {
            let script = signature.primary.unwrap_or_default();
            format!("python-file:{}:{}", script, signature.remainder.join(" "))
        }
        SignatureKind::Default => {
            let mut tail = String::new();
            for (i, v) in signature.normalized_args.iter().map(|s| s.as_str()).take(3).enumerate() {
                if i > 0 {
                    tail.push(' ');
                }
                tail.push_str(v);
            }
            format!("cmd:{}:{}", signature.command, tail)
        }
    }
}

#[derive(Debug, Clone)]
pub struct UrlSignature {
    pub fingerprint: String,
    pub base: String,
    pub filtered_query: String,
    pub display_filtered_query: String,
    pub raw_query: Option<String>,
}

impl UrlSignature {
    pub fn display_query(&self) -> Option<String> {
        if self.display_filtered_query.is_empty() {
            None
        } else {
            Some(self.display_filtered_query.clone())
        }
    }
}

pub(crate) fn url_signature(raw: &str) -> UrlSignature {
    if let Ok(mut url) = Url::parse(raw) {
        url.set_fragment(None);

        let scheme = url.scheme().to_ascii_lowercase();
        let host = url.host_str().map(|h| h.to_ascii_lowercase()).unwrap_or_default();

        let default_port = match scheme.as_str() {
            "http" => Some(80),
            "https" => Some(443),
            _ => None,
        };
        let explicit_port = url.port();
        let final_port = explicit_port.or(default_port);
        let include_port = match (explicit_port, default_port) {
            (Some(p), Some(d)) => p != d,
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => false,
        };
        let port_part = if include_port {
            format!(":{}", final_port.unwrap())
        } else {
            String::new()
        };

        let mut path = url.path().to_string();
        if path.is_empty() {
            path = "/".to_string();
        } else if path.len() > 1 && path.ends_with('/') {
            path.pop();
        }

        let base = format!("{}{}{}", host, port_part, path);

        let (raw_query, filtered_query, display_filtered_query) = canonicalize_query(url.query());
        let fingerprint = if filtered_query.is_empty() {
            base.clone()
        } else {
            format!("{}?{}", base, filtered_query)
        };

        UrlSignature {
            fingerprint,
            base,
            filtered_query,
            display_filtered_query,
            raw_query,
        }
    } else {
        UrlSignature {
            fingerprint: raw.to_string(),
            base: raw.to_string(),
            filtered_query: String::new(),
            display_filtered_query: String::new(),
            raw_query: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CommandSignature {
    command: String,
    normalized_args: Vec<String>,
    kind: SignatureKind,
    primary: Option<String>,
    remainder: Vec<String>,
}

impl CommandSignature {
    fn new(
        command: String,
        normalized_args: Vec<String>,
        kind: SignatureKind,
        primary: Option<String>,
        remainder: Vec<String>,
    ) -> Self {
        Self {
            command,
            normalized_args,
            kind,
            primary,
            remainder,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SignatureKind {
    NodePackage,
    PythonModule,
    PythonPackage,
    PythonScript,
    Default,
}

fn parse_command_signature(
    command: &str,
    args: &[String],
) -> CommandSignature {
    let normalized_args: Vec<String> = args
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let cmd = command.trim().to_ascii_lowercase();

    if let Some((pkg, remainder)) = extract_node_package_spec(&cmd, &normalized_args) {
        return CommandSignature::new(cmd, normalized_args, SignatureKind::NodePackage, Some(pkg), remainder);
    }

    if let Some(sig) = parse_python_signature(&cmd, &normalized_args) {
        return CommandSignature::new(cmd, normalized_args, sig.kind, sig.primary, sig.remainder);
    }

    CommandSignature::new(cmd, normalized_args.clone(), SignatureKind::Default, None, Vec::new())
}

struct PythonSignature {
    kind: SignatureKind,
    primary: Option<String>,
    remainder: Vec<String>,
}

fn parse_python_signature(
    cmd: &str,
    args: &[String],
) -> Option<PythonSignature> {
    if !matches!(cmd, "uvx" | "pipx" | "python" | "python3" | "py") {
        return None;
    }

    let mut offset = 0;
    if cmd == "uvx" {
        if let Some(first) = args.first() {
            let lower = first.to_ascii_lowercase();
            if lower == "python" || lower == "python3" || lower == "py" {
                offset += 1;
            }
        }
    }

    if cmd == "pipx" {
        if let Some(next) = args.get(offset) {
            if next.eq_ignore_ascii_case("run") {
                offset += 1;
            }
        }
    }

    if offset >= args.len() {
        return None;
    }

    let slice = &args[offset..];
    if slice.is_empty() {
        return None;
    }

    if matches!(cmd, "python" | "python3" | "py") {
        return parse_python_interpreter(slice);
    }

    parse_python_runner(slice)
}

const PY_BOOL_FLAGS: &[&str] = &[
    "-B",
    "-d",
    "-E",
    "-i",
    "-I",
    "-O",
    "-OO",
    "-q",
    "-s",
    "-S",
    "-u",
    "-v",
    "-x",
    "-3",
    "--help",
    "--version",
    "--quiet",
    "--silent",
    "--verbose",
    "--isolated",
    "--no-user-site",
    "--no-site",
];

const PY_VALUE_FLAGS: &[&str] = &["-c", "-W", "-X", "--check-hash-based-pycs"];

fn parse_python_interpreter(args: &[String]) -> Option<PythonSignature> {
    let mut i = 0;
    while i < args.len() {
        let token = args[i].as_str();

        if token == "--" {
            if i + 1 < args.len() {
                let script = shorten_script_name(&args[i + 1]);
                let remainder = args[i + 2..].to_vec();
                return Some(PythonSignature {
                    kind: SignatureKind::PythonScript,
                    primary: Some(script),
                    remainder,
                });
            }
            return None;
        }

        if token == "-m" {
            if i + 1 < args.len() {
                let module = args[i + 1].clone();
                let remainder = args[i + 2..].to_vec();
                return Some(PythonSignature {
                    kind: SignatureKind::PythonModule,
                    primary: Some(module),
                    remainder,
                });
            }
            return None;
        }

        if token.starts_with("-m") && token.len() > 2 {
            let module = token[2..].to_string();
            let remainder = args[i + 1..].to_vec();
            return Some(PythonSignature {
                kind: SignatureKind::PythonModule,
                primary: Some(module),
                remainder,
            });
        }

        if let Some(skip) = python_flag_span(token, &args[i..]) {
            i += skip;
            continue;
        }

        let script = shorten_script_name(&args[i]);
        let remainder = args[i + 1..].to_vec();
        return Some(PythonSignature {
            kind: SignatureKind::PythonScript,
            primary: Some(script),
            remainder,
        });
    }

    None
}

fn python_flag_span(
    token: &str,
    rest: &[String],
) -> Option<usize> {
    if PY_BOOL_FLAGS.contains(&token) {
        return Some(1);
    }

    if token.len() > 2
        && (token.starts_with("-O") || token.starts_with("-B") || token.starts_with("-q") || token.starts_with("-v"))
    {
        return Some(1);
    }

    if token.starts_with("-W") && token.len() > 2 {
        return Some(1);
    }
    if token.starts_with("-X") && token.len() > 2 {
        return Some(1);
    }
    if token.starts_with("--check-hash-based-pycs=") {
        return Some(1);
    }

    if PY_VALUE_FLAGS.contains(&token) {
        return Some(if rest.len() > 1 { 2 } else { 1 });
    }

    None
}

const PY_RUNNER_BOOL_FLAGS: &[&str] = &[
    "-q",
    "--quiet",
    "--silent",
    "--system-site-packages",
    "--no-cache",
    "--no-cache-dir",
    "--no-install",
    "--force",
    "--upgrade",
    "--refresh",
    "--install",
    "--isolated",
];

const PY_RUNNER_VALUE_FLAGS: &[&str] = &[
    "--python",
    "--with",
    "--from",
    "--index-url",
    "--extra-index-url",
    "--pip",
    "--pip-args",
    "--pip-install-args",
    "--python-argv",
    "--python-args",
    "--python-option",
    "--env",
    "--cache-dir",
    "--cwd",
    "--spec",
];

fn parse_python_runner(args: &[String]) -> Option<PythonSignature> {
    let mut i = 0;
    let mut spec_value: Option<String> = None;

    while i < args.len() {
        let token = args[i].as_str();
        if token == "--" {
            i += 1;
            break;
        }

        if token == "-m" {
            if i + 1 < args.len() {
                let module = args[i + 1].clone();
                let remainder = args[i + 2..].to_vec();
                return Some(PythonSignature {
                    kind: SignatureKind::PythonModule,
                    primary: Some(module),
                    remainder,
                });
            }
            return None;
        }

        if token.starts_with("-m") && token.len() > 2 {
            let module = token[2..].to_string();
            let remainder = args[i + 1..].to_vec();
            return Some(PythonSignature {
                kind: SignatureKind::PythonModule,
                primary: Some(module),
                remainder,
            });
        }

        if let Some(stripped) = token.strip_prefix("--spec=") {
            if spec_value.is_none() {
                spec_value = Some(stripped.to_string());
            }
            i += 1;
            continue;
        }

        if PY_RUNNER_BOOL_FLAGS.contains(&token) {
            i += 1;
            continue;
        }

        if PY_RUNNER_VALUE_FLAGS.contains(&token) {
            if token == "--spec" && spec_value.is_none() && i + 1 < args.len() {
                spec_value = Some(args[i + 1].clone());
            }
            i += if i + 1 < args.len() { 2 } else { 1 };
            continue;
        }

        if let Some(eq_pos) = token.find('=') {
            let flag_name = &token[..eq_pos];
            if PY_RUNNER_VALUE_FLAGS.contains(&flag_name) {
                if flag_name == "--spec" && spec_value.is_none() {
                    spec_value = Some(token[eq_pos + 1..].to_string());
                }
                i += 1;
                continue;
            }
        }

        if token.starts_with('-') {
            i += 1;
            continue;
        }

        break;
    }

    let positional_index = if i < args.len() { Some(i) } else { None };

    let package = if let Some(spec) = spec_value {
        spec
    } else if let Some(idx) = positional_index {
        args[idx].clone()
    } else {
        return None;
    };

    let raw_remainder = if let Some(idx) = positional_index {
        args[idx + 1..].to_vec()
    } else {
        Vec::new()
    };

    let remainder = sanitize_runner_remainder(&raw_remainder);

    Some(PythonSignature {
        kind: SignatureKind::PythonPackage,
        primary: Some(package),
        remainder,
    })
}

fn shorten_script_name(script: &str) -> String {
    script.rsplit('/').next().unwrap_or(script).to_string()
}

fn sanitize_runner_remainder(values: &[String]) -> Vec<String> {
    let mut cleaned = Vec::new();
    let mut idx = 0;
    while idx < values.len() {
        let token = values[idx].as_str();
        if PY_RUNNER_BOOL_FLAGS.contains(&token) {
            idx += 1;
            continue;
        }
        if PY_RUNNER_VALUE_FLAGS.contains(&token) {
            idx += if idx + 1 < values.len() { 2 } else { 1 };
            continue;
        }
        if let Some(eq_pos) = token.find('=') {
            let flag_name = &token[..eq_pos];
            if PY_RUNNER_VALUE_FLAGS.contains(&flag_name) {
                idx += 1;
                continue;
            }
        }
        cleaned.push(values[idx].clone());
        idx += 1;
    }
    cleaned
}

const QUERY_IGNORE_KEYS: &[&str] = &[
    "token",
    "access_token",
    "auth",
    "auth_token",
    "signature",
    "sig",
    "timestamp",
    "ts",
    "nonce",
    "api_key",
    "apikey",
    "session",
];

const DISPLAY_QUERY_IGNORE_KEYS: &[&str] = &[
    "token",
    "access_token",
    "auth",
    "auth_token",
    "signature",
    "sig",
    "timestamp",
    "ts",
    "nonce",
    "api_key",
    "apikey",
    "session",
    "secret",
    "client_secret",
    "password",
    "passwd",
    "private_key",
    "refresh_token",
    "id_token",
    "key",
];

fn canonicalize_query(query: Option<&str>) -> (Option<String>, String, String) {
    if let Some(q) = query {
        let mut pairs: Vec<(String, String)> = url::form_urlencoded::parse(q.as_bytes()).into_owned().collect();
        if pairs.is_empty() {
            return (None, String::new(), String::new());
        }
        pairs.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

        let filtered_pairs: Vec<(String, String)> = pairs
            .iter()
            .filter(|(k, _)| !should_ignore_query_key(k))
            .cloned()
            .collect();

        let display_filtered_pairs: Vec<(String, String)> = filtered_pairs
            .iter()
            .filter(|(k, _)| !should_ignore_display_query_key(k))
            .cloned()
            .collect();

        let raw_query = Some(
            pairs
                .into_iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("&"),
        );

        let filtered_query = if filtered_pairs.is_empty() {
            String::new()
        } else {
            filtered_pairs
                .into_iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("&")
        };

        let display_filtered_query = if display_filtered_pairs.is_empty() {
            String::new()
        } else {
            display_filtered_pairs
                .into_iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("&")
        };

        (raw_query, filtered_query, display_filtered_query)
    } else {
        (None, String::new(), String::new())
    }
}

fn should_ignore_query_key(key: &str) -> bool {
    QUERY_IGNORE_KEYS.iter().any(|ignore| key.eq_ignore_ascii_case(ignore))
}

fn should_ignore_display_query_key(key: &str) -> bool {
    DISPLAY_QUERY_IGNORE_KEYS
        .iter()
        .any(|ignore| key.eq_ignore_ascii_case(ignore))
}

fn extract_node_package_spec(
    cmd: &str,
    args: &[String],
) -> Option<(String, Vec<String>)> {
    if cmd != "npx" && cmd != "bunx" && cmd != "pnpm" && cmd != "yarn" {
        return None;
    }

    let mut slice = args;
    if (cmd == "pnpm" || cmd == "yarn") && slice.first().map(|s| s.eq_ignore_ascii_case("dlx")).unwrap_or(false) {
        slice = &slice[1..];
    } else if cmd == "pnpm" || cmd == "yarn" {
        return None;
    }

    let mut pkg: Option<String> = None;
    let mut remainder: Vec<String> = Vec::new();
    let mut past_flags = false;
    let mut idx = 0;
    while idx < slice.len() {
        let current = &slice[idx];
        if !past_flags {
            if current == "--" {
                past_flags = true;
                idx += 1;
                continue;
            }
            if is_node_runner_flag(current) {
                idx += 1;
                continue;
            }
        }

        if pkg.is_none() {
            pkg = Some(current.clone());
            past_flags = true;
            idx += 1;
            continue;
        }

        remainder.extend(slice[idx..].iter().cloned());
        break;
    }

    pkg.map(|p| (p, remainder))
}

fn is_node_runner_flag(arg: &str) -> bool {
    matches!(
        arg.trim().to_ascii_lowercase().as_str(),
        "-y" | "--yes" | "--quiet" | "--prefer-offline" | "--no-install" | "--legacy-peer-deps"
    )
}

#[cfg(test)]
mod tests {
    use super::{fingerprint_for_stdio, url_signature};

    fn to_vec(args: &[&str]) -> Vec<String> {
        args.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn npx_flag_filtered_from_fingerprint() {
        let with_flag = fingerprint_for_stdio("npx", &to_vec(&["-y", "@playwright/mcp@latest", "--extension"]));
        let without_flag = fingerprint_for_stdio("npx", &to_vec(&["@playwright/mcp@latest", "--extension"]));

        assert_eq!(with_flag, without_flag);
        assert_eq!(without_flag, "node-pkg:@playwright/mcp@latest:--extension".to_string());
    }

    #[test]
    fn pnpm_dlx_flag_filtered() {
        let base = fingerprint_for_stdio("pnpm", &to_vec(&["dlx", "@scope/server"]));
        let with_flag = fingerprint_for_stdio("pnpm", &to_vec(&["dlx", "-y", "@scope/server"]));

        assert_eq!(base, with_flag);
        assert_eq!(base, "node-pkg:@scope/server:".to_string());
    }

    #[test]
    fn fallback_still_includes_arguments() {
        let fp = fingerprint_for_stdio("bash", &to_vec(&["./start.sh", "--mode", "prod"]));
        assert_eq!(fp, "cmd:bash:./start.sh --mode prod".to_string());
    }

    #[test]
    fn uvx_python_module_normalizes_module_name() {
        let fp = fingerprint_for_stdio("uvx", &to_vec(&["python", "-m", "playwright_cli", "--verbose"]));
        assert_eq!(fp, "python-module:playwright_cli:--verbose".to_string());
    }

    #[test]
    fn python_with_interpreter_flags_and_module() {
        let fp = fingerprint_for_stdio("python", &to_vec(&["-O", "-m", "openai_mcp", "--debug"]));
        assert_eq!(fp, "python-module:openai_mcp:--debug".to_string());
    }

    #[test]
    fn python_script_skips_interpreter_flags() {
        let fp = fingerprint_for_stdio("python3", &to_vec(&["-OO", "./server/main.py", "--port", "8080"]));
        assert_eq!(fp, "python-file:main.py:--port 8080".to_string());
    }

    #[test]
    fn pipx_run_treated_as_python_package() {
        let fp = fingerprint_for_stdio("pipx", &to_vec(&["run", "playwright", "--log"]));
        assert_eq!(fp, "python-pkg:playwright:--log".to_string());
    }

    #[test]
    fn pipx_run_with_spec_prefers_spec_value() {
        let fp = fingerprint_for_stdio(
            "pipx",
            &to_vec(&["run", "--spec", "playwright-mcp==1.2.3", "playwright", "--log"]),
        );
        assert_eq!(fp, "python-pkg:playwright-mcp==1.2.3:--log".to_string());
    }

    #[test]
    fn uvx_with_runner_flags_ignores_flag_noise() {
        let fp = fingerprint_for_stdio(
            "uvx",
            &to_vec(&[
                "--python=python3.11",
                "--with",
                "playwright",
                "@playwright/mcp",
                "--no-cache",
            ]),
        );
        assert_eq!(fp, "python-pkg:@playwright/mcp:".to_string());
    }

    #[test]
    fn url_signature_collapses_default_ports() {
        let sig_http = url_signature("http://Example.com:80/path/?b=2&a=1");
        let sig_https = url_signature("https://example.com/path?a=1&b=2");

        assert_eq!(sig_http.base, "example.com/path");
        assert_eq!(sig_https.base, sig_http.base);
        assert_eq!(sig_http.fingerprint, sig_https.fingerprint);
        assert_eq!(sig_http.filtered_query, "a=1&b=2");
    }

    #[test]
    fn url_signature_ignores_common_tokens() {
        let sig = url_signature("https://example.com/data?api_key=ABC123&workspace=alpha&token=XYZ");
        assert_eq!(sig.filtered_query, "workspace=alpha");
        assert_eq!(sig.fingerprint, "example.com/data?workspace=alpha");
        assert_eq!(sig.display_query(), Some("workspace=alpha".to_string()));
    }

    #[test]
    fn url_signature_hides_queries_when_only_sensitive_params_remain() {
        let sig = url_signature("https://example.com/data?api_key=ABC123&token=XYZ");
        assert_eq!(sig.filtered_query, "");
        assert_eq!(sig.display_query(), None);
    }

    #[test]
    fn url_signature_hides_display_only_sensitive_params_without_changing_fingerprint() {
        let sig = url_signature("https://example.com/data?client_secret=ABC123&password=XYZ&workspace=alpha");
        assert_eq!(sig.filtered_query, "client_secret=ABC123&password=XYZ&workspace=alpha");
        assert_eq!(
            sig.fingerprint,
            "example.com/data?client_secret=ABC123&password=XYZ&workspace=alpha"
        );
        assert_eq!(sig.display_query(), Some("workspace=alpha".to_string()));
    }

    #[test]
    fn url_signature_hides_display_when_only_display_sensitive_params_remain() {
        let sig = url_signature("https://example.com/data?client_secret=ABC123&password=XYZ");
        assert_eq!(sig.filtered_query, "client_secret=ABC123&password=XYZ");
        assert_eq!(sig.display_query(), None);
    }

    #[test]
    fn url_signature_handles_unparseable_input() {
        let sig = url_signature("not a valid url");
        assert_eq!(sig.fingerprint, "not a valid url");
        assert_eq!(sig.base, "not a valid url");
    }
}
