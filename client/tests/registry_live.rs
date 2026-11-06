//! Talks to a running registry, so it only runs on request:
//! `GRIDLOCK_REGISTRY=http://127.0.0.1:8787 cargo test -p client --test registry_live -- --ignored`
//! (start one with `npx wrangler dev` in `registry/`).

// Real sockets and threads: native only.
#![cfg(not(target_arch = "wasm32"))]

use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use client::net::registry::{registry_url, Announce, Endpoint, ListFetch, Listing};

fn listed_names(url: &str) -> Vec<String> {
    let mut fetch = ListFetch::start(url);
    let listed = loop {
        if let Some(result) = fetch.poll() {
            break result.unwrap();
        }
        thread::sleep(Duration::from_millis(20));
    };
    listed.into_iter().map(|l| l.name).collect()
}

#[test]
#[ignore]
fn listing_appears_waits_for_its_port_and_withdraws() {
    let url = registry_url().expect("set GRIDLOCK_REGISTRY");
    let name = format!("live-test-{}", std::process::id());

    // The port is unknown at first, as while the router is still being asked.
    let port_ready = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let ready = port_ready.clone();
    let listing = Listing::start(
        url.clone(),
        Announce {
            name: name.clone(),
            max_players: 16,
            endpoint: Endpoint::Udp(Box::new(move || {
                ready.load(std::sync::atomic::Ordering::Relaxed).then_some(7878)
            })),
            players: Arc::new(AtomicUsize::new(1)),
        },
    );

    thread::sleep(Duration::from_millis(500));
    assert!(!listed_names(&url).contains(&name), "must not list before the port is known");

    port_ready.store(true, std::sync::atomic::Ordering::Relaxed);
    thread::sleep(Duration::from_secs(1));
    assert!(listed_names(&url).contains(&name));
    let addr = listing.listed_addr().expect("registry echoes the address it saw");
    assert!(addr.ends_with(":7878"), "{addr}");

    drop(listing);
    thread::sleep(Duration::from_secs(1));
    assert!(!listed_names(&url).contains(&name), "dropping withdraws the listing");
}
