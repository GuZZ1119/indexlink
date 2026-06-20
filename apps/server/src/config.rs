use std::{
    env,
    net::{IpAddr, SocketAddr},
    num::ParseIntError,
    time::Duration,
};

use axum::http::HeaderValue;

const DEFAULT_HOST: &str = "0.0.0.0";
const DEFAULT_PORT: &str = "8080";
const DEFAULT_MAX_CONNECTIONS: &str = "10";
const DEFAULT_CONNECT_TIMEOUT_SECONDS: &str = "5";

#[derive(Debug)]
pub(crate) struct Config {
    pub(crate) address: SocketAddr,
    pub(crate) database_url: String,
    pub(crate) database_max_connections: u32,
    pub(crate) database_connect_timeout: Duration,
    pub(crate) cors_allowed_origins: Vec<HeaderValue>,
}

impl Config {
    pub(crate) fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|name| env::var(name).ok())
    }

    fn from_lookup(mut lookup: impl FnMut(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let host = value_or_default(&mut lookup, "APP_HOST", DEFAULT_HOST);
        let port = parse_u16(
            "APP_PORT",
            &value_or_default(&mut lookup, "APP_PORT", DEFAULT_PORT),
        )?;
        let ip = host
            .parse::<IpAddr>()
            .map_err(|_| ConfigError::InvalidHost)?;

        let database_url = lookup("DATABASE_URL").ok_or(ConfigError::MissingDatabaseUrl)?;
        if database_url.trim().is_empty() {
            return Err(ConfigError::MissingDatabaseUrl);
        }

        let database_max_connections = parse_u32(
            "DATABASE_MAX_CONNECTIONS",
            &value_or_default(
                &mut lookup,
                "DATABASE_MAX_CONNECTIONS",
                DEFAULT_MAX_CONNECTIONS,
            ),
        )?;
        if database_max_connections == 0 {
            return Err(ConfigError::NonPositive("DATABASE_MAX_CONNECTIONS"));
        }

        let timeout_seconds = parse_u64(
            "DATABASE_CONNECT_TIMEOUT_SECONDS",
            &value_or_default(
                &mut lookup,
                "DATABASE_CONNECT_TIMEOUT_SECONDS",
                DEFAULT_CONNECT_TIMEOUT_SECONDS,
            ),
        )?;
        if timeout_seconds == 0 {
            return Err(ConfigError::NonPositive("DATABASE_CONNECT_TIMEOUT_SECONDS"));
        }

        let cors_allowed_origins = lookup("CORS_ALLOWED_ORIGINS")
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|origin| !origin.is_empty())
            .map(|origin| {
                origin
                    .parse::<HeaderValue>()
                    .map_err(|_| ConfigError::InvalidCorsOrigin)
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            address: SocketAddr::new(ip, port),
            database_url,
            database_max_connections,
            database_connect_timeout: Duration::from_secs(timeout_seconds),
            cors_allowed_origins,
        })
    }
}

fn value_or_default(
    lookup: &mut impl FnMut(&str) -> Option<String>,
    name: &'static str,
    default: &'static str,
) -> String {
    lookup(name).unwrap_or_else(|| default.to_owned())
}

fn parse_u16(name: &'static str, value: &str) -> Result<u16, ConfigError> {
    value
        .parse()
        .map_err(|source| ConfigError::InvalidInteger { name, source })
}

fn parse_u32(name: &'static str, value: &str) -> Result<u32, ConfigError> {
    value
        .parse()
        .map_err(|source| ConfigError::InvalidInteger { name, source })
}

fn parse_u64(name: &'static str, value: &str) -> Result<u64, ConfigError> {
    value
        .parse()
        .map_err(|source| ConfigError::InvalidInteger { name, source })
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ConfigError {
    #[error("DATABASE_URL must be set")]
    MissingDatabaseUrl,
    #[error("APP_HOST must be a valid IP address")]
    InvalidHost,
    #[error("{name} must be a valid integer")]
    InvalidInteger {
        name: &'static str,
        #[source]
        source: ParseIntError,
    },
    #[error("{0} must be greater than zero")]
    NonPositive(&'static str),
    #[error("CORS_ALLOWED_ORIGINS contains an invalid origin")]
    InvalidCorsOrigin,
}

#[cfg(test)]
mod tests {
    use super::*;

    const DATABASE_URL: &str = "postgres://indexlink:indexlink@localhost/indexlink";

    fn parse(values: &[(&str, &str)]) -> Result<Config, ConfigError> {
        Config::from_lookup(|name| {
            values
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| (*value).to_owned())
        })
    }

    #[test]
    fn minimal_configuration_uses_documented_defaults() {
        let config = parse(&[("DATABASE_URL", DATABASE_URL)]).unwrap();

        assert_eq!(config.address, "0.0.0.0:8080".parse().unwrap());
        assert_eq!(config.database_url, DATABASE_URL);
        assert_eq!(config.database_max_connections, 10);
        assert_eq!(config.database_connect_timeout, Duration::from_secs(5));
        assert!(config.cors_allowed_origins.is_empty());
    }

    #[test]
    fn custom_network_and_pool_values_are_parsed() {
        let config = parse(&[
            ("DATABASE_URL", DATABASE_URL),
            ("APP_HOST", "127.0.0.1"),
            ("APP_PORT", "0"),
            ("DATABASE_MAX_CONNECTIONS", "23"),
            ("DATABASE_CONNECT_TIMEOUT_SECONDS", "17"),
        ])
        .unwrap();

        assert_eq!(config.address, "127.0.0.1:0".parse().unwrap());
        assert_eq!(config.database_max_connections, 23);
        assert_eq!(config.database_connect_timeout, Duration::from_secs(17));
    }

    #[test]
    fn missing_database_url_is_rejected() {
        assert!(matches!(parse(&[]), Err(ConfigError::MissingDatabaseUrl)));
    }

    #[test]
    fn blank_database_url_is_rejected() {
        assert!(matches!(
            parse(&[("DATABASE_URL", "  ")]),
            Err(ConfigError::MissingDatabaseUrl)
        ));
    }

    #[test]
    fn invalid_host_is_rejected() {
        assert!(matches!(
            parse(&[("DATABASE_URL", DATABASE_URL), ("APP_HOST", "localhost")]),
            Err(ConfigError::InvalidHost)
        ));
    }

    #[test]
    fn invalid_port_is_rejected_with_variable_name() {
        let error = parse(&[("DATABASE_URL", DATABASE_URL), ("APP_PORT", "eight")])
            .expect_err("non-numeric port must fail");

        assert!(matches!(
            error,
            ConfigError::InvalidInteger {
                name: "APP_PORT",
                ..
            }
        ));
    }

    #[test]
    fn invalid_max_connections_is_rejected() {
        let error = parse(&[
            ("DATABASE_URL", DATABASE_URL),
            ("DATABASE_MAX_CONNECTIONS", "many"),
        ])
        .expect_err("non-numeric pool size must fail");

        assert!(matches!(
            error,
            ConfigError::InvalidInteger {
                name: "DATABASE_MAX_CONNECTIONS",
                ..
            }
        ));
    }

    #[test]
    fn zero_max_connections_is_rejected() {
        let error = parse(&[
            ("DATABASE_URL", DATABASE_URL),
            ("DATABASE_MAX_CONNECTIONS", "0"),
        ])
        .expect_err("zero pool size must fail");

        assert!(matches!(
            error,
            ConfigError::NonPositive("DATABASE_MAX_CONNECTIONS")
        ));
    }

    #[test]
    fn invalid_connect_timeout_is_rejected() {
        let error = parse(&[
            ("DATABASE_URL", DATABASE_URL),
            ("DATABASE_CONNECT_TIMEOUT_SECONDS", "soon"),
        ])
        .expect_err("non-numeric timeout must fail");

        assert!(matches!(
            error,
            ConfigError::InvalidInteger {
                name: "DATABASE_CONNECT_TIMEOUT_SECONDS",
                ..
            }
        ));
    }

    #[test]
    fn zero_connect_timeout_is_rejected() {
        let error = parse(&[
            ("DATABASE_URL", DATABASE_URL),
            ("DATABASE_CONNECT_TIMEOUT_SECONDS", "0"),
        ])
        .expect_err("zero timeout must fail");

        assert!(matches!(
            error,
            ConfigError::NonPositive("DATABASE_CONNECT_TIMEOUT_SECONDS")
        ));
    }

    #[test]
    fn single_cors_origin_is_parsed() {
        let config = parse(&[
            ("DATABASE_URL", DATABASE_URL),
            ("CORS_ALLOWED_ORIGINS", "https://app.example"),
        ])
        .unwrap();

        assert_eq!(
            config.cors_allowed_origins,
            vec![HeaderValue::from_static("https://app.example")]
        );
    }

    #[test]
    fn multiple_cors_origins_are_trimmed() {
        let config = parse(&[
            ("DATABASE_URL", DATABASE_URL),
            (
                "CORS_ALLOWED_ORIGINS",
                " https://one.example, https://two.example ",
            ),
        ])
        .unwrap();

        assert_eq!(
            config.cors_allowed_origins,
            vec![
                HeaderValue::from_static("https://one.example"),
                HeaderValue::from_static("https://two.example")
            ]
        );
    }

    #[test]
    fn empty_cors_entries_are_filtered() {
        let config = parse(&[
            ("DATABASE_URL", DATABASE_URL),
            ("CORS_ALLOWED_ORIGINS", ", ,https://app.example,,"),
        ])
        .unwrap();

        assert_eq!(
            config.cors_allowed_origins,
            vec![HeaderValue::from_static("https://app.example")]
        );
    }

    #[test]
    fn invalid_cors_header_value_is_rejected() {
        assert!(matches!(
            parse(&[
                ("DATABASE_URL", DATABASE_URL),
                ("CORS_ALLOWED_ORIGINS", "https://ok.example\nbad"),
            ]),
            Err(ConfigError::InvalidCorsOrigin)
        ));
    }

    #[test]
    fn configuration_errors_do_not_expose_database_url() {
        let secret_url = "postgres://private-user:private-password@internal/database";
        let error = parse(&[("DATABASE_URL", secret_url), ("APP_PORT", "invalid")])
            .expect_err("invalid port must fail");
        let display = error.to_string();

        assert_eq!(display, "APP_PORT must be a valid integer");
        assert!(!display.contains(secret_url));
        assert!(!display.contains("private-password"));
    }
}
