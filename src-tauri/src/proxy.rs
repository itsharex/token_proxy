pub(crate) mod config;
mod http;
mod log;
mod openai_compat;
mod response;
mod server;
mod sse;
mod upstream;
mod usage;

pub(crate) use server::spawn;

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

