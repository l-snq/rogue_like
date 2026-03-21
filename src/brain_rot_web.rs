//! Injects a real YouTube embed iframe into the page DOM, positioned fixed
//! on the right side of the screen over the Bevy canvas.
//! Only compiled for wasm32 targets — the module is empty on native.
#![cfg(target_arch = "wasm32")]

use wasm_bindgen::JsCast;
use web_sys::HtmlElement;

const IFRAME_ID: &str = "yt-brain-rot";

/// YouTube video IDs to cycle through.
///https://youtu.be/
/// These are embedded via youtube.com/embed/<ID> — swap in any Short's ID.
pub const VIDEO_IDS: &[&str] = &[
    "Hpye8Muh84o",
    "363mhSB6Uac",
    "KJOc6WBh5FM",
    "Z2eAznubE3s",
    "Kuw0zT6pDNA",
    "XtVoGu626WE",
];

pub fn show(video_idx: usize) {
    let Some(window) = web_sys::window() else { return };
    let Some(document) = window.document() else { return };

    // Remove any previous iframe first.
    hide_in(&document);

    let video_id = VIDEO_IDS[video_idx % VIDEO_IDS.len()];
    let src = format!(
        "https://www.youtube.com/embed/{}?autoplay=1&rel=0&loop=1&playlist={}",
        video_id, video_id
    );

    let Ok(el) = document.create_element("iframe") else { return };
    let _ = el.set_attribute("id", IFRAME_ID);
    let _ = el.set_attribute("src", &src);
    let _ = el.set_attribute("allow", "autoplay; encrypted-media; fullscreen");
    let _ = el.set_attribute("allowfullscreen", "");

    let Ok(html_el) = el.dyn_into::<HtmlElement>() else { return };
    let style = html_el.style();
    let _ = style.set_property("position", "fixed");
    let _ = style.set_property("right", "0");
    let _ = style.set_property("top", "0");
    let _ = style.set_property("width", "360px");
    let _ = style.set_property("height", "100vh");
    let _ = style.set_property("border", "none");
    let _ = style.set_property("z-index", "9999");
    let _ = style.set_property("background", "#000");

    let Some(body) = document.body() else { return };
    let _ = body.append_child(&html_el);
}

pub fn hide() {
    let Some(window) = web_sys::window() else { return };
    let Some(document) = window.document() else { return };
    hide_in(&document);
}

fn hide_in(document: &web_sys::Document) {
    if let Some(el) = document.get_element_by_id(IFRAME_ID) {
        el.remove();
    }
}
