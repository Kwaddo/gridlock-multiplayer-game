use std::sync::atomic::Ordering;

use platform::{Duration, Instant};
use transport::web::HttpRequest;

use super::{create_body, heartbeat_body, Announce, Created, Listed, HEARTBEAT_SECS, RETRY_SECS};

/// The public list, fetched with the browser's fetch().
pub struct ListFetch {
    request: HttpRequest,
    done: bool,
}

impl ListFetch {
    pub fn start(base: &str) -> Self {
        ListFetch { request: HttpRequest::start("GET", &format!("{base}/servers"), &[], None), done: false }
    }

    /// `None` until the request finishes, then the result once.
    pub fn poll(&mut self) -> Option<Result<Vec<Listed>, String>> {
        if self.done {
            return None;
        }
        let (status, body) = self.request.poll()?;
        self.done = true;
        Some(match status {
            200 => serde_json::from_slice(&body).map_err(|e| e.to_string()),
            0 => Err("registry unreachable".into()),
            s => Err(format!("registry answered {s}")),
        })
    }
}

enum State {
    /// Not listed; announce at `at` (immediately at first).
    Idle { at: Instant },
    Creating(HttpRequest),
    Listed { id: String, token: String, next_beat: Instant },
    Beating { id: String, token: String, request: HttpRequest },
}

/// Keeps a hosted game on the public list until dropped. The browser has no
/// threads, so the host calls [`Listing::tick`] every frame.
pub struct Listing {
    base: String,
    announce: Announce,
    state: State,
    listed_addr: Option<String>,
}

impl Listing {
    pub fn start(base: String, announce: Announce) -> Self {
        Listing { base, announce, state: State::Idle { at: Instant::now() }, listed_addr: None }
    }

    pub fn tick(&mut self) {
        let players = self.announce.players.load(Ordering::Relaxed);
        let now = Instant::now();
        let retry = State::Idle { at: now + Duration::from_secs(RETRY_SECS) };
        let beat_at = now + Duration::from_secs(HEARTBEAT_SECS);

        self.state = match std::mem::replace(&mut self.state, State::Idle { at: now }) {
            State::Idle { at } if now >= at => match create_body(&self.announce, players) {
                Some(body) => State::Creating(HttpRequest::start(
                    "POST",
                    &format!("{}/servers", self.base),
                    body.to_string().as_bytes(),
                    None,
                )),
                None => State::Idle { at },
            },
            State::Creating(mut request) => match request.poll() {
                None => State::Creating(request),
                Some((201, body)) => match serde_json::from_slice::<Created>(&body) {
                    Ok(Created { id, token, addr }) => {
                        self.listed_addr = addr;
                        State::Listed { id, token, next_beat: beat_at }
                    }
                    Err(_) => retry,
                },
                Some(_) => retry,
            },
            State::Listed { id, token, next_beat } if now >= next_beat => {
                let request = HttpRequest::start(
                    "PUT",
                    &format!("{}/servers/{id}", self.base),
                    heartbeat_body(players).to_string().as_bytes(),
                    Some(&format!("Bearer {token}")),
                );
                State::Beating { id, token, request }
            }
            State::Beating { id, token, mut request } => match request.poll() {
                None => State::Beating { id, token, request },
                Some((204, _)) => State::Listed { id, token, next_beat: beat_at },
                // Listing expired or the registry restarted: register again.
                Some(_) => State::Idle { at: now },
            },
            other => other,
        };
    }

    /// The address the registry advertises for us, once listed.
    pub fn listed_addr(&self) -> Option<String> {
        self.listed_addr.clone()
    }
}

impl Drop for Listing {
    fn drop(&mut self) {
        // fetch() keeps going after we stop tracking it, so this still arrives.
        if let State::Listed { id, token, .. } | State::Beating { id, token, .. } = &self.state {
            let _ = HttpRequest::start(
                "DELETE",
                &format!("{}/servers/{id}", self.base),
                &[],
                Some(&format!("Bearer {token}")),
            );
        }
    }
}
