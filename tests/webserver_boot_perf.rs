//! Boot performance of the webserver and the dashboard's first paint.
//!
//! Scope is deliberately narrow: the path between "the window appeared" and "the home
//! dashboard is showing real numbers instead of a skeleton". Nothing here touches the
//! network, the wallet or an RPC endpoint.
//!
//! The measurements are wall-clock and therefore machine-dependent, so every ceiling is
//! set orders of magnitude above what the operation should cost. They exist to catch a
//! change of KIND — a synchronous build, a blocking wait, a per-request re-render — not to
//! police microseconds.

use std::sync::Arc;
use std::time::{Duration, Instant};

use screenerbot::filtering;
use screenerbot::webserver::routes;
use screenerbot::webserver::state::AppState;
use screenerbot::webserver::templates;

/// A first-paint fetch may not wait for work measured in seconds. The frontend's own
/// request timeout is 10s and the dashboard's poll is 5s, so anything approaching a second
/// on this path is already a defect.
const FIRST_PAINT_BUDGET: Duration = Duration::from_millis(500);

/// The dashboard's ONE landing fetch must never wait for the first filtering snapshot.
///
/// This is the regression test for a launch that sat in its loading skeleton for thirty
/// seconds. `/api/dashboard/home` reads filtering counts, those counts came from
/// `fetch_stats`, and `fetch_stats` on a cold store BUILDS the snapshot — every token in
/// the database through the whole pipeline — behind a 30-second timeout. The one fetch
/// that clears the skeleton was therefore gated on the single most expensive operation in
/// the process, on every launch, for a handful of numbers.
///
/// The store here is cold (no service has ever refreshed it), which is exactly the state a
/// freshly-launched process is in. `try_fetch_stats` must answer from what exists and
/// return, leaving the build to the background.
#[tokio::test]
async fn filtering_counts_never_block_the_first_paint() {
    let started = Instant::now();
    let stats = filtering::try_fetch_stats().await;
    let elapsed = started.elapsed();

    assert!(
        elapsed < FIRST_PAINT_BUDGET,
        "reading filtering counts on a cold store took {elapsed:?}, over the {FIRST_PAINT_BUDGET:?} \
         first-paint budget — it is waiting for the snapshot to be built instead of \
         reporting that there isn't one yet"
    );

    // Absent, not zero and not an error: no snapshot has been built in this process. The
    // route renders the counts it has and the next poll picks up the real ones.
    assert!(
        stats.is_none(),
        "a cold store reported statistics; either a snapshot leaked in from another test \
         or this call built one synchronously"
    );

    // Asking again must stay just as cheap — a refresh is now in flight, and the second
    // caller must not queue behind it either.
    let started = Instant::now();
    let _ = filtering::try_fetch_stats().await;
    let elapsed = started.elapsed();

    assert!(
        elapsed < FIRST_PAINT_BUDGET,
        "the second cold read took {elapsed:?} — it is waiting on the refresh the first \
         one started"
    );
}

/// The HTML the browser blocks on is assembled per request from embedded templates, so its
/// cost is paid on every page load and before anything can paint.
///
/// Two separate things are guarded here, and the SIZE is the one that matters. Rendering is
/// string concatenation over `include_str!` constants, so it is cheap by construction; what
/// the browser actually waits for is the megabyte-and-a-half of inlined stylesheet that
/// comes with it. Every page's CSS is deliberately in that bundle (the per-page map is
/// injected lazily, after first paint, so anything hidden by default would otherwise flash)
/// — which makes the bundle easy to grow without noticing.
#[tokio::test]
async fn the_dashboard_shell_stays_cheap_to_render_and_send() {
    // Warm any lazily-initialised template state so the measurement is of the steady-state
    // render rather than one-time setup.
    let _ = templates::base_template("ScreenerBot", "home", &templates::home_content());

    const RENDERS: u32 = 20;
    let started = Instant::now();
    let mut bytes = 0usize;
    for _ in 0..RENDERS {
        let html = templates::base_template("ScreenerBot", "home", &templates::home_content());
        bytes = html.len();
    }
    let per_render = started.elapsed() / RENDERS;

    println!(
        "home shell: {per_render:?} per render, {:.2} MB",
        bytes as f64 / (1024.0 * 1024.0)
    );

    // Generous because tests build unoptimised, and the point is to catch a change of KIND:
    // a template that starts reading from disk, querying, or recompiling something per
    // request would land orders of magnitude above this.
    assert!(
        per_render < Duration::from_millis(120),
        "rendering the home shell took {per_render:?} per render — that is no longer plain \
         string assembly over embedded constants"
    );

    // ~1.7 MB today, almost entirely the inlined stylesheet bundle. The browser parses all
    // of it before first paint, so this is a real boot cost and it must not creep.
    const MAX_SHELL_BYTES: usize = 2_400_000;
    assert!(
        bytes < MAX_SHELL_BYTES,
        "the initial HTML is {bytes} bytes, over the {MAX_SHELL_BYTES} ceiling — the inlined \
         style bundle has grown and every page load now pays for it before painting"
    );
}

/// The first-paint HTML must arrive already carrying its skeleton.
///
/// This is a boot-perception invariant, not decoration: the cards' `loading` class is baked
/// into the template so the page has its final geometry before any data lands, and the
/// stylesheet is inlined into the initial HTML so nothing flashes unstyled while the
/// router lazily injects the per-page styles. A JS-injected skeleton would still flash
/// empty for a frame.
#[tokio::test]
async fn the_first_paint_html_carries_its_own_skeleton_and_styles() {
    let html = templates::base_template("ScreenerBot", "home", &templates::home_content());

    // Assert on the shell's ROOT container and its skeleton flag, not on an inner card
    // class: the home page has been redesigned before, and a card name that no longer
    // exists fails the test while the invariant it guards is still intact.
    assert!(
        html.contains("home-dashboard"),
        "the home shell no longer ships its layout in the initial HTML"
    );
    assert!(
        html.contains("loading"),
        "the home shell no longer carries the loading class at first paint, so the skeleton \
         is being applied by script after the page is already visible"
    );
    assert!(
        html.contains("<style"),
        "the initial HTML no longer inlines a stylesheet — the page will paint unstyled \
         until the router injects the per-page styles"
    );
}

/// Route construction happens once during boot, before the port is bound and therefore
/// before the Electron shell can load anything. With 200+ endpoints it must still be a
/// matter of assembling a table, not of touching the filesystem or a database.
#[tokio::test]
async fn building_the_router_is_not_a_boot_cost() {
    let state = Arc::new(AppState::new());

    // First build separately: it is the one that would pay for any one-time initialisation
    // hiding inside a route module, and it is the one that actually runs at boot.
    let started = Instant::now();
    let _router = routes::create_router(state.clone());
    let first = started.elapsed();

    const BUILDS: u32 = 5;
    let started = Instant::now();
    for _ in 0..BUILDS {
        let _router = routes::create_router(state.clone());
    }
    let per_build = started.elapsed() / BUILDS;

    println!("router: first build {first:?}, steady state {per_build:?} per build");

    assert!(
        first < Duration::from_millis(500),
        "the first router build took {first:?} — something in a route module is doing real \
         work at construction time, and it is delaying the port bind"
    );
    assert!(
        per_build < Duration::from_millis(250),
        "building the router costs {per_build:?} — route registration is no longer just \
         table assembly"
    );
}
