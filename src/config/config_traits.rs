use std::fmt::Debug;

use serde::de::DeserializeOwned;

// Ciekawe podejście co do traktowania konfiguracji
pub trait Configuration {
    type ConfigListType: ConfigurationList<ConfigType = Self>;
    fn benchmark_name(&self) -> String;
}

pub trait ConfigurationList: DeserializeOwned + Debug + Clone + Eq {
    type ConfigType: Configuration<ConfigListType = Self>;
    // Z jakiego powodu jest to iterator?
    // Jedyne miejsce w którym jest to użyte, to find_config. 
    // Czy hash mapa nie będzie tutaj miała więcej sensu?
    fn configs(&self) -> impl Iterator<Item = Self::ConfigType>;
    fn find_config(&self, benchmark_name: impl AsRef<str>) -> Option<Self::ConfigType> {
        self.configs()
            .find(|config| config.benchmark_name() == benchmark_name.as_ref())
    }
}
