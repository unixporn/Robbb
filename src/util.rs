use anyhow::*;
use std::env;

/// return with an error value immediately.
#[macro_export]
macro_rules! abort_with {
    ($err:expr) => {
        return Err($err.into());
    };
}

#[macro_export]
macro_rules! log_error {
    ($e:expr) => {
        if let Err(e) = $e {
            log::error!("{:?}", e);
        }
    };
    ($context:expr, $e:expr $(,)?) => {
        if let Err(e) = $e {
            log::error!("{:?}", ::anyhow::anyhow!(e).context($context));
        }
    };
}

/// Get an environment variable, returning an Err with a
/// nice error message mentioning the missing variable in case the value is not found.
pub fn required_env_var(key: &str) -> Result<String> {
    env::var(key).with_context(|| format!("Missing environment variable {}", key))
}

/// like [required_env_var], but also uses FromStr to parse the value.
pub fn parse_required_env_var<E: Into<anyhow::Error>, T: std::str::FromStr<Err = E>>(
    key: &str,
) -> Result<T> {
    required_env_var(key)?
        .parse()
        .map_err(|e: E| anyhow!(e))
        .with_context(|| format!("Failed to parse env-var {}", key))
}

/// Format a date into a normalized "2 days ago"-like format.
pub fn format_date_ago(date: chrono::DateTime<chrono::Utc>) -> String {
    let formatted = chrono_humanize::HumanTime::from(date).to_text_en(
        chrono_humanize::Accuracy::Rough,
        chrono_humanize::Tense::Past,
    );
    // lmao
    if formatted == "now ago" {
        "now".to_string()
    } else {
        formatted
    }
}

/// Format a date.
pub fn format_date(date: chrono::DateTime<chrono::Utc>) -> String {
    format!("{}", date)
}

/// Format a date, showing both the concrete date and the "n days ago"-format.
pub fn format_date_detailed(date: chrono::DateTime<chrono::Utc>) -> String {
    format!("{} ({})", format_date(date), format_date_ago(date))
}

/// Format a number into the 1st, 2nd, 3rd, 4th,... format
pub fn format_count(num: i32) -> String {
    match num {
        1 => "1st".to_string(),
        2 => "2nd".to_string(),
        3 => "3rd".to_string(),
        _ => format!("{}th", num),
    }
}

/// Validate that a string is a valid URL.
pub fn validate_url(value: &str) -> bool {
    url::Url::parse(value)
        .map(|url| !url.scheme().is_empty() && url.host().is_some() && url.domain().is_some())
        .unwrap_or(false)
}
