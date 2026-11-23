//! First run: connect an LG account with a personal access token, or try the demo.

use rune::{Align, El, col, el, icon, row, text};
use rune_kit::{Button, ButtonVariant, Spinner, TextInput};

use crate::app::{App, Phase};
use crate::thinq::Country;
use crate::ui::icons;
use crate::ui::parts::{P, card, heading};

pub const TOKEN_URL: &str = "https://connect-pat.lgthinq.com";

fn step(p: P, n: usize, title: &str, body: Option<El>) -> El {
    row()
        .key(format!("step{n}"))
        .w_full()
        .px(16.0)
        .py(14.0)
        .gap(12.0)
        .items_start()
        .child(el().size(20.0, 20.0).radius(10.0).center().fill(p.k.c.layer_02).child(text(n.to_string()).font(p.k.ty.small_medium).color(p.muted())))
        .child(col().grow(1.0).min_w(0.0).gap(10.0).child(text(title.to_string()).font(p.k.ty.ui).color(p.text()).wrap()).children(body))
}

/// The chosen country, and a search over LG's countries by name or code.
fn country_picker(s: &App, p: P, busy: bool) -> El {
    let code = s.country.text.trim().to_ascii_uppercase();
    let chosen = match Country::parse(&code) {
        Some(_) => format!("{} ({code})", Country::name(&code)),
        None => "None yet".into(),
    };
    let search = TextInput::new(&s.country_query)
        .spellcheck(false)
        .placeholder("Search countries, e.g. Brazil")
        .label("Search countries")
        .leading_icon(p.k.glyph.search)
        .disabled(busy)
        .on_edit(|s: &mut App| &mut s.country_query)
        .on_submit(|s: &mut App, _| {
            if let Some(c) = Country::search(&s.country_query.text).first() {
                s.pick_country(c);
            }
        })
        .build(p.k);
    let mut out = col()
        .key("country")
        .w_full()
        .gap(6.0)
        .child(row().gap(6.0).center_y().child(text("Country").font(p.k.ty.ui).color(p.muted())).child(text(chosen).font(p.k.ty.ui_medium).color(p.text())))
        .child(search);
    let q = s.country_query.text.trim();
    if !q.is_empty() {
        let found = Country::search(q);
        let rows: Vec<El> = found
            .iter()
            .take(6)
            .map(|c| {
                let c = *c;
                row()
                    .key(format!("country-{c}"))
                    .role("button")
                    .label(Country::name(c))
                    .focusable(true)
                    .h(30.0)
                    .px(10.0)
                    .gap(8.0)
                    .center_y()
                    .radius(p.k.radius.sm)
                    .hover(p.k.c.hover)
                    .press(p.k.c.pressed)
                    .on_press(move |s: &mut App| s.pick_country(c))
                    .child(text(Country::name(c)).font(p.k.ty.ui).color(p.text()).grow(1.0))
                    .child(text(c).font(p.k.ty.small).color(p.faint()))
            })
            .collect();
        let list = if rows.is_empty() {
            col().child(text("No LG country matches that.").font(p.k.ty.ui).color(p.muted()).px(10.0).py(6.0))
        } else {
            col().children(rows)
        };
        out = out.child(list.key("matches").w_full().padding(4.0).radius(p.k.radius.md).border(p.hair.max(0.5), p.k.c.border_base).fill(p.k.c.bg_base));
    }
    out
}

pub fn view(s: &App, p: P) -> El {
    let k = p.k;
    let busy = s.phase == Phase::SigningIn || s.phase == Phase::Starting;

    let logo = match s.logo.as_ref() {
        Some(img) => el().key("logo").size(72.0, 72.0).radius(36.0).clip_soft(0.0).pixels(img.pixels()).shadow(6.0, 24.0, rune::Color::BLACK.alpha(0.18)),
        None => el().key("logo").size(72.0, 72.0),
    };
    let hero = col()
        .w_full()
        .items_center()
        .gap(6.0)
        .child(logo)
        .child(text("Frostfire").font(heading(24.0, 650.0)).color(p.text()).mt(6.0))
        .child(text("Your LG air conditioners, on your Mac.").font(p.k.ty.body).color(p.muted()).align(Align::Center));

    let token = TextInput::new(&s.token)
        .secure()
        .spellcheck(false)
        .placeholder("Personal access token")
        .label("ThinQ personal access token")
        .leading_icon(&icons::KEY)
        .disabled(busy)
        .on_edit(|s: &mut App| &mut s.token)
        .on_submit(|s: &mut App, _| s.sign_in())
        .build(k);
    let country = country_picker(s, p, busy);

    let open = Button::new("Open connect-pat.lgthinq.com")
        .variant(ButtonVariant::Neutral)
        .on_press_tx(|s: &mut App, tx| {
            if tx.open_url(TOKEN_URL).is_err() {
                s.say("Couldn't open the browser. Visit connect-pat.lgthinq.com.", true);
            }
        })
        .build(k)
        .self_start();
    let steps = card(
        p,
        vec![
            step(p, 1, "Sign in to LG's token page with your ThinQ account.", Some(open)),
            step(
                p,
                2,
                "Add a new token and tick: Device List Retrieval, Device Status Inquiry, Device Control and Device Energy Consumption Inquiry. Frostfire doesn't need the two subscription permissions.",
                None,
            ),
            step(p, 3, "Paste it here, and pick the country your LG account is in.", Some(col().w_full().gap(8.0).child(token).child(country))),
        ],
    );

    let actions = if busy {
        row().h(32.0).gap(8.0).center().child(Spinner::new().build(k)).child(text("Talking to LG…").font(p.k.ty.ui).color(p.muted()))
    } else {
        row()
            .gap(8.0)
            .center()
            .child(Button::new("Try the demo").variant(ButtonVariant::Ghost).on_press(|s: &mut App| s.try_demo()).build(k))
            .child(Button::new("Connect").variant(ButtonVariant::Contrast).on_press(|s: &mut App| s.sign_in()).build(k))
    };

    let mut body = col().key("welcome").w(440.0).gap(18.0).child(hero).child(steps);
    if let Some(err) = &s.sign_in_error {
        body = body.child(
            row()
                .w_full()
                .padding(12.0)
                .radius(p.k.radius.md)
                .fill(p.k.c.danger.bg)
                .border(p.hair, p.k.c.danger.border)
                .child(text(err.clone()).font(p.k.ty.ui).color(p.k.c.danger.fg).wrap().grow(1.0)),
        );
    }
    body.child(row().w_full().justify_end().child(actions)).child(
        row().w_full().gap(6.0).items_start().child(icon(&icons::LOCK, 12.0).color(p.faint()).mt(2.0)).child(
            text("The token goes straight into your Keychain. Frostfire talks only to LG's official ThinQ Connect servers.")
                .font(p.k.ty.small)
                .color(p.faint())
                .wrap()
                .grow(1.0),
        ),
    )
}
