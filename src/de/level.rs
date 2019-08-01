use crate::api::LogLevel;
use serde::de::{self, Deserialize, Deserializer, Visitor};
use std::fmt;

struct LogLevelVisitor;

impl<'de> Visitor<'de> for LogLevelVisitor {
    type Value = LogLevel;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ERROR | WARN | INFO | DEBUG | NOTICE")
    }

    fn visit_str<E>(self, s: &str) -> Result<LogLevel, E>
    where
        E: de::Error,
    {
        let upper = s.to_ascii_uppercase();
        match upper.as_str() {
            "INFO" => Ok(LogLevel::Info),
            "DEBUG" => Ok(LogLevel::Debug),
            "ERR" | "ERROR" => Ok(LogLevel::Error),
            "WARN" | "WARNING" => Ok(LogLevel::Warning),
            "NOTICE" => Ok(LogLevel::Notice),
            x => Err(E::custom(format!("Did not expect log level of: {}", x))),
        }
    }
}

impl<'de> Deserialize<'de> for LogLevel {
    fn deserialize<D>(deserializer: D) -> Result<LogLevel, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(LogLevelVisitor)
    }
}
