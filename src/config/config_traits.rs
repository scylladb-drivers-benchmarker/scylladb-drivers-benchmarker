use std::fmt::Debug;

use serde::de::DeserializeOwned;

pub trait Configuration {
    type ConfigListType: ConfigurationList<ConfigType = Self>;
    fn benchmark_name(&self) -> String;
}

pub trait ConfigurationList: DeserializeOwned + Debug + Clone + Eq {
    type ConfigType: Configuration<ConfigListType = Self>;
    fn configs(&self) -> impl Iterator<Item = Self::ConfigType>;
    fn find_config(&self, benchmark_name: impl AsRef<str>) -> Option<Self::ConfigType> {
        self.configs()
            .find(|config| config.benchmark_name() == benchmark_name.as_ref())
    }
}
