use dashmap::DashMap;
use std::{net::Ipv4Addr, sync::Arc};

use crate::{AppResult, apperror::AppError::XCustomError};

pub type Store = Arc<DashMap<String, StoreValue>>;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum StoreValue {
    Text(String),
    Bytes(Vec<u8>),
    Integer(i64),
    Float(f64),
    Bool(bool),
    Host(KnownHost),
}

#[derive(Debug, Clone)]
pub struct KnownHost {
    pub hostname: String,
    pub addr: Ipv4Addr,
    pub quic_port: u16,
    pub encoded_cert: String,
}

pub fn get_attribute(dashmap: &Store, key: &str) -> AppResult<StoreValue> {
    dashmap
        .get(key)
        .ok_or(XCustomError("Attribute not found"))
        .map(|v| v.clone())
}

pub fn get_text(dashmap: &Store, key: &str) -> AppResult<String> {
    match get_attribute(dashmap, key)? {
        StoreValue::Text(t) => Ok(t),
        _ => Err(XCustomError("Expected Text")),
    }
}

pub fn get_host(dashmap: &Store, key: &str) -> AppResult<KnownHost> {
    match get_attribute(dashmap, key)? {
        StoreValue::Host(h) => Ok(h),
        _ => Err(XCustomError("Expected Host")),
    }
}

pub fn get_bool(dashmap: &Store, key: &str) -> AppResult<bool> {
    match get_attribute(dashmap, key)? {
        StoreValue::Bool(b) => Ok(b),
        _ => Err(XCustomError("Expected Bool")),
    }
}

pub fn get_int(dashmap: &Store, key: &str) -> AppResult<i64> {
    match get_attribute(dashmap, key)? {
        StoreValue::Integer(i) => Ok(i),
        _ => Err(XCustomError("Expected Integer")),
    }
}
