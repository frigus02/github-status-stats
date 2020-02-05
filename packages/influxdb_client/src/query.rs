use super::FieldValue;
use serde::{de, Deserialize, Serialize};
use std::marker::PhantomData;
use std::vec::IntoIter;

#[derive(Debug, Serialize)]
pub(crate) struct Query<'a> {
    pub q: &'a str,
}

pub struct Rows<T>
where
    T: de::DeserializeOwned,
{
    columns: Vec<String>,
    values: IntoIter<Vec<FieldValue>>,
    phantom: PhantomData<T>,
}

impl<T> Rows<T>
where
    T: de::DeserializeOwned,
{
    fn new(columns: Vec<String>, values: Vec<Vec<FieldValue>>) -> Self {
        Self {
            columns,
            values: values.into_iter(),
            phantom: PhantomData,
        }
    }
}

impl<T> Iterator for Rows<T>
where
    T: de::DeserializeOwned,
{
    type Item = Result<T, rowde::Error>;

    fn next(&mut self) -> Option<Result<T, rowde::Error>> {
        match self.values.next() {
            Some(row) => {
                let mut deserializer = rowde::Deserializer::from_row(&self.columns, row);
                Some(T::deserialize(&mut deserializer))
            }
            None => None,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct QuerySeries {
    pub name: String,
    pub columns: Vec<String>,
    pub values: Vec<Vec<FieldValue>>,
}

impl QuerySeries {
    pub fn into_rows<T>(self) -> Rows<T>
    where
        T: de::DeserializeOwned,
    {
        Rows::new(self.columns, self.values)
    }
}

#[derive(Debug, Deserialize)]
pub struct QueryResult {
    pub statement_id: i32,
    pub error: Option<String>,
    pub series: Option<Vec<QuerySeries>>,
}

impl QueryResult {
    pub fn into_single_series(self) -> Result<QuerySeries, String> {
        if let Some(mut series_list) = self.series {
            let series = series_list.pop();
            if let Some(series) = series {
                if series_list.is_empty() {
                    Ok(series)
                } else {
                    Err("more than 1 series".to_owned())
                }
            } else {
                Err("zero series".to_owned())
            }
        } else {
            Err("no series".to_owned())
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct QueryResponse {
    pub results: Vec<QueryResult>,
}

impl QueryResponse {
    pub fn into_single_result(mut self) -> Result<QueryResult, String> {
        let result = self.results.pop();
        if let Some(result) = result {
            if self.results.is_empty() {
                Ok(result)
            } else {
                Err("more than 1 result".to_owned())
            }
        } else {
            Err("zero results".to_owned())
        }
    }
}

mod rowde {
    use super::FieldValue;
    use serde::de::Error as SerdeDeError;
    use serde::{de, forward_to_deserialize_any};
    use std::error::Error as StdError;
    use std::fmt::Display;
    use std::iter::Peekable;
    use std::slice::Iter;
    use std::vec::IntoIter;

    #[derive(Debug)]
    pub struct Error {
        message: String,
    }

    impl Display for Error {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
            formatter.write_str(&self.message)
        }
    }

    impl StdError for Error {}

    impl SerdeDeError for Error {
        fn custom<T>(msg: T) -> Self
        where
            T: Display,
        {
            Self {
                message: format!("{}", msg),
            }
        }
    }

    enum State {
        Initial,
        Column,
        Value,
    }

    pub struct Deserializer<'de> {
        columns: Peekable<Iter<'de, String>>,
        values: IntoIter<FieldValue>,
        state: State,
    }

    impl<'de> Deserializer<'de> {
        pub fn from_row(columns: &'de [String], values: Vec<FieldValue>) -> Self {
            Self {
                columns: columns.iter().peekable(),
                values: values.into_iter(),
                state: State::Initial,
            }
        }

        fn peek_column(&mut self) -> Option<&&String> {
            self.columns.peek()
        }
    }

    impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
        type Error = Error;

        fn deserialize_any<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: de::Visitor<'de>,
        {
            match self.state {
                State::Initial => {
                    self.state = State::Column;
                    visitor.visit_map(RowMap::new(&mut self))
                }
                State::Column => {
                    self.state = State::Value;
                    match self.columns.next() {
                        Some(column) => visitor.visit_str(column),
                        None => Err(Error::custom("no more columns")),
                    }
                }
                State::Value => {
                    self.state = State::Column;
                    match self.values.next() {
                        Some(field) => match field {
                            FieldValue::Boolean(v) => visitor.visit_bool(v),
                            FieldValue::Float(v) => visitor.visit_f64(v),
                            FieldValue::Integer(v) => visitor.visit_i64(v),
                            FieldValue::String(v) => visitor.visit_string(v),
                        },
                        None => Err(Error::custom("no more values")),
                    }
                }
            }
        }

        forward_to_deserialize_any! {
            bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }

    struct RowMap<'a, 'de: 'a> {
        de: &'a mut Deserializer<'de>,
    }

    impl<'a, 'de> RowMap<'a, 'de> {
        fn new(de: &'a mut Deserializer<'de>) -> Self {
            RowMap { de }
        }
    }

    impl<'de, 'a> de::MapAccess<'de> for RowMap<'a, 'de> {
        type Error = Error;

        fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
        where
            K: de::DeserializeSeed<'de>,
        {
            match self.de.peek_column() {
                Some(_) => seed.deserialize(&mut *self.de).map(Some),
                None => Ok(None),
            }
        }

        fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
        where
            V: de::DeserializeSeed<'de>,
        {
            seed.deserialize(&mut *self.de)
        }
    }
}
