//! Boots the real service layer offline and proves it still comes up.
//!
//! Sixteen recent commits reorganized service registration, process-global
//! `OnceLock` initialization, database opening, signing and swap-router
//! registration. Nothing booted the whole service layer in a test before this
//! file, so a startup hang or an accessor read before its writer only showed up
//! when the owner ran the app.
//!
//! # Process-global state
//!
//! This binary constructs a real `ServiceManager`, registers the full service
//! list by calling `crate::run::services::register_all_services` itself — the
//! same function production boots through, so a service added there is covered
//! here automatically — which also calls `set_router_factory` /
//! `set_runtime_factory` —
//! both one-shot `OnceLock`-backed globals. Every test in this file therefore
//! shares process-global state (the swap router registry, the wallet-watch
//! runtime factory, the readiness flags in `screenerbot::global`) with every
//! OTHER test in this file and must not assume isolation from them. In
//! practice there is exactly one boot test below for that reason.
//!
//! No network, no RPC endpoint, no wallet key, no running bot: `isolated_env`
//! points every path at a fresh temp directory and config stays at defaults.

mod common;

use std::time::Duration;

use screenerbot::services::ServiceManager;

/// The one boot: an isolated env, no config.toml, no network — the same shape
/// Explore Mode/pre-init boot in `src/run/mod.rs` uses before the wallet/RPC
/// setup path runs. A hang here must fail the test, never hang the run.
#[tokio::test]
async fn the_service_layer_boots_offline_inside_a_startup_budget() {
    let _dir = common::isolated_env();

    let mut service_manager = ServiceManager::new()
        .await
        .expect("ServiceManager::new must not fail offline");
    screenerbot::run::services::register_all_services(&mut service_manager);

    let registered_names = service_manager.get_all_service_names();
    assert!(
        !registered_names.is_empty(),
        "registration produced no services at all"
    );

    tokio::time::timeout(Duration::from_secs(60), service_manager.start_all())
        .await
        .expect("service layer boot hung past the 60s startup budget")
        .expect("service layer boot returned an error");

    // (b) Every registered service reaches Ready (has running handles), or the
    // code's own `is_enabled()` declares it out of scope for this boot — the
    // real gate `start_all` uses to decide what to start at all. Offline, with
    // no config.toml and no wallet, that is most of the list (RPC/wallet-bound
    // services stay disabled rather than degraded — there is no network to be
    // degraded against).
    let running = service_manager.get_running_service_names();
    for name in &registered_names {
        let is_running = running.contains(name);
        let is_enabled = service_manager.is_service_enabled(name);
        assert!(
            is_running || !is_enabled,
            "service '{name}' is enabled but never reached Ready, and boot returned Ok — a \
             silent no-op start"
        );
    }
    assert!(
        !running.is_empty(),
        "not one service reached Ready — offline boot should still start the webserver"
    );

    // (c) Process-global accessors resolve without panicking after boot.
    screenerbot::swaps::registry::get_registry()
        .expect("router registry must resolve once set_router_factory has run");
    assert!(
        screenerbot::swaps::registry::try_get_registry().is_some(),
        "try_get_registry must be Some once the registry has been built"
    );
    // Answers `false` here (no wallet database was opened offline) — this is
    // exactly the "reported absent, not fatal" contract `dashboard_launch_readiness`
    // pins for the same accessor; the point of this assertion is that reading it
    // does not panic before any writer has run.
    let _ = screenerbot::wallet::is_wallet_database_ready();
    let _ = screenerbot::pools::service::is_pool_service_running();

    // (d) A OnceLock read before its writer ran: `set_router_factory` is a
    // one-shot OnceLock set unconditionally at the top of
    // `src/run/services.rs::register_all_services`, which this binary calls
    // before any assertion can run. There is no reachable point inside a binary that also
    // boots the service layer where the router factory OnceLock is still
    // unset — so this ordering cannot be exercised here.
}
