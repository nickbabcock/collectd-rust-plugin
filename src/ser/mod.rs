use super::Value;
use serde::{Serialize, Serializer};

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            Value::Counter(x) => serializer.serialize_u64(x),
            Value::Gauge(x) => serializer.serialize_f64(x),
            Value::Derive(x) => serializer.serialize_i64(x),
            Value::Absolute(x) => serializer.serialize_u64(x),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_test;

    use self::serde_test::{assert_ser_tokens, Token};
    use super::Value;

    #[test]
    fn test_ser_value_counter() {
        assert_ser_tokens(&Value::Counter(10), &[Token::U64(10)]);
    }

    #[test]
    fn test_ser_value_gauge() {
        assert_ser_tokens(&Value::Gauge(1.0), &[Token::F64(1.0)]);
    }

    #[test]
    fn test_ser_value_derive() {
        assert_ser_tokens(&Value::Derive(-5), &[Token::I64(-5)]);
    }

    #[test]
    fn test_ser_value_absolute() {
        assert_ser_tokens(&Value::Absolute(15), &[Token::U64(15)]);
    }
}
