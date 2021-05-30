mod deconfig;
mod errors;
mod level;
pub use self::errors::*;
pub use self::level::*;

use self::deconfig::*;
use self::errors::Error;
use crate::api::ConfigItem;
use serde::de::{self, Deserialize, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::forward_to_deserialize_any;

/// Serde documentation shadows the std's Result type which can be really confusing for Rust
/// newcomers, so we compromise by creating an alias but prefixing with "De" to make it standout.
pub type DeResult<T> = Result<T, Error>;

/// Keeps track of the current state of deserialization.
#[derive(Debug, Clone)]
enum DeType<'a> {
    /// The values that had the associated key
    Item(&'a str, Vec<DeConfig<'a>>),

    /// Vector of key, values tuples and the index that we are at in deserializing them
    Struct(Vec<(&'a str, Vec<DeConfig<'a>>)>, usize),

    /// The index of the current object we're deserializing and associated vector of objects
    Seq(Vec<DeConfig<'a>>, usize),
}

pub struct Deserializer<'a> {
    depth: Vec<DeType<'a>>,
}

impl<'a> Deserializer<'a> {
    fn from_collectd(input: Vec<(&'a str, Vec<DeConfig<'a>>)>) -> Self {
        Deserializer {
            depth: vec![DeType::Struct(input, 0)],
        }
    }

    fn current(&self) -> DeResult<&DeType<'a>> {
        if self.depth.is_empty() {
            return Err(Error(DeError::NoMoreValuesLeft));
        }

        Ok(&self.depth[self.depth.len() - 1])
    }

    fn grab_val(&self) -> DeResult<&DeConfig<'a>> {
        match *self.current()? {
            DeType::Item(_, ref values) => {
                if values.len() != 1 {
                    return Err(Error(DeError::ExpectSingleValue));
                }

                Ok(&values[0])
            }
            DeType::Seq(ref items, ind) => Ok(&items[ind]),
            _ => Err(Error(DeError::ExpectSingleValue)),
        }
    }

    fn grab_string(&self) -> DeResult<&'a str> {
        if let DeConfig::String(x) = *self.grab_val()? {
            Ok(x)
        } else {
            Err(Error(DeError::ExpectString))
        }
    }

    fn grab_bool(&self) -> DeResult<bool> {
        if let DeConfig::Boolean(x) = *self.grab_val()? {
            Ok(x)
        } else {
            Err(Error(DeError::ExpectBoolean))
        }
    }

    fn grab_number(&self) -> DeResult<f64> {
        if let DeConfig::Number(x) = *self.grab_val()? {
            Ok(x)
        } else {
            Err(Error(DeError::ExpectNumber))
        }
    }

    fn pop(&mut self) {
        self.depth.pop();
    }

    fn push(&mut self, pos: usize) {
        // Find the parent -- it's either the tail element of depth or penultimate.
        let cur = if pos == 0 { 1 } else { 2 };
        let end = self.depth.len() - cur;

        let mut t = None;
        if let DeType::Struct(ref values, _ind) = self.depth[end] {
            t = Some(values[pos].clone());
        }

        if let Some((key, vs)) = t {
            if pos == 0 {
                self.depth.push(DeType::Item(key, vs));
            } else {
                let end = self.depth.len() - 1;
                self.depth[end] = DeType::Item(key, vs);
            }
        }
    }

    fn push_seq(&mut self, pos: usize) {
        // Find the parent -- it's either the tail element of depth or penultimate.
        let cur = if pos == 0 { 1 } else { 2 };
        let end = self.depth.len() - cur;

        let mut t = None;
        if let DeType::Item(_key, ref values) = self.depth[end] {
            t = Some(values.clone());
        }

        if let Some(vs) = t {
            if pos == 0 {
                self.depth.push(DeType::Seq(vs, pos));
            } else {
                let end = self.depth.len() - 1;
                self.depth[end] = DeType::Seq(vs, pos);
            }
        }
    }
}

pub fn from_collectd<'a, T>(s: &'a [ConfigItem<'a>]) -> DeResult<T>
where
    T: Deserialize<'a>,
{
    let props = from_config(s);
    let mut deserializer = Deserializer::from_collectd(props);
    T::deserialize(&mut deserializer)
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_bool<V>(self, visitor: V) -> DeResult<V::Value>
    where
        V: Visitor<'de>,
    {
        self.grab_bool().and_then(|x| visitor.visit_bool(x))
    }

    fn deserialize_string<V>(self, visitor: V) -> DeResult<V::Value>
    where
        V: Visitor<'de>,
    {
        self.grab_string()
            .and_then(|x| visitor.visit_string(String::from(x)))
    }

    fn deserialize_str<V>(self, visitor: V) -> DeResult<V::Value>
    where
        V: Visitor<'de>,
    {
        self.grab_string()
            .and_then(|x| visitor.visit_borrowed_str(x))
    }

    fn deserialize_i8<V>(self, visitor: V) -> DeResult<V::Value>
    where
        V: Visitor<'de>,
    {
        self.grab_number().and_then(|x| visitor.visit_i8(x as i8))
    }

    fn deserialize_i16<V>(self, visitor: V) -> DeResult<V::Value>
    where
        V: Visitor<'de>,
    {
        self.grab_number().and_then(|x| visitor.visit_i16(x as i16))
    }

    fn deserialize_i32<V>(self, visitor: V) -> DeResult<V::Value>
    where
        V: Visitor<'de>,
    {
        self.grab_number().and_then(|x| visitor.visit_i32(x as i32))
    }

    fn deserialize_i64<V>(self, visitor: V) -> DeResult<V::Value>
    where
        V: Visitor<'de>,
    {
        self.grab_number().and_then(|x| visitor.visit_i64(x as i64))
    }

    fn deserialize_u8<V>(self, visitor: V) -> DeResult<V::Value>
    where
        V: Visitor<'de>,
    {
        self.grab_number().and_then(|x| visitor.visit_u8(x as u8))
    }

    fn deserialize_u16<V>(self, visitor: V) -> DeResult<V::Value>
    where
        V: Visitor<'de>,
    {
        self.grab_number().and_then(|x| visitor.visit_u16(x as u16))
    }

    fn deserialize_u32<V>(self, visitor: V) -> DeResult<V::Value>
    where
        V: Visitor<'de>,
    {
        self.grab_number().and_then(|x| visitor.visit_u32(x as u32))
    }

    fn deserialize_u64<V>(self, visitor: V) -> DeResult<V::Value>
    where
        V: Visitor<'de>,
    {
        self.grab_number().and_then(|x| visitor.visit_u64(x as u64))
    }

    fn deserialize_f32<V>(self, visitor: V) -> DeResult<V::Value>
    where
        V: Visitor<'de>,
    {
        self.grab_number().and_then(|x| visitor.visit_f32(x as f32))
    }

    fn deserialize_f64<V>(self, visitor: V) -> DeResult<V::Value>
    where
        V: Visitor<'de>,
    {
        self.grab_number().and_then(|x| visitor.visit_f64(x))
    }

    fn deserialize_option<V>(self, visitor: V) -> DeResult<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    fn deserialize_char<V>(self, visitor: V) -> DeResult<V::Value>
    where
        V: Visitor<'de>,
    {
        self.grab_string().and_then(|x| {
            if x.len() != 1 {
                Err(Error(DeError::ExpectChar(String::from(x))))
            } else {
                visitor.visit_char(x.chars().next().unwrap())
            }
        })
    }

    fn deserialize_identifier<V>(self, visitor: V) -> DeResult<V::Value>
    where
        V: Visitor<'de>,
    {
        let v = &self.depth[self.depth.len() - 1];
        match *v {
            DeType::Item(key, _) => visitor.visit_borrowed_str(key),
            DeType::Seq(ref x, pos) if pos == 0 => {
                if let DeConfig::String(ss) = x[pos] {
                    visitor.visit_borrowed_str(ss)
                } else {
                    Err(de::Error::custom("expected the first element, when deserializing for an identifier, to be a string"))
                }
            }
            _ => Err(de::Error::custom(
                "expected an item when deserializing identifier",
            )),
        }
    }

    fn deserialize_seq<V>(mut self, visitor: V) -> DeResult<V::Value>
    where
        V: Visitor<'de>,
    {
        let len = if let DeType::Item(_key, ref v) = *self.current()? {
            v.len()
        } else {
            return Err(de::Error::custom("expected an item when deserializing seq"));
        };

        visitor.visit_seq(SeqSeparated::new(&mut self, len))
    }

    fn deserialize_struct<V>(
        mut self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> DeResult<V::Value>
    where
        V: Visitor<'de>,
    {
        // A small hack to remember if we dive into a sequence's children. Since we're going down
        // two levels, we need to remember to pop back up when were down the children.
        let mut to_pop = false;

        let t = match self.current()?.clone() {
            DeType::Struct(ref values, _ind) => Some(values.len()),
            DeType::Seq(ref values, ind) => {
                if let DeConfig::Object(ref obj) = values[ind] {
                    // Push the children onto the stack
                    let s = DeType::Struct(obj.clone(), 0);
                    self.depth.push(s);
                    to_pop = true;
                    Some(obj.len())
                } else {
                    return Err(Error(DeError::ExpectObject));
                }
            }
            _ => None,
        };

        let res = visitor.visit_map(FieldSeparated::new(&mut self, t.unwrap_or(0)))?;
        if to_pop {
            self.pop();
        }
        Ok(res)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> DeResult<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_none()
    }

    fn deserialize_any<V>(self, _visitor: V) -> DeResult<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error(DeError::DataTypeNotSupported))
    }

    fn deserialize_newtype_struct<V>(self, _name: &str, visitor: V) -> DeResult<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_enum<V>(
        self,
        _name: &str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> DeResult<V::Value>
    where
        V: Visitor<'de>,
    {
        if let DeType::Item(_key, ref v) = self.current()?.clone() {
            if v.len() != 1 {
                return Err(de::Error::custom(
                    "expected enum item to have a single item list",
                ));
            }

            // With a unit variant enum, it needs to take a look at the identifier, so in the case
            // deserializing JSON {"level": "INFO"} serde will attempt to deserialize two identifiers
            // in a row (first "level" and then "INFO"). Since we don't want to re-read the "level"
            // identifier we simulate sequence. Not the most elegant solution, but it works.
            self.push_seq(0);
            let res = visitor.visit_enum(UnitVariantAccess::new(self))?;
            self.pop();
            Ok(res)
        } else {
            Err(Error(DeError::ExpectStruct))
        }
    }

    forward_to_deserialize_any! {
        bytes
        byte_buf unit unit_struct tuple
        tuple_struct map
    }
}

struct UnitVariantAccess<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de: 'a> UnitVariantAccess<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        UnitVariantAccess { de }
    }
}

impl<'de, 'a> de::EnumAccess<'de> for UnitVariantAccess<'a, 'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> DeResult<(V::Value, Self)>
    where
        V: de::DeserializeSeed<'de>,
    {
        let variant = seed.deserialize(&mut *self.de)?;
        Ok((variant, self))
    }
}

impl<'de, 'a> de::VariantAccess<'de> for UnitVariantAccess<'a, 'de> {
    type Error = Error;

    fn unit_variant(self) -> DeResult<()> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, _seed: T) -> DeResult<T::Value>
    where
        T: de::DeserializeSeed<'de>,
    {
        Err(de::Error::custom("newtype variant"))
    }

    fn tuple_variant<V>(self, _len: usize, _visitor: V) -> DeResult<V::Value>
    where
        V: de::Visitor<'de>,
    {
        Err(de::Error::custom("tuple variant"))
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], _visitor: V) -> DeResult<V::Value>
    where
        V: de::Visitor<'de>,
    {
        Err(de::Error::custom("struct variant"))
    }
}

struct FieldSeparated<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    item_count: usize,
    item_pos: usize,
}

impl<'a, 'de> FieldSeparated<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>, item_count: usize) -> Self {
        FieldSeparated {
            de,
            item_pos: 0,
            item_count,
        }
    }
}

impl<'de, 'a> MapAccess<'de> for FieldSeparated<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> DeResult<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        // Check if there are no more entries.
        if self.item_pos == self.item_count {
            if self.item_count != 0 {
                self.de.pop();
            }
            return Ok(None);
        }

        self.de.push(self.item_pos);
        self.item_pos += 1;
        seed.deserialize(&mut *self.de).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> DeResult<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.de)
    }
}

struct SeqSeparated<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    item_count: usize,
    item_pos: usize,
}

impl<'a, 'de> SeqSeparated<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>, item_count: usize) -> Self {
        SeqSeparated {
            de,
            item_count,
            item_pos: 0,
        }
    }
}

impl<'de, 'a> SeqAccess<'de> for SeqSeparated<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> DeResult<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        if self.item_pos == self.item_count {
            if self.item_count != 0 {
                self.de.pop();
            }
            return Ok(None);
        }

        self.de.push_seq(self.item_pos);
        self.item_pos += 1;
        seed.deserialize(&mut *self.de).map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::super::ConfigValue;
    use super::*;
    use crate::api::LogLevel;
    use serde::Deserialize;

    #[test]
    fn test_serde_simple_bool() {
        #[derive(Deserialize, PartialEq, Eq, Debug)]
        struct MyStruct {
            my_bool: bool,
        }

        let items = vec![ConfigItem {
            key: "my_bool",
            values: vec![ConfigValue::Boolean(true)],
            children: vec![],
        }];

        let actual = from_collectd(&items).unwrap();
        assert_eq!(MyStruct { my_bool: true }, actual);
    }

    #[test]
    fn test_serde_empty_bool() {
        #[derive(Deserialize, PartialEq, Eq, Debug)]
        struct MyStruct {
            my_bool: Option<bool>,
        }

        let actual = from_collectd(Default::default()).unwrap();
        assert_eq!(MyStruct { my_bool: None }, actual);
    }

    #[test]
    fn test_serde_simple_number() {
        #[derive(Deserialize, PartialEq, Eq, Debug)]
        struct MyStruct {
            my_int: i8,
        }

        let items = vec![ConfigItem {
            key: "my_int",
            values: vec![ConfigValue::Number(1.0)],
            children: vec![],
        }];

        let actual = from_collectd(&items).unwrap();
        assert_eq!(MyStruct { my_int: 1 }, actual);
    }

    #[test]
    fn test_serde_simple_string() {
        #[derive(Deserialize, PartialEq, Eq, Debug)]
        struct MyStruct {
            my_string: String,
        }

        let items = vec![ConfigItem {
            key: "my_string",
            values: vec![ConfigValue::String("HEY")],
            children: vec![],
        }];

        let actual = from_collectd(&items).unwrap();
        assert_eq!(
            MyStruct {
                my_string: String::from("HEY"),
            },
            actual
        );
    }

    #[test]
    fn test_serde_simple_str() {
        #[derive(Deserialize, PartialEq, Eq, Debug)]
        struct MyStruct<'a> {
            my_string: &'a str,
        }

        let items = vec![ConfigItem {
            key: "my_string",
            values: vec![ConfigValue::String("HEY")],
            children: vec![],
        }];

        let actual = from_collectd(&items).unwrap();
        assert_eq!(MyStruct { my_string: "HEY" }, actual);
    }

    #[test]
    fn test_serde_multiple() {
        #[derive(Deserialize, PartialEq, Eq, Debug)]
        struct MyStruct {
            my_bool: bool,
            my_string: String,
        }

        let items = vec![
            ConfigItem {
                key: "my_bool",
                values: vec![ConfigValue::Boolean(true)],
                children: vec![],
            },
            ConfigItem {
                key: "my_string",
                values: vec![ConfigValue::String("/")],
                children: vec![],
            },
        ];

        let actual = from_collectd(&items).unwrap();
        assert_eq!(
            MyStruct {
                my_bool: true,
                my_string: String::from("/"),
            },
            actual
        );
    }

    #[test]
    fn test_serde_bool_vec() {
        #[derive(Deserialize, PartialEq, Eq, Debug)]
        struct MyStruct {
            my_bool: Vec<bool>,
        }

        let items = vec![ConfigItem {
            key: "my_bool",
            values: vec![ConfigValue::Boolean(true), ConfigValue::Boolean(false)],
            children: vec![],
        }];

        let actual = from_collectd(&items).unwrap();
        assert_eq!(
            MyStruct {
                my_bool: vec![true, false],
            },
            actual
        );
    }

    #[test]
    fn test_serde_bool_vec_sep() {
        #[derive(Deserialize, PartialEq, Eq, Debug)]
        struct MyStruct {
            my_bool: Vec<bool>,
        }

        let items = vec![
            ConfigItem {
                key: "my_bool",
                values: vec![ConfigValue::Boolean(true)],
                children: vec![],
            },
            ConfigItem {
                key: "my_bool",
                values: vec![ConfigValue::Boolean(false)],
                children: vec![],
            },
        ];

        let actual = from_collectd(&items).unwrap();
        assert_eq!(
            MyStruct {
                my_bool: vec![true, false],
            },
            actual
        );
    }

    #[test]
    fn test_serde_multiple_vec() {
        #[derive(Deserialize, PartialEq, Debug)]
        struct MyStruct {
            my_bool: Vec<bool>,
            my_num: Vec<f64>,
        }

        let items = vec![
            ConfigItem {
                key: "my_bool",
                values: vec![ConfigValue::Boolean(true)],
                children: vec![],
            },
            ConfigItem {
                key: "my_bool",
                values: vec![ConfigValue::Boolean(false)],
                children: vec![],
            },
            ConfigItem {
                key: "my_num",
                values: vec![ConfigValue::Number(10.0), ConfigValue::Number(12.0)],
                children: vec![],
            },
        ];

        let actual = from_collectd(&items).unwrap();
        assert_eq!(
            MyStruct {
                my_bool: vec![true, false],
                my_num: vec![10.0, 12.0],
            },
            actual
        );
    }

    #[test]
    fn test_serde_options() {
        #[derive(Deserialize, PartialEq, Eq, Debug)]
        struct MyStruct {
            my_bool: Option<bool>,
            my_string: Option<String>,
        }

        let items = vec![ConfigItem {
            key: "my_bool",
            values: vec![ConfigValue::Boolean(true)],
            children: vec![],
        }];

        let actual = from_collectd(&items).unwrap();
        assert_eq!(
            MyStruct {
                my_bool: Some(true),
                my_string: None,
            },
            actual
        );
    }

    #[test]
    fn test_serde_log_level() {
        #[derive(Deserialize, PartialEq, Eq, Debug)]
        struct MyStruct {
            warn: LogLevel,
            warning: LogLevel,
            err: LogLevel,
            error: LogLevel,
            debug: LogLevel,
            info: LogLevel,
            notice: LogLevel,
        }

        let items = vec![
            ConfigItem {
                key: "warn",
                values: vec![ConfigValue::String("warn")],
                children: vec![],
            },
            ConfigItem {
                key: "warning",
                values: vec![ConfigValue::String("Warning")],
                children: vec![],
            },
            ConfigItem {
                key: "err",
                values: vec![ConfigValue::String("ErR")],
                children: vec![],
            },
            ConfigItem {
                key: "error",
                values: vec![ConfigValue::String("Error")],
                children: vec![],
            },
            ConfigItem {
                key: "debug",
                values: vec![ConfigValue::String("debug")],
                children: vec![],
            },
            ConfigItem {
                key: "info",
                values: vec![ConfigValue::String("INFO")],
                children: vec![],
            },
            ConfigItem {
                key: "notice",
                values: vec![ConfigValue::String("notice")],
                children: vec![],
            },
        ];

        let actual = from_collectd(&items).unwrap();
        assert_eq!(
            MyStruct {
                warn: LogLevel::Warning,
                warning: LogLevel::Warning,
                err: LogLevel::Error,
                error: LogLevel::Error,
                debug: LogLevel::Debug,
                info: LogLevel::Info,
                notice: LogLevel::Notice,
            },
            actual
        );
    }

    #[test]
    fn test_serde_char() {
        #[derive(Deserialize, PartialEq, Eq, Debug)]
        struct MyStruct {
            my_char: char,
        }

        let items = vec![ConfigItem {
            key: "my_char",
            values: vec![ConfigValue::String("/")],
            children: vec![],
        }];

        let actual = from_collectd(&items).unwrap();
        assert_eq!(MyStruct { my_char: '/' }, actual);
    }

    #[test]
    fn test_serde_ignore() {
        #[derive(Deserialize, PartialEq, Eq, Debug)]
        struct MyStruct {
            my_char: char,
        }

        let items = vec![
            ConfigItem {
                key: "my_char",
                values: vec![ConfigValue::String("/")],
                children: vec![],
            },
            ConfigItem {
                key: "my_boat",
                values: vec![ConfigValue::String("/")],
                children: vec![],
            },
        ];

        let actual = from_collectd(&items).unwrap();
        assert_eq!(MyStruct { my_char: '/' }, actual);
    }

    #[test]
    fn test_serde_nested() {
        #[derive(Deserialize, PartialEq, Eq, Debug)]
        struct MyPort {
            port: i32,
        }

        #[derive(Deserialize, PartialEq, Eq, Debug)]
        struct MyStruct {
            ports: Vec<MyPort>,
        }

        let items = vec![
            ConfigItem {
                key: "ports",
                values: vec![],
                children: vec![ConfigItem {
                    key: "port",
                    values: vec![ConfigValue::Number(2003.0)],
                    children: vec![],
                }],
            },
            ConfigItem {
                key: "ports",
                values: vec![],
                children: vec![ConfigItem {
                    key: "port",
                    values: vec![ConfigValue::Number(2004.0)],
                    children: vec![],
                }],
            },
        ];

        let actual = from_collectd(&items).unwrap();
        assert_eq!(
            MyStruct {
                ports: vec![MyPort { port: 2003 }, MyPort { port: 2004 }],
            },
            actual
        );
    }

    #[test]
    fn test_serde_nested_multiple() {
        #[derive(Deserialize, PartialEq, Eq, Debug)]
        struct MyAddress {
            port: i32,
            host: String,
        }

        #[derive(Deserialize, PartialEq, Eq, Debug)]
        struct MyStruct {
            address: Vec<MyAddress>,
        }

        let items = vec![
            ConfigItem {
                key: "address",
                values: vec![],
                children: vec![
                    ConfigItem {
                        key: "port",
                        values: vec![ConfigValue::Number(2003.0)],
                        children: vec![],
                    },
                    ConfigItem {
                        key: "host",
                        values: vec![ConfigValue::String("localhost")],
                        children: vec![],
                    },
                ],
            },
            ConfigItem {
                key: "address",
                values: vec![],
                children: vec![
                    ConfigItem {
                        key: "host",
                        values: vec![ConfigValue::String("127.0.0.1")],
                        children: vec![],
                    },
                    ConfigItem {
                        key: "port",
                        values: vec![ConfigValue::Number(2004.0)],
                        children: vec![],
                    },
                ],
            },
        ];

        let actual = from_collectd(&items).unwrap();
        assert_eq!(
            MyStruct {
                address: vec![
                    MyAddress {
                        host: String::from("localhost"),
                        port: 2003,
                    },
                    MyAddress {
                        host: String::from("127.0.0.1"),
                        port: 2004,
                    },
                ],
            },
            actual
        );
    }

    #[test]
    fn test_serde_new_type() {
        #[derive(Deserialize, PartialEq, Eq, Debug)]
        struct MyNew(String);

        #[derive(Deserialize, PartialEq, Eq, Debug)]
        struct MyStruct {
            it: MyNew,
        }

        let items = vec![ConfigItem {
            key: "it",
            values: vec![ConfigValue::String("INFO")],
            children: vec![],
        }];

        let actual = from_collectd(&items).unwrap();
        assert_eq!(
            MyStruct {
                it: MyNew(String::from("INFO"))
            },
            actual
        );
    }

    #[test]
    fn test_log_serde_enum() {
        use log::Level;

        #[derive(Deserialize, PartialEq, Eq, Debug)]
        #[serde(deny_unknown_fields)]
        struct MyStruct {
            it: Level,
            sentinel: i32,
        }

        let items = vec![
            ConfigItem {
                key: "it",
                values: vec![ConfigValue::String("INFO")],
                children: vec![],
            },
            ConfigItem {
                key: "sentinel",
                values: vec![ConfigValue::Number(2003.0)],
                children: vec![],
            },
        ];

        let actual = from_collectd(&items).unwrap();
        assert_eq!(
            MyStruct {
                it: Level::Info,
                sentinel: 2003
            },
            actual
        );
    }

    #[test]
    fn test_serde_enum() {
        #[derive(PartialEq, Eq, Debug)]
        enum MyEnum {
            Foo,
        }

        use serde::de::{self, Deserializer};

        impl<'de> Deserialize<'de> for MyEnum {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let s = String::deserialize(deserializer)?;
                match s.as_str() {
                    "Foo" => Ok(MyEnum::Foo),
                    _ => Err(de::Error::custom(format!("bad type: {}", s))),
                }
            }
        }

        #[derive(Deserialize, PartialEq, Eq, Debug)]
        #[serde(deny_unknown_fields)]
        struct MyStruct {
            it: MyEnum,
            sentinel: i32,
        }

        let items = vec![
            ConfigItem {
                key: "it",
                values: vec![ConfigValue::String("Foo")],
                children: vec![],
            },
            ConfigItem {
                key: "sentinel",
                values: vec![ConfigValue::Number(2003.0)],
                children: vec![],
            },
        ];

        let actual = from_collectd(&items).unwrap();
        assert_eq!(
            MyStruct {
                it: MyEnum::Foo,
                sentinel: 2003
            },
            actual
        );
    }
}
