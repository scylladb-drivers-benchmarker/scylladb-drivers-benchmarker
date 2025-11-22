use std::fmt::Debug;

use serde::de::DeserializeOwned;

pub trait Configuration {
    fn name(&self) -> String;
}

pub trait ConfigurationList: DeserializeOwned+ Debug + Clone + Eq {
    type ConfigType: Configuration;
    fn configs(&self) -> impl Iterator<Item = Self::ConfigType>;
    fn find_config(&self, config_name: &str) -> Option<Self::ConfigType> {
        self.configs().find(|config| config.name() == config_name)
    }
}
