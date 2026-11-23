//! Frostfire: LG ThinQ air conditioners on your Mac, in the menu bar and a GPU-drawn window.
//!
//! `cargo run` opens it. Headless, for screenshots and checks:
//! `cargo run -- --snap shot.png --scene cool` (scenes: welcome, country, cool, heat, auto, dry, off, settings).

#![deny(unsafe_op_in_unsafe_fn)]

mod app;
mod demo;
mod gfx;
mod status_item;
mod store;
mod thinq;
mod ui;
mod worker;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use rune::AppSpec;

use app::{App, Phase};
use thinq::model::{Command, Timer};
use worker::{Job, Worker};

/// Runs that render to files instead of a window: no worker, no network, no Keychain.
fn headless() -> bool {
    std::env::args().any(|a| matches!(a.as_str(), "--snap" | "--tree" | "--script" | "--frames" | "--video" | "--list-scenes" | "--trace"))
}

fn blank() -> App {
    App::new(Worker::detached(), None)
}

/// The demo's living-room unit, ready, for the scenes below.
fn demo_ready(s: &mut App) {
    s.phase = Phase::Ready;
    s.demo = true;
    s.devices = demo::devices();
    s.device = Some(demo::LIVING.into());
    s.profile = Some(demo::profile(demo::LIVING));
    s.state = Some(demo::initial_state(demo::LIVING));
    s.energy = demo::energy_week();
    s.updated_at = Some(0.0);
}

fn with(s: &mut App, cmd: Command) {
    if let (Some(p), Some(st)) = (s.profile.as_ref(), s.state.as_mut())
        && let Ok(payload) = cmd.payload(p, st)
    {
        st.apply(&cmd.optimistic(&payload));
    }
}

type Scene = (&'static str, fn(&mut App));

const SCENES: &[Scene] = &[
    ("welcome", |s| s.phase = Phase::Welcome),
    ("country", |s| {
        s.phase = Phase::Welcome;
        s.country_query = rune::TextEdit::new("bra");
    }),
    ("cool", demo_ready),
    ("heat", |s| {
        demo_ready(s);
        with(s, Command::Mode("HEAT".into()));
        with(s, Command::Target(25.5, thinq::model::Unit::C));
        with(s, Command::TimerCancel(Timer::Stop));
        with(s, Command::TimerIn(Timer::Sleep, 240));
    }),
    ("auto", |s| {
        demo_ready(s);
        with(s, Command::Mode("AUTO".into()));
        with(s, Command::Wind("AUTO".into()));
    }),
    ("dry", |s| {
        demo_ready(s);
        with(s, Command::Mode("AIR_DRY".into()));
    }),
    ("off", |s| {
        demo_ready(s);
        with(s, Command::Power(false));
    }),
    ("settings", |s| {
        demo_ready(s);
        s.settings = true;
    }),
];

/// `--dump-profiles`: prints what each air conditioner on the signed-in account reports (its
/// profile and state), for working out what a model supports. Never prints the token.
fn dump_profiles() {
    let config = store::load_config();
    let args: Vec<String> = std::env::args().collect();
    let flag = args.iter().position(|a| a == "--country").and_then(|i| args.get(i + 1)).cloned();
    let (Some(token), Some(country)) = (store::load_token(), flag.or(config.country.clone())) else {
        eprintln!("Not signed in: connect in the app first.");
        std::process::exit(1);
    };
    let Some(country) = thinq::Country::parse(&country) else {
        eprintln!("No valid country saved.");
        std::process::exit(1);
    };
    let client = match thinq::client::Client::new(token, country, config.client_id.clone().unwrap_or_default()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", e.friendly());
            std::process::exit(1);
        }
    };
    let devices = match client.devices() {
        Ok(d) => thinq::model::parse_devices(&d),
        Err(e) => {
            eprintln!("{}", e.friendly());
            std::process::exit(1);
        }
    };
    for d in devices.iter().filter(|d| d.kind == "DEVICE_AIR_CONDITIONER") {
        let short: String = d.id.chars().take(6).collect();
        println!("=== {} ({}) {short}…", d.alias, d.model);
        for (what, v) in [("profile", client.profile(&d.id)), ("state", client.state(&d.id))] {
            match v {
                Ok(v) => println!("--- {what}\n{}", serde_json::to_string_pretty(&v).unwrap_or_default()),
                Err(e) => println!("--- {what}: {}", e.friendly()),
            }
        }
    }
}

fn main() {
    if std::env::args().any(|a| a == "--dump-profiles") {
        dump_profiles();
        return;
    }
    rune::prewarm_gpu();
    let headless = headless();
    let config = if headless { store::Config::default() } else { store::load_config() };

    let spec = AppSpec::<App> {
        scenes: SCENES,
        min_size: Some((720.0, 480.0)),
        fonts: rune_theme_opencode::FONTS,
        families: rune_theme_opencode::FAMILIES,
        setup: Some(|s, waker| {
            let cache = s.images.insert(rune::ImageCache::new(waker));
            s.logo = Some(cache.load(&include_bytes!("../assets/logo-256.png")[..]));
        }),
        poll: Some(|s, t, tx| s.poll(t, tx)),
        on_key: Some(ui::on_key),
        ..AppSpec::new("Frostfire", (880.0, 660.0), blank, ui::view)
    };
    let app = rune::App::new(spec);
    let handle = app.handle();
    let live = Arc::new(AtomicBool::new(false));
    let fx_live = live.clone();
    if !headless {
        status_item::install(handle.clone());
    }

    let mut app = app
        .init(move || {
            if headless {
                let mut s = blank();
                s.fx_live = fx_live;
                return s;
            }
            let mut s = App::new(Worker::spawn(handle.clone()), Some(handle));
            s.fx_live = fx_live;
            s.worker.send(Job::Restore);
            s
        })
        // Closing the window keeps Frostfire in the menu bar (or the Dock); Quit ends it.
        .keep_running()
        .compositor(gfx::Gfx::new(live));
    if !config.dock_icon && config.menu_bar {
        app = app.accessory();
    }
    app.run();
}
