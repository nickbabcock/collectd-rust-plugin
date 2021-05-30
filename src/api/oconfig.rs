use crate::bindings::{
    oconfig_item_t, oconfig_value_s__bindgen_ty_1, oconfig_value_t, OCONFIG_TYPE_BOOLEAN,
    OCONFIG_TYPE_NUMBER, OCONFIG_TYPE_STRING,
};
use crate::errors::ConfigError;
use std::ffi::CStr;
use std::slice;

/// A parsed value from the Collectd config
#[derive(Debug, PartialEq, Clone)]
pub enum ConfigValue<'a> {
    /// Numeric value
    Number(f64),

    /// True / false, on / off
    Boolean(bool),

    /// Contents enclosed in quote
    String(&'a str),
}

/// Parsed key, values, children objects from the Collectd config.
#[derive(Debug, PartialEq, Clone)]
pub struct ConfigItem<'a> {
    /// Key of the field, does not have to be unique
    pub key: &'a str,

    /// Values on the same line as the key
    pub values: Vec<ConfigValue<'a>>,

    /// Sub elements
    pub children: Vec<ConfigItem<'a>>,
}

impl<'a> ConfigValue<'a> {
    /// # Safety
    ///
    /// Assumed that the any pointers are non null
    pub unsafe fn from(value: &oconfig_value_t) -> Result<ConfigValue<'_>, ConfigError> {
        match value.value {
            oconfig_value_s__bindgen_ty_1 { string }
                if value.type_ == OCONFIG_TYPE_STRING as i32 =>
            {
                Ok(ConfigValue::String(
                    CStr::from_ptr(string)
                        .to_str()
                        .map_err(ConfigError::StringDecode)?,
                ))
            }
            oconfig_value_s__bindgen_ty_1 { number }
                if value.type_ == OCONFIG_TYPE_NUMBER as i32 =>
            {
                Ok(ConfigValue::Number(number))
            }
            oconfig_value_s__bindgen_ty_1 { boolean }
                if value.type_ == OCONFIG_TYPE_BOOLEAN as i32 =>
            {
                Ok(ConfigValue::Boolean(boolean != 0))
            }
            _ => Err(ConfigError::UnknownType(value.type_)),
        }
    }
}

impl<'a> ConfigItem<'a> {
    /// # Safety
    ///
    /// Assumed that the pointer is non-null
    pub unsafe fn from<'b>(item: &'b oconfig_item_t) -> Result<ConfigItem<'b>, ConfigError> {
        let key = CStr::from_ptr(item.key)
            .to_str()
            .map_err(ConfigError::StringDecode)?;

        let values: Result<Vec<ConfigValue<'b>>, ConfigError> =
            slice::from_raw_parts(item.values, item.values_num as usize)
                .iter()
                .map(|x| ConfigValue::from(x))
                .collect();

        let children: Result<Vec<ConfigItem<'b>>, ConfigError> =
            slice::from_raw_parts(item.children, item.children_num as usize)
                .iter()
                .map(|x| ConfigItem::from(x))
                .collect();

        Ok(ConfigItem {
            key,
            values: values?,
            children: children?,
        })
    }
}
