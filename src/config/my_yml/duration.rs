use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::fmt;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MyDuration {
    #[serde(with = "self")]
    value: std::time::Duration
}

pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let seconds = duration.as_secs();
    let nanos = duration.subsec_nanos();
    let mut parts = Vec::new();

    let mut push_part = |value, unit| {
        if value > 0 {
            parts.push(format!("{}{}", value, unit));
        }
    };

    let mut remaining = seconds;

    push_part(remaining / (60 * 60), "h");
    remaining %= 60 * 60;

    push_part(remaining / 60, "m");
    remaining %= 60;

    push_part(remaining, "s");

    if nanos > 0 {
        if nanos % 1_000_000 == 0 {
            push_part(nanos as u64 / 1_000_000, "ms");
        } else {
            push_part(nanos as u64, "ns");
        }
    }

    let duration_str = if parts.is_empty() {
        "0s".to_owned()
    } else {
        parts.join("")
    };

    serializer.serialize_str(&duration_str)
}

struct DurationVisitor;

impl<'de> de::Visitor<'de> for DurationVisitor {
    type Value = Duration;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a duration string like '1h30m5s'")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let mut total_duration = Duration::new(0, 0);
        let mut current_value = 0u64;
        let mut current_unit = String::new();

        for c in value.chars() {
            if c.is_ascii_digit() {
                let digit = c.to_digit(10).unwrap() as u64;
                current_value = current_value
                    .checked_mul(10)
                    .and_then(|v| v.checked_add(digit))
                    .ok_or_else(|| E::custom("duration component overflow"))?;
            } else if c.is_ascii_alphabetic() {
                current_unit.push(c);

                match current_unit.as_str() {
                    "h" => total_duration += Duration::from_secs(current_value * 60 * 60),
                    "m" => total_duration += Duration::from_secs(current_value * 60),
                    "s" => total_duration += Duration::from_secs(current_value),
                    "ms" => total_duration += Duration::from_millis(current_value),
                    "us" => total_duration += Duration::from_micros(current_value),
                    "ns" => total_duration += Duration::from_nanos(current_value),
                    _ => {
                        return Err(E::custom(format!(
                            "unrecognized duration unit: {}",
                            current_unit
                        )));
                    }
                }

                current_value = 0;
                current_unit.clear();
            } else {
                return Err(E::custom(format!(
                    "invalid character in duration string: {}",
                    c
                )));
            }
        }

        if current_value > 0 || !current_unit.is_empty() {
            Err(E::custom(
                "duration string ended prematurely or had trailing value/unit",
            ))
        } else {
            Ok(total_duration)
        }
    }
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_str(DurationVisitor)
}
