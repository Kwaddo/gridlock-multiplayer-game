use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use ureq::config::IpFamily;

use super::{create_body, heartbeat_body, Announce, Created, Listed, HEARTBEAT_SECS, RETRY_SECS};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

fn agent(family: IpFamily) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(REQUEST_TIMEOUT))
        .ip_family(family)
        .build()
        .into()
}

/// The public list, fetched on a background thread.
pub struct ListFetch {
    rx: Receiver<Result<Vec<Listed>, String>>,
    done: bool,
}

impl ListFetch {
    pub fn start(base: &str) -> Self {
        let url = format!("{base}/servers");
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let result = agent(IpFamily::Any)
                .get(&url)
                .call()
                .and_then(|mut resp| resp.body_mut().read_to_string())
                .map_err(|e| e.to_string())
                .and_then(|body| serde_json::from_str(&body).map_err(|e| e.to_string()));
            let _ = tx.send(result);
        });
        ListFetch { rx, done: false }
    }

    /// `None` until the request finishes, then the result once.
    pub fn poll(&mut self) -> Option<Result<Vec<Listed>, String>> {
        if self.done {
            return None;
        }
        match self.rx.try_recv() {
            Ok(result) => {
                self.done = true;
                Some(result)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.done = true;
                Some(Err("lookup thread stopped".into()))
            }
        }
    }
}

/// Keeps a hosted game on the public list until dropped.
///
/// Dropping only signals the heartbeat thread, which withdraws the listing on
/// its own, so leaving a game never waits on the network. If the process exits
/// first, the listing simply expires.
pub struct Listing {
    stop: Arc<AtomicBool>,
    listed_addr: Arc<Mutex<Option<String>>>,
}

impl Listing {
    pub fn start(base: String, announce: Announce) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let listed_addr = Arc::new(Mutex::new(None));
        let (flag, addr) = (stop.clone(), listed_addr.clone());
        thread::spawn(move || heartbeat_loop(&base, &announce, &flag, &addr));
        Listing { stop, listed_addr }
    }

    /// The heartbeat runs on its own thread here; nothing to do per frame.
    pub fn tick(&mut self) {}

    /// The address the registry advertises for us, once listed. It is the IP
    /// our requests came from, which is what the internet sees.
    pub fn listed_addr(&self) -> Option<String> {
        self.listed_addr.lock().ok()?.clone()
    }
}

impl Drop for Listing {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

fn heartbeat_loop(base: &str, announce: &Announce, stop: &AtomicBool, listed_addr: &Mutex<Option<String>>) {
    // The registry lists the address our requests come from, and game hosts
    // only serve IPv4, so announce over IPv4.
    let agent = agent(IpFamily::Ipv4Only);
    let players = || announce.players.load(Ordering::Relaxed);

    let mut listed: Option<Created> = None;
    while !stop.load(Ordering::Relaxed) {
        listed = match listed.take() {
            None => {
                // Wait for the endpoint (e.g. the router's port) before announcing.
                let Some(body) = create_body(announce, players()) else {
                    thread::sleep(Duration::from_millis(100));
                    continue;
                };
                let created = agent
                    .post(&format!("{base}/servers"))
                    .send_json(body)
                    .and_then(|mut resp| resp.body_mut().read_json::<Created>())
                    .ok();
                if let (Some(c), Ok(mut addr)) = (&created, listed_addr.lock()) {
                    *addr = c.addr.clone();
                }
                created
            }
            Some(created) => {
                let alive = agent
                    .put(&format!("{base}/servers/{}", created.id))
                    .header("Authorization", &format!("Bearer {}", created.token))
                    .send_json(heartbeat_body(players()))
                    .is_ok();
                // A failed heartbeat (registry restarted, listing expired) re-registers.
                alive.then_some(created)
            }
        };
        let wait = if listed.is_some() { HEARTBEAT_SECS } else { RETRY_SECS };
        sleep_unless_stopped(Duration::from_secs(wait), stop);
    }

    if let Some(created) = listed {
        let _ = agent
            .delete(&format!("{base}/servers/{}", created.id))
            .header("Authorization", &format!("Bearer {}", created.token))
            .call();
    }
}

fn sleep_unless_stopped(total: Duration, stop: &AtomicBool) {
    let step = Duration::from_millis(100);
    let mut waited = Duration::ZERO;
    while waited < total && !stop.load(Ordering::Relaxed) {
        thread::sleep(step);
        waited += step;
    }
}
