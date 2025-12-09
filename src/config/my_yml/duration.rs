use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::fmt;
use std::time::Duration;

impl From<Duration> for MyDuration {
    fn from(value: Duration) -> Self {
        MyDuration::Only(value)
    }
}
impl From<MyDuration> for Duration {
    fn from(value: MyDuration) -> Self {
        let MyDuration::Only(duration) = value;
        duration
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MyDuration {
    #[serde(with = "self", untagged)]
    Only(std::time::Duration),
}

pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let seconds = duration.as_secs();
    let nanos = duration.subsec_nanos();
    let mut output = String::new();

    let mut push_part = |value: u64, unit| {
        if value > 0 {
            output += value.to_string().as_str();
            output += unit;
        }
    };

    let mut remaining = seconds;

    push_part(remaining / (60 * 60), "h");
    remaining %= 60 * 60;

    push_part(remaining / 60, "m");
    remaining %= 60;

    push_part(remaining, "s");

    if nanos > 0 {
        push_part(nanos as u64, "ns");
    }

    if output.is_empty() {
        output = "0s".to_owned();
    }

    serializer.serialize_str(&output)
}

struct DurationVisitor;

impl<'de> de::Visitor<'de> for DurationVisitor {
    type Value = Duration;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a duration string like '1h30m5s'")
    }

    fn visit_str<E>(self, string: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let mut output = Duration::ZERO;
        let pattern = ['h', 'm', 's'];
        for part in string.split_inclusive(pattern) {
            let number_str = part.strip_suffix(pattern).unwrap();
            let number: u64 = number_str
                .parse()
                .map_err(|_| E::custom(format!("parsing of {} failed", number_str)))?;
            let unit = part.strip_prefix(number_str).unwrap();
            match unit {
                "h" => output += Duration::from_secs(number * 60 * 60),
                "m" => output += Duration::from_secs(number * 60),
                "s" => output += Duration::from_secs(number),
                "ns" => output += Duration::from_nanos(number),
                _ => {
                    return Err(E::custom(format!("unrecognized unit: {}", unit)));
                }
            }
        }
        Ok(output)
    }
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_str(DurationVisitor)
}
