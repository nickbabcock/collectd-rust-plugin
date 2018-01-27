use api::{ConfigItem, ConfigValue};
use std::collections::HashMap;

/// This looks just like `ConfigValue` except it add in the `Object` association. While collectd
/// differentiate between values and children, for simplicity, we don't. It's kinda like JSON this
/// way.
#[derive(Debug, PartialEq, Clone)]
pub enum DeConfig<'a> {
    Number(f64),
    Boolean(bool),
    String(&'a str),
    Object(Vec<(&'a str, Vec<DeConfig<'a>>)>),
}

/// Since a collectd config can (and often) contains multiple keys, we aggregate all instances of
/// the same key under a single key. Serde likes it this way. Won't run into duplicate key errors.
pub fn from_config<'a>(s: &'a [ConfigItem<'a>]) -> Vec<(&'a str, Vec<DeConfig<'a>>)> {
    let mut props: HashMap<&'a str, Vec<DeConfig<'a>>> = HashMap::new();
    for item in s {
        if !item.values.is_empty() {
            props
                .entry(item.key)
                .or_insert_with(Vec::new)
                .extend(item.values.iter().map(value_to_config));
        }

        if !item.children.is_empty() {
            props
                .entry(item.key)
                .or_insert_with(Vec::new)
                .push(de_config_item(&item.children[..]));
        }
    }

    props.into_iter().collect()
}

fn de_config_item<'a>(s: &'a [ConfigItem<'a>]) -> DeConfig<'a> {
    DeConfig::Object(from_config(s))
}

fn value_to_config<'a>(v: &'a ConfigValue) -> DeConfig<'a> {
    match *v {
        ConfigValue::Number(x) => DeConfig::Number(x),
        ConfigValue::Boolean(x) => DeConfig::Boolean(x),
        ConfigValue::String(x) => DeConfig::String(x),
    }
}
