/// Remove comments from JSONC (JSON with Comments) content
/// Handles both single-line (//) and multi-line (/* */) comments
pub fn strip_comments(content: &str) -> String {
    let mut result = String::new();
    let mut chars = content.chars().peekable();
    let mut in_string = false;
    let mut in_single_comment = false;
    let mut in_multi_comment = false;
    let mut escaped = false;

    while let Some(ch) = chars.next() {
        if escaped {
            result.push(ch);
            escaped = false;
            continue;
        }

        if in_string {
            result.push(ch);
            if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        if in_single_comment {
            if ch == '\n' {
                in_single_comment = false;
                result.push(ch); // Keep the newline
            }
            continue;
        }

        if in_multi_comment {
            if ch == '*' && chars.peek() == Some(&'/') {
                chars.next(); // consume '/'
                in_multi_comment = false;
            }
            continue;
        }

        // Not in string or comment, check for comment start
        if ch == '"' {
            in_string = true;
            result.push(ch);
        } else if ch == '/' {
            match chars.peek() {
                Some('/') => {
                    chars.next(); // consume second '/'
                    in_single_comment = true;
                }
                Some('*') => {
                    chars.next(); // consume '*'
                    in_multi_comment = true;
                }
                _ => result.push(ch),
            }
        } else {
            result.push(ch);
        }
    }

    result
}
