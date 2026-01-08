pub(crate) mod config;
pub(crate) mod dashboard;
pub(crate) mod service;
mod http;
mod log;
mod openai_compat;
mod request_body;
mod response;
mod server;
mod sse;
mod sqlite;
mod upstream;
mod usage;

use std::{
    collections::HashMap,
    sync::{atomic::AtomicUsize, Arc},
};

struct ProxyState {
    config: config::ProxyConfig,
    client: reqwest::Client,
    log: Arc<log::LogWriter>,
    cursors: HashMap<String, Vec<AtomicUsize>>,
}

struct RequestMeta {
    stream: bool,
    model: Option<String>,
}
