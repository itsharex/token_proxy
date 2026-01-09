use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use tiktoken_rs::{cl100k_base, o200k_base, CoreBPE};
use tokio::sync::watch;

const RATE_WINDOW: Duration = Duration::from_secs(1);

#[derive(Clone)]
pub(crate) struct TokenRateTracker {
    inner: Arc<Mutex<TrackerInner>>,
    activity_tx: watch::Sender<u64>,
}

struct TrackerInner {
    next_id: u64,
    requests: HashMap<u64, RequestWindow>,
}

struct RequestWindow {
    events: VecDeque<TokenEvent>,
}

struct TokenEvent {
    ts: Instant,
    input: u64,
    output: u64,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct TokenRateSnapshot {
    pub(crate) input: u64,
    pub(crate) output: u64,
    pub(crate) total: u64,
}

pub(crate) struct RequestTokenTracker {
    id: u64,
    tracker: TokenRateTracker,
    model: Option<String>,
}

impl TokenRateTracker {
    pub(crate) fn new() -> Arc<Self> {
        let (activity_tx, _activity_rx) = watch::channel(0u64);
        Arc::new(Self {
            inner: Arc::new(Mutex::new(TrackerInner {
                next_id: 1,
                requests: HashMap::new(),
            })),
            activity_tx,
        })
    }

    pub(crate) fn subscribe_activity(&self) -> watch::Receiver<u64> {
        self.activity_tx.subscribe()
    }

    pub(crate) fn notify_activity(&self) {
        let next = self.activity_tx.borrow().wrapping_add(1);
        let _ = self.activity_tx.send(next);
    }

    pub(crate) fn register(
        &self,
        model: Option<String>,
        input_tokens: Option<u64>,
    ) -> RequestTokenTracker {
        let id = {
            let mut guard = self.inner.lock().expect("token rate lock poisoned");
            let id = guard.next_id;
            guard.next_id = guard.next_id.saturating_add(1);
            guard.requests.insert(id, RequestWindow::new());
            id
        };

        let mut tracker = RequestTokenTracker {
            id,
            tracker: self.clone(),
            model,
        };
        if let Some(tokens) = input_tokens {
            tracker.add_input_tokens(tokens);
        }
        self.notify_activity();
        tracker
    }

    pub(crate) fn snapshot(&self) -> TokenRateSnapshot {
        let now = Instant::now();
        let mut guard = self.inner.lock().expect("token rate lock poisoned");
        let mut input = 0u64;
        let mut output = 0u64;
        for window in guard.requests.values_mut() {
            window.prune(now);
            let (i, o) = window.sum();
            input = input.saturating_add(i);
            output = output.saturating_add(o);
        }
        TokenRateSnapshot {
            input,
            output,
            total: input.saturating_add(output),
        }
    }

    pub(crate) fn has_active_requests(&self) -> bool {
        let now = Instant::now();
        let mut guard = self.inner.lock().expect("token rate lock poisoned");
        for window in guard.requests.values_mut() {
            window.prune(now);
        }
        !guard.requests.is_empty()
    }

    fn record(&self, id: u64, input: u64, output: u64) {
        if input == 0 && output == 0 {
            return;
        }
        let mut guard = self.inner.lock().expect("token rate lock poisoned");
        let Some(window) = guard.requests.get_mut(&id) else {
            return;
        };
        window.push(TokenEvent {
            ts: Instant::now(),
            input,
            output,
        });
    }

    fn unregister(&self, id: u64) {
        let mut guard = self.inner.lock().expect("token rate lock poisoned");
        guard.requests.remove(&id);
    }
}

impl RequestWindow {
    fn new() -> Self {
        Self {
            events: VecDeque::new(),
        }
    }

    fn push(&mut self, event: TokenEvent) {
        self.events.push_back(event);
        self.prune(Instant::now());
    }

    fn prune(&mut self, now: Instant) {
        while let Some(front) = self.events.front() {
            if now.duration_since(front.ts) <= RATE_WINDOW {
                break;
            }
            self.events.pop_front();
        }
    }

    fn sum(&self) -> (u64, u64) {
        let mut input = 0u64;
        let mut output = 0u64;
        for event in &self.events {
            input = input.saturating_add(event.input);
            output = output.saturating_add(event.output);
        }
        (input, output)
    }
}

impl RequestTokenTracker {
    pub(crate) fn add_input_tokens(&mut self, tokens: u64) {
        self.tracker.record(self.id, tokens, 0);
    }

    pub(crate) fn add_output_text(&mut self, text: &str) {
        let tokens = estimate_text_tokens(self.model.as_deref(), text);
        self.tracker.record(self.id, 0, tokens);
    }
}

impl Drop for RequestTokenTracker {
    fn drop(&mut self) {
        self.tracker.unregister(self.id);
    }
}

pub(crate) fn estimate_text_tokens(model: Option<&str>, text: &str) -> u64 {
    if text.is_empty() {
        return 0;
    }
    let bpe = bpe_for_model(model);
    bpe.encode_with_special_tokens(text).len() as u64
}

fn bpe_for_model(model: Option<&str>) -> &'static CoreBPE {
    if matches_o200k(model) {
        static O200K: OnceLock<CoreBPE> = OnceLock::new();
        return O200K.get_or_init(|| {
            o200k_base().unwrap_or_else(|_| cl100k_base().expect("cl100k_base"))
        });
    }

    static CL100K: OnceLock<CoreBPE> = OnceLock::new();
    CL100K.get_or_init(|| cl100k_base().expect("cl100k_base"))
}

fn matches_o200k(model: Option<&str>) -> bool {
    let Some(model) = model else {
        return false;
    };
    let model = model.trim();
    if model.is_empty() {
        return false;
    }
    let model = model.to_ascii_lowercase();
    model.starts_with("o1")
        || model.starts_with("o3")
        || model.starts_with("o4")
        || model.starts_with("gpt-4o")
        || model.starts_with("gpt-4.1")
}
