use serde::de::{Deserialize, Deserializer, Error, MapAccess, Visitor};
use serde_json::from_str;
use std::{borrow::Cow, hint::black_box, time::Instant};

fn main() {
    // json with ~35655 lines of fields with backslashes
    let json = include_str!("../a.json");

    // json deserialization iteration count
    const N: usize = 1000;

    loop {
        // deserialize with `enum Key`
        let now = Instant::now();
        for _ in 0..N {
            let resp = black_box(from_str::<ResponseKey>(black_box(json))).unwrap();
        }
        println!("key: {}", now.elapsed().as_secs_f32());

        // deserialize with `Cow<'de, str>`
        let now = Instant::now();
        for _ in 0..N {
            let resp = black_box(from_str::<ResponseCow>(black_box(json))).unwrap();
        }
        println!("cow: {}\n", now.elapsed().as_secs_f32());
    }
}

/// Deserialization impl uses `enum Key`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponseKey {
    pub jsonrpc: (),
    pub id: (),
    pub payload: Result<(), ()>,
}

/// Deserialization impl uses `Cow<'de, str>`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponseCow {
    pub jsonrpc: (),
    pub id: (),
    pub payload: Result<(), ()>,
}

impl<'de> Deserialize<'de> for ResponseKey {
    fn deserialize<D: Deserializer<'de>>(der: D) -> Result<Self, D::Error> {
        enum Key {
            JsonRpc,
            Result,
            Error,
            Id,
            Unknown,
        }

        impl<'de> Deserialize<'de> for Key {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct KeyVisitor;

                impl Visitor<'_> for KeyVisitor {
                    type Value = Key;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str("`jsonrpc`, `result`, `error`, `id`")
                    }

                    fn visit_str<E>(self, string: &str) -> Result<Key, E>
                    where
                        E: Error,
                    {
                        match string {
                            "jsonrpc" => Ok(Key::JsonRpc),
                            "id" => Ok(Key::Id),
                            "result" => Ok(Key::Result),
                            "error" => Ok(Key::Error),
                            _ => Ok(Key::Unknown),
                        }
                    }
                }

                deserializer.deserialize_identifier(KeyVisitor)
            }
        }

        struct MapVisit;

        impl<'de> Visitor<'de> for MapVisit {
            type Value = ResponseKey;

            fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
                formatter.write_str("JSON-RPC 2.0 Response")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut jsonrpc = None;
                let mut payload = None;
                let mut id = None;

                while let Some(key) = map.next_key::<Key>()? {
                    match key {
                        Key::JsonRpc => jsonrpc = Some(map.next_value::<()>()?),
                        Key::Id => id = Some(map.next_value::<()>()?),
                        Key::Result => {
                            if payload.is_none() {
                                payload = Some(Ok(map.next_value::<()>()?));
                            } else {
                                return Err(serde::de::Error::duplicate_field("result/error"));
                            }
                        }
                        Key::Error => {
                            if payload.is_none() {
                                payload = Some(Err(map.next_value::<()>()?));
                            } else {
                                return Err(serde::de::Error::duplicate_field("result/error"));
                            }
                        }
                        Key::Unknown => {
                            map.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                }

                match (jsonrpc, id, payload) {
                    (Some(jsonrpc), Some(id), Some(payload)) => Ok(ResponseKey {
                        jsonrpc,
                        id,
                        payload,
                    }),

                    // No fields existed.
                    (None, None, None) => Err(Error::missing_field("jsonrpc + id + result/error")),

                    // Some field was missing.
                    (None, _, _) => Err(Error::missing_field("jsonrpc")),
                    (_, None, _) => Err(Error::missing_field("id")),
                    (_, _, None) => Err(Error::missing_field("result/error")),
                }
            }
        }

        const FIELDS: &[&str; 4] = &["jsonrpc", "id", "result", "error"];
        der.deserialize_struct("Response", FIELDS, MapVisit)
    }
}

impl<'de> Deserialize<'de> for ResponseCow {
    fn deserialize<D: Deserializer<'de>>(der: D) -> Result<Self, D::Error> {
        struct MapVisit;

        impl<'de> Visitor<'de> for MapVisit {
            type Value = ResponseCow;

            fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
                formatter.write_str("JSON-RPC 2.0 Response")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut jsonrpc = None;
                let mut payload = None;
                let mut id = None;

                while let Some(key) = map.next_key::<Cow<'de, str>>()? {
                    // hmm why is this always a Cow::Owned...?
                    // assert!(key, Cow::Owned(_));

                    match key.as_ref() {
                        "jsonrpc" => jsonrpc = Some(map.next_value::<()>()?),
                        "id" => id = Some(map.next_value::<()>()?),
                        "result" => {
                            if payload.is_none() {
                                payload = Some(Ok(map.next_value::<()>()?));
                            } else {
                                return Err(serde::de::Error::duplicate_field("result/error"));
                            }
                        }
                        "error" => {
                            if payload.is_none() {
                                payload = Some(Err(map.next_value::<()>()?));
                            } else {
                                return Err(serde::de::Error::duplicate_field("result/error"));
                            }
                        }
                        _ => {
                            map.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                }

                match (jsonrpc, id, payload) {
                    (Some(jsonrpc), Some(id), Some(payload)) => Ok(ResponseCow {
                        jsonrpc,
                        id,
                        payload,
                    }),

                    // No fields existed.
                    (None, None, None) => Err(Error::missing_field("jsonrpc + id + result/error")),

                    // Some field was missing.
                    (None, _, _) => Err(Error::missing_field("jsonrpc")),
                    (_, None, _) => Err(Error::missing_field("id")),
                    (_, _, None) => Err(Error::missing_field("result/error")),
                }
            }
        }

        const FIELDS: &[&str; 4] = &["jsonrpc", "id", "result", "error"];
        der.deserialize_struct("Response", FIELDS, MapVisit)
    }
}
