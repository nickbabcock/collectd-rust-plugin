#![allow(dead_code)]
#![allow(unused)]
#[macro_use]
extern crate serde;
#[macro_use]
extern crate failure;
extern crate collectd_plugin;

#[cfg(test)]
#[macro_use]
extern crate serde_derive;

use failure::Error;
use std::fmt::{self, Display};

use collectd_plugin::{ConfigItem, ConfigValue};
use serde::de::{self, Deserialize, DeserializeSeed, Visitor, SeqAccess,
                MapAccess, EnumAccess, VariantAccess, IntoDeserializer};

pub type Result<T> = std::result::Result<T, Err2>;

#[derive(Debug)]
pub struct Err2(Error);

impl de::Error for Err2 {
    fn custom<T: Display>(msg: T) -> Self {
        Err2(format_err!("{}", msg.to_string()))
    }
}

impl ::std::error::Error for Err2 {
    fn description(&self) -> &str {
        "an error"
    }
}

impl Display for Err2 {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("hu")
    }
}

pub struct Deserializer<'a> {
    input: &'a [ConfigItem<'a>],
    item: Option<&'a ConfigItem<'a>>,
    value: Option<&'a ConfigValue<'a>>,
}

impl<'a> Deserializer<'a> {
    pub fn from_collectd(input: &'a [ConfigItem]) -> Self {
        Deserializer {
            input: input,
            item: None,
            value: None,
        }
    }

    fn grab_val(&self) -> Result<&ConfigValue> {
        let vs = &self.item.unwrap().values;
        if vs.len() != 1 {
            return Err(Err2(format_err!("Expecting values to hold a single value")))
        }

        return Ok(&vs[0])
    }

    fn grab_string(&self) -> Result<&str> {
        if let ConfigValue::String(x) = *self.grab_val()? {
            Ok(x)
        } else {
            Err(Err2(format_err!("Expected string")))
        }
    }

    fn grab_bool(&self) -> Result<bool> {
        if let ConfigValue::Boolean(x) = *self.grab_val()? {
            Ok(x)
        } else {
            Err(Err2(format_err!("Expected boolean")))
        }
    }

    fn grab_number(&self) -> Result<f64> {
        if let ConfigValue::Number(x) = *self.grab_val()? {
            Ok(x)
        } else {
            Err(Err2(format_err!("Expected number")))
        }
    }
}

pub fn from_collectd<'a, T>(s: &'a [ConfigItem<'a>]) -> Result<T>
    where T: Deserialize<'a>
{
    let mut deserializer = Deserializer::from_collectd(s);
    let t = T::deserialize(&mut deserializer)?;
    if deserializer.input.is_empty() {
        Ok(t)
    } else {
        Err(Err2(format_err!("Trailing items")))
    }
}

impl<'de: 'a, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Err2;

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        self.grab_bool().and_then(|x| visitor.visit_bool(x))
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        self.grab_string().and_then(|x| visitor.visit_string(String::from(x)))
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        self.grab_number().and_then(|x| visitor.visit_i8(x as i8))
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        self.grab_number().and_then(|x| visitor.visit_i16(x as i16))
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        self.grab_number().and_then(|x| visitor.visit_i32(x as i32))
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        self.grab_number().and_then(|x| visitor.visit_i64(x as i64))
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        self.grab_number().and_then(|x| visitor.visit_u8(x as u8))
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        self.grab_number().and_then(|x| visitor.visit_u16(x as u16))
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        self.grab_number().and_then(|x| visitor.visit_u32(x as u32))
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        self.grab_number().and_then(|x| visitor.visit_u64(x as u64))
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        self.grab_number().and_then(|x| visitor.visit_f32(x as f32))
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        self.grab_number().and_then(|x| visitor.visit_f64(x))
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_borrowed_str(self.item.unwrap().key)
    }

    fn deserialize_struct<V>(
        mut self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V
    ) -> Result<V::Value>
        where V: Visitor<'de>
    {
		let value = visitor.visit_map(FieldSeparated::new(&mut self))?;
        Ok(value)
    }

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        unimplemented!()
    }

    forward_to_deserialize_any! {
        char bytes str
        byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map enum ignored_any
    }
}

struct FieldSeparated<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> FieldSeparated<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        FieldSeparated { de: de }
    }
}

impl<'de, 'a> MapAccess<'de> for FieldSeparated<'a, 'de> {
    type Error = Err2;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
        where K: DeserializeSeed<'de>
    {
        // Check if there are no more entries.
		if self.de.input.is_empty() {
            return Ok(None)
        }

		self.de.item = Some(&self.de.input[0]);
        self.de.input = &self.de.input[1..];

        seed.deserialize(&mut *self.de).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
        where V: DeserializeSeed<'de>
    {
        // Deserialize a map value.
        seed.deserialize(&mut *self.de)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_bool() {
        #[derive(Deserialize, PartialEq, Eq, Debug)]
        struct MyStruct {
            my_bool: bool
        };

        let items = vec![
            ConfigItem {
                key: "my_bool",
                values: vec![
                    ConfigValue::Boolean(true)
                ],
                children: vec![],
            }
        ];

        let actual = from_collectd(&items).unwrap();
        assert_eq!(MyStruct { my_bool: true }, actual);
    }

    #[test]
    fn simple_number() {
        #[derive(Deserialize, PartialEq, Eq, Debug)]
        struct MyStruct {
            my_int: i8
        };

        let items = vec![
            ConfigItem {
                key: "my_int",
                values: vec![
                    ConfigValue::Number(1.0)
                ],
                children: vec![],
            }
        ];

        let actual = from_collectd(&items).unwrap();
        assert_eq!(MyStruct { my_int: 1 }, actual);
    }

    #[test]
    fn simple_string() {
        #[derive(Deserialize, PartialEq, Eq, Debug)]
        struct MyStruct {
            my_string: String
        };

        let items = vec![
            ConfigItem {
                key: "my_string",
                values: vec![
                    ConfigValue::String("HEY")
                ],
                children: vec![],
            }
        ];

        let actual = from_collectd(&items).unwrap();
        assert_eq!(MyStruct { my_string: String::from("HEY") }, actual);
    }
}
