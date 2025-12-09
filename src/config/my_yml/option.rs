use std::fmt::Debug;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

impl<T: Serialize + for<'a> Deserialize<'a> + Debug + Clone + Eq + PartialEq> From<MyOption<T>>
    for Option<T>
{
    fn from(value: MyOption<T>) -> Self {
        match value {
            MyOption::Some(some) => Some(some),
            MyOption::None => None,
        }
    }
}

impl<T: Serialize + for<'a> Deserialize<'a> + Debug + Clone + Eq + PartialEq> From<Option<T>>
    for MyOption<T>
{
    fn from(value: Option<T>) -> Self {
        match value {
            Some(some) => MyOption::Some(some),
            None => MyOption::None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
enum MyOption<T: Serialize + Debug + Clone + Eq + PartialEq> {
    #[serde(untagged)]
    None,
    #[serde(untagged)]
    Some(T),
}

pub fn serialize<
    S: Serializer,
    T: Serialize + for<'a> Deserialize<'a> + Debug + Clone + Eq + PartialEq,
>(
    input: &Option<T>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    MyOption::from(input.clone()).serialize(serializer)
}

pub fn deserialize<
    'de,
    D,
    T: Serialize + for<'a> Deserialize<'a> + Debug + Clone + Eq + PartialEq,
>(
    deserializer: D,
) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = MyOption::deserialize(deserializer)?;
    Ok(Option::from(s))
}
