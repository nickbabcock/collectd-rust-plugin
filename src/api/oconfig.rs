use bindings::{oconfig_item_t, oconfig_value_t, OCONFIG_TYPE_STRING, OCONFIG_TYPE_NUMBER, OCONFIG_TYPE_BOOLEAN, oconfig_value_s__bindgen_ty_1};
use failure::{Error, ResultExt};
use std::ffi::CStr;
use std::slice;

pub enum ConfigValue<'a> {
    Number(f64),
    Boolean(bool),
    String(&'a str),
}

pub struct ConfigItem<'a> {
    key: &'a str,
    values: Vec<ConfigValue<'a>>,
    children: Vec<ConfigItem<'a>>,
}

impl<'a> ConfigValue<'a> {
    pub unsafe fn from<'b>(value: &'b oconfig_value_t) -> Result<ConfigValue<'b>, Error> {
        match value.value {
            oconfig_value_s__bindgen_ty_1 { string } if value.type_ == OCONFIG_TYPE_STRING as i32 =>
                Ok(ConfigValue::String(CStr::from_ptr(string).to_str().context("failed to decode config value string")?)),
            oconfig_value_s__bindgen_ty_1 { number } if value.type_ == OCONFIG_TYPE_NUMBER as i32 =>
                Ok(ConfigValue::Number(number)),
            oconfig_value_s__bindgen_ty_1 { boolean } if value.type_ == OCONFIG_TYPE_BOOLEAN as i32 =>
                Ok(ConfigValue::Boolean(boolean != 0)),
            _ => Err(format_err!("Unrecognized value: {}", value.type_)),
        }
    }
}

impl<'a> ConfigItem<'a> {
    pub unsafe fn from<'b>(item: &'b oconfig_item_t) -> Result<ConfigItem<'b>, Error> {
        let key = CStr::from_ptr(item.key)
            .to_str()
            .context("failed to decode config item key")?;

        let values: Result<Vec<ConfigValue<'b>>, Error> =
            slice::from_raw_parts(item.values, item.values_num as usize)
            .iter()
            .map(|x| ConfigValue::from(x))
            .collect();

        let children: Result<Vec<ConfigItem<'b>>, Error> =
            slice::from_raw_parts(item.children, item.children_num as usize)
            .iter()
            .map(|x| ConfigItem::from(x))
            .collect();

        Ok(ConfigItem {
            key: key,
            values: values?,
            children: children?,
        })
    }
}
