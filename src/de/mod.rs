use failure::Error;
use std;
use std::fmt::{self, Display};

use api::{ConfigItem, ConfigValue};
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

enum DeType<'a> {
    Struct(&'a ConfigItem<'a>),
    Seq(&'a ConfigValue<'a>),
}

pub struct Deserializer<'a> {
    input: &'a [ConfigItem<'a>],
    depth: Vec<DeType<'a>>,
    root: bool,
}

impl<'a> Deserializer<'a> {
    pub fn from_collectd(input: &'a [ConfigItem]) -> Self {
        Deserializer {
            input: input,
            depth: vec![],
            root: true,
        }
    }

    fn current(&self) -> Result<&DeType> {
        if self.depth.is_empty() {
            return Err(Err2(format_err!("No more values left, this should never happen")));
        }

        Ok(&self.depth[self.depth.len() - 1])
    }

    fn struct_children(&self) -> Result<&[ConfigItem]> {
        let ind = self.depth.len() - 1;
        let a = &self.depth[ind];
        if let &DeType::Struct(ref item) = a {
            Ok(&item.children[..])
        } else {
            Err(Err2(format_err!("Expecting struct")))
        }
    }

    fn grab_val(&self) -> Result<&ConfigValue> {
        match self.current()? {
            &DeType::Struct(item) => {
                if item.values.len() != 1 {
                    return Err(Err2(format_err!("Expecting values to hold a single value")))
                }

                Ok(&item.values[0])
            },
            &DeType::Seq(item) => Ok(item),
        }
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
    T::deserialize(&mut deserializer)
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

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_some(self)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        self.grab_string().and_then(|x| {
            if x.len() != 1 {
                Err(Err2(format_err!("For character, expected string of length 1")))
            } else {
                visitor.visit_char(x.chars().next().unwrap())
            }
        })
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        let v = &self.depth[self.depth.len() - 1];
        if let &DeType::Struct(ref item) = v {
            if item.children.is_empty() || item.values.is_empty() {
                visitor.visit_borrowed_str(&item.key)
            } else {
                if let &ConfigValue::String(x) = &item.values[0] {
                    visitor.visit_borrowed_str(x)
                } else {
                    Err(Err2(format_err!("Expected string")))
                }
            }
        } else {
            Err(Err2(format_err!("Expecting struct")))
        }
    }

    fn deserialize_seq<V>(mut self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        if self.depth.is_empty() {
            return Err(Err2(format_err!("No more values left, this should never happen")));
        }

        match &self.depth[self.depth.len() - 1] {
            &DeType::Struct(item) => visitor.visit_seq(SeqSeparated::new(&mut self, &item.values)),
            &DeType::Seq(item) => Err(Err2(format_err!("Did not expect sequence"))),
        }
    }

    fn deserialize_struct<V>(
        mut self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V
    ) -> Result<V::Value>
        where V: Visitor<'de>
    {
        if self.root {
            self.root = false;
            visitor.visit_map(FieldSeparated::new(&mut self, self.input))
        } else {
            let ind = self.depth.len() - 1;
            let children = if let &DeType::Struct(ref item) = &self.depth[ind] {
                Ok(&item.children[..])
            } else {
                Err(Err2(format_err!("Expecting struct")))
            };

            visitor.visit_map(FieldSeparated::new(&mut self, children?))
        }
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_none()
    }

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        unimplemented!()
    }

    forward_to_deserialize_any! {
        bytes str
        byte_buf unit unit_struct newtype_struct tuple
        tuple_struct map enum
    }
}

struct FieldSeparated<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
	items: &'de [ConfigItem<'de>],
    first: bool,
}

impl<'a, 'de> FieldSeparated<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>, items: &'de [ConfigItem<'de>]) -> Self {
        FieldSeparated { de: de, first: true, items: items }
    }
}

impl<'de, 'a> MapAccess<'de> for FieldSeparated<'a, 'de> {
    type Error = Err2;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
        where K: DeserializeSeed<'de>
    {
        // Check if there are no more entries.
		if self.items.is_empty() {
            self.de.depth.pop().unwrap();
            return Ok(None)
        }

        if self.first {
            self.de.depth.push(DeType::Struct(&self.items[0]));
            self.first = false;
        } else {
            let ind = self.de.depth.len() - 1;
            self.de.depth[ind] = DeType::Struct(&self.items[0]);
        }

        self.items = &self.items[1..];
        seed.deserialize(&mut *self.de).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
        where V: DeserializeSeed<'de>
    {
        seed.deserialize(&mut *self.de)
    }
}

struct SeqSeparated<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
	values: &'de [ConfigValue<'de>],
    first: bool,
}

impl<'a, 'de> SeqSeparated<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>, v: &'de [ConfigValue<'de>]) -> Self {
        SeqSeparated { de: de, values: v, first: true }
    }
}

impl<'de, 'a> SeqAccess<'de> for SeqSeparated<'a, 'de> {
    type Error = Err2;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
        where T: DeserializeSeed<'de>
    {
        if self.values.is_empty() {
            self.de.depth.pop().unwrap();
            return Ok(None);
        }

        if self.first {
            self.de.depth.push(DeType::Seq(&self.values[0]));
            self.first = false;
        } else {
            let ind = self.de.depth.len() - 1;
            self.de.depth[ind] = DeType::Seq(&self.values[0]);
        }

        self.values = &self.values[1..];
        seed.deserialize(&mut *self.de).map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serde_simple_bool() {
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
    fn test_serde_simple_number() {
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
    fn test_serde_simple_string() {
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

    #[test]
    fn test_serde_bool_vec() {
        #[derive(Deserialize, PartialEq, Eq, Debug)]
        struct MyStruct {
            my_bool: Vec<bool>
        };

        let items = vec![
            ConfigItem {
                key: "my_bool",
                values: vec![
                    ConfigValue::Boolean(true),
                    ConfigValue::Boolean(false),
                ],
                children: vec![],
            }
        ];

        let actual = from_collectd(&items).unwrap();
        assert_eq!(MyStruct { my_bool: vec![true, false] }, actual);
    }

    #[test]
    fn test_serde_options() {
        #[derive(Deserialize, PartialEq, Eq, Debug)]
        struct MyStruct {
            my_bool: Option<bool>,
            my_string: Option<String>,
        };

        let items = vec![
            ConfigItem {
                key: "my_bool",
                values: vec![
                    ConfigValue::Boolean(true),
                ],
                children: vec![],
            }
        ];

        let actual = from_collectd(&items).unwrap();
        assert_eq!(MyStruct { my_bool: Some(true), my_string: None }, actual);
    }

    #[test]
    fn test_serde_char() {
        #[derive(Deserialize, PartialEq, Eq, Debug)]
        struct MyStruct {
            my_char: char,
        };

        let items = vec![
            ConfigItem {
                key: "my_char",
                values: vec![
                    ConfigValue::String("/"),
                ],
                children: vec![],
            }
        ];

        let actual = from_collectd(&items).unwrap();
        assert_eq!(MyStruct { my_char: '/' }, actual);
    }

    #[test]
    fn test_serde_ignore() {
        #[derive(Deserialize, PartialEq, Eq, Debug)]
        struct MyStruct {
            my_char: char,
        };

        let items = vec![
            ConfigItem {
                key: "my_char",
                values: vec![
                    ConfigValue::String("/"),
                ],
                children: vec![],
            },
            ConfigItem {
                key: "my_boat",
                values: vec![
                    ConfigValue::String("/"),
                ],
                children: vec![],
            }
        ];

        let actual = from_collectd(&items).unwrap();
        assert_eq!(MyStruct { my_char: '/' }, actual);
    }

    #[test]
    fn test_serde_graphite() {
        #[derive(Deserialize, PartialEq, Eq, Debug)]
        struct Node {
            port: i32,
        };

        #[derive(Deserialize, PartialEq, Eq, Debug)]
        struct MyStruct {
            example: Node,
        };

        let items = vec![
            ConfigItem {
                key: "node",
                values: vec![
                    ConfigValue::String("example")
                ],
                children: vec![
                    ConfigItem {
                        key: "port",
                        values: vec![
                            ConfigValue::Number(2003.0)
                        ],
                        children: vec![],
                    },
                ],
            }
        ];

        let actual = from_collectd(&items).unwrap();
        assert_eq!(MyStruct { example: Node { port: 2003} }, actual);
    }
}
