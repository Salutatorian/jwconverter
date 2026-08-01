//! Validate pasted media URLs before invoking the link resolver.
//! Reject dangerous schemes, local files, and private/loopback targets.

use std::net::IpAddr;

use crate::errors::AppError;

/// Validated remote http(s) URL safe to pass as a single argv to yt-dlp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafeMediaUrl {
    raw: String,
}

impl SafeMediaUrl {
    pub fn as_str(&self) -> &str {
        &self.raw
    }
}

pub fn validate_media_url(input: &str) -> Result<SafeMediaUrl, AppError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(AppError::UnsupportedFormat {
            detail: "Paste a media URL first.".to_string(),
        });
    }
    if trimmed.contains('\0') {
        return Err(AppError::UnsupportedFormat {
            detail: "URL contains invalid characters.".to_string(),
        });
    }
    // Reject shell metacharacters that should never appear in a normal URL paste.
    if trimmed.chars().any(|c| matches!(c, '\n' | '\r' | '\t')) {
        return Err(AppError::UnsupportedFormat {
            detail: "URL must be a single line.".to_string(),
        });
    }

    let parsed = url::Url::parse(trimmed).map_err(|_| AppError::UnsupportedFormat {
        detail: "That does not look like a valid URL.".to_string(),
    })?;

    match parsed.scheme() {
        "https" | "http" => {}
        "file" | "ftp" | "ftps" | "data" | "javascript" | "vbscript" | "about" => {
            return Err(AppError::UnsupportedFormat {
                detail: "Only http and https links are supported.".to_string(),
            });
        }
        other => {
            return Err(AppError::UnsupportedFormat {
                detail: format!("Unsupported URL scheme: {other}"),
            });
        }
    }

    if parsed.cannot_be_a_base() {
        return Err(AppError::UnsupportedFormat {
            detail: "URL is not a usable web address.".to_string(),
        });
    }

    let host = parsed.host_str().ok_or_else(|| AppError::UnsupportedFormat {
        detail: "URL is missing a host.".to_string(),
    })?;

    let host_lower = host.to_ascii_lowercase();
    if host_lower == "localhost"
        || host_lower.ends_with(".localhost")
        || host_lower.ends_with(".local")
        || host_lower == "localtest.me"
        || host_lower.ends_with(".localtest.me")
    {
        return Err(AppError::PermissionDenied {
            detail: "Localhost links are not allowed in Links.".to_string(),
        });
    }

    if let Some(url::Host::Ipv4(v4)) = parsed.host() {
        if is_blocked_ip(IpAddr::V4(v4)) {
            return Err(AppError::PermissionDenied {
                detail: "Private or local network addresses are not allowed in Links.".to_string(),
            });
        }
    } else if let Some(url::Host::Ipv6(v6)) = parsed.host() {
        if is_blocked_ip(IpAddr::V6(v6)) {
            return Err(AppError::PermissionDenied {
                detail: "Private or local network addresses are not allowed in Links.".to_string(),
            });
        }
    } else if let Ok(ip) = host.parse::<IpAddr>() {
        // Hostname that happens to be an IP literal without Host::Ipv*
        if is_blocked_ip(ip) {
            return Err(AppError::PermissionDenied {
                detail: "Private or local network addresses are not allowed in Links.".to_string(),
            });
        }
    }

    Ok(SafeMediaUrl {
        raw: trimmed.to_string(),
    })
}

fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let octets = v4.octets();
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_broadcast()
                || v4.is_unspecified()
                || octets[0] == 0
                // Carrier-grade NAT 100.64.0.0/10
                || (octets[0] == 100 && octets[1] >= 64 && octets[1] <= 127)
        }
        IpAddr::V6(v6) => {
            if let Some(mapped) = v6.to_ipv4_mapped() {
                return is_blocked_ip(IpAddr::V4(mapped));
            }
            v6.is_loopback()
                || v6.is_unique_local()
                || v6.is_unicast_link_local()
                || v6.is_unspecified()
        }
    }
}

/// Redact common token-like query params for diagnostics.
pub fn redact_url_for_log(url: &str) -> String {
    let Ok(mut parsed) = url::Url::parse(url) else {
        return "[unparseable-url]".to_string();
    };
    let sensitive = [
        "token",
        "access_token",
        "auth",
        "key",
        "api_key",
        "signature",
        "sig",
        "password",
        "session",
        "cookie",
    ];
    let pairs: Vec<(String, String)> = parsed
        .query_pairs()
        .map(|(k, v)| {
            let key = k.to_string();
            let lower = key.to_ascii_lowercase();
            if sensitive.iter().any(|s| lower.contains(s)) {
                (key, "[redacted]".to_string())
            } else {
                (key, v.to_string())
            }
        })
        .collect();
    if pairs.is_empty() {
        return parsed.to_string();
    }
    parsed.set_query(None);
    let mut ser = parsed;
    {
        let mut qp = ser.query_pairs_mut();
        qp.clear();
        for (k, v) in pairs {
            qp.append_pair(&k, &v);
        }
    }
    ser.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_https_public_host() {
        let url = validate_media_url("https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap();
        assert!(url.as_str().starts_with("https://"));
    }

    #[test]
    fn rejects_empty() {
        assert!(validate_media_url("   ").is_err());
    }

    #[test]
    fn rejects_file_scheme() {
        assert!(validate_media_url("file:///C:/secret.mp4").is_err());
    }

    #[test]
    fn rejects_localhost() {
        assert!(validate_media_url("http://localhost:8080/a").is_err());
        assert!(validate_media_url("http://127.0.0.1/a").is_err());
        assert!(validate_media_url("http://[::ffff:127.0.0.1]/a").is_err());
        assert!(validate_media_url("http://foo.local/a").is_err());
    }

    #[test]
    fn rejects_private_lan() {
        assert!(validate_media_url("http://192.168.1.10/v").is_err());
        assert!(validate_media_url("http://10.0.0.5/v").is_err());
        assert!(validate_media_url("http://100.64.1.2/v").is_err());
    }

    #[test]
    fn rejects_javascript() {
        assert!(validate_media_url("javascript:alert(1)").is_err());
    }

    #[test]
    fn redacts_token_query() {
        let out = redact_url_for_log("https://example.com/x?id=1&access_token=secret&ok=yes");
        assert!(out.contains("access_token=%5Bredacted%5D") || out.contains("[redacted]"));
        assert!(!out.contains("secret"));
    }
}
