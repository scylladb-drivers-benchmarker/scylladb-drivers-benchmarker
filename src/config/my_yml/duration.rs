use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::{fmt, time::Duration};

impl From<Duration> for MyDuration {
    fn from(value: Duration) -> Self {
        MyDuration { duration: value }
    }
}
impl From<MyDuration> for Duration {
    fn from(value: MyDuration) -> Self {
        value.duration
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MyDuration {
    duration: Duration,
}

impl Serialize for MyDuration {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let seconds = self.duration.as_secs();
        let nanos = self.duration.subsec_nanos();
        let mut output: String = String::new();

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
            if nanos % 1000000 == 0 {
                push_part((nanos / 1000000) as u64, "ms");
            } else {
                push_part(nanos as u64, "ns");
            }
        }

        if output.is_empty() {
            output = "0s".to_owned();
        }

        serializer.serialize_str(&output)
    }
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
        let mut working_str = string;
        let split = std::iter::from_fn(|| -> Option<Result<(&str, &str), E>> {
            println!("{}", working_str);
            if working_str.is_empty() {
                return None;
            }

            let patterns = ["ns", "ms", "s", "m", "h"];

            for pattern in patterns {
                let Some(before_pat) = working_str.strip_suffix(pattern) else {
                    continue;
                };

                if let Some((rest, integer)) = before_pat
                    .rfind(|c: char| !c.is_ascii_digit())
                    .map(|i| (&before_pat[..i + 1], &before_pat[i + 1..]))
                {
                    working_str = rest;
                    return Some(Ok((integer, pattern)));
                } else {
                    working_str = "";
                    return Some(Ok((before_pat, pattern)));
                }
            }
            return Some(Err(E::custom(format!(
                "unrecognized unit. Unparsed: {}",
                working_str
            ))));
        });

        let mut output = Duration::ZERO;
        for res in split {
            let (num_str, unit) = res?;

            let number: u64 = num_str
                .parse()
                .map_err(|_| E::custom(format!("parsing of {} failed", num_str)))?;

            match unit {
                "h" => output += Duration::from_secs(number * 60 * 60),
                "m" => output += Duration::from_secs(number * 60),
                "s" => output += Duration::from_secs(number),
                "ms" => output += Duration::from_millis(number),
                "ns" => output += Duration::from_nanos(number),
                _ => {
                    panic!("Unhandled unit");
                }
            }
        }
        Ok(output)
    }
}

impl<'de> Deserialize<'de> for MyDuration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer
            .deserialize_str(DurationVisitor)
            .map(MyDuration::from)
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use crate::config::my_yml::duration::MyDuration;

    fn test(duration: Duration, expected: &str) {
        let my_duration = MyDuration { duration };
        print!("{}", expected);
        assert_eq!(serde_yml::to_string(&my_duration).unwrap(), expected);
        assert_eq!(my_duration, serde_yml::from_str(expected).unwrap());
    }

    #[test]
    fn serde_test_single_duration() {
        test(Duration::from_secs(2), "'2s'\n");
        test(Duration::from_millis(13), "'13ms'\n");
        test(Duration::from_nanos(7), "'7ns'\n");
        test(Duration::from_secs(4 * 60), "'4m'\n");
        test(Duration::from_secs(20 * 60 * 60), "'20h'\n");
    }
    #[test]
    fn serde_test_combined_duration() {
        test(
            Duration::from_secs(2) + Duration::from_millis(13),
            "'2s13ms'\n",
        );
        test(
            Duration::from_nanos(7) + Duration::from_secs(4 * 60),
            "'4m7ns'\n",
        );
        test(Duration::from_secs(20 * 60 * 60 + 5), "'20h5s'\n");
    }
}
