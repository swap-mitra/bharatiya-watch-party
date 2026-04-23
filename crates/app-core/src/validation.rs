use url::Url;

use crate::{AppError, AppResult};

pub fn validate_display_name(value: &str) -> AppResult<()> {
    let trimmed = value.trim();
    if trimmed.len() < 2 || trimmed.len() > 24 {
        return Err(AppError::Validation(
            "display name must be between 2 and 24 characters".into(),
        ));
    }

    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, ' ' | '_' | '-'))
    {
        return Err(AppError::Validation(
            "display name may only contain letters, numbers, spaces, underscores, and hyphens"
                .into(),
        ));
    }

    Ok(())
}

pub fn validate_stream_url(value: &str) -> AppResult<()> {
    let parsed = Url::parse(value).map_err(|_| AppError::InvalidStreamUrl)?;
    let scheme_ok = matches!(parsed.scheme(), "http" | "https");
    let host_ok = parsed.has_host();

    if scheme_ok && host_ok {
        Ok(())
    } else {
        Err(AppError::InvalidStreamUrl)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_name() {
        let err = validate_display_name("!").expect_err("should reject");
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn accepts_http_stream_url() {
        validate_stream_url("https://example.com/stream.m3u8").expect("valid url");
    }
}
