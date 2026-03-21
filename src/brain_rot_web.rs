//! Injects a real YouTube embed iframe into the page DOM alongside the Bevy
//! canvas (not overlapping it). Only compiled for wasm32 targets.
#![cfg(target_arch = "wasm32")]

use wasm_bindgen::JsCast;
use web_sys::HtmlElement;

const IFRAME_ID: &str = "yt-brain-rot";
const IFRAME_WIDTH: &str = "360px";

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

    hide_in(&document);

    // Make <body> a flex row so canvas + iframe sit side by side.
    if let Some(body) = document.body() {
        let s = body.style();
        let _ = s.set_property("display", "flex");
        let _ = s.set_property("flex-direction", "row");
        let _ = s.set_property("align-items", "stretch");
        let _ = s.set_property("margin", "0");
        let _ = s.set_property("padding", "0");
        let _ = s.set_property("overflow", "hidden");
        let _ = s.set_property("background", "#000");
    }

    // Shrink the canvas to leave room for the iframe.
    if let Ok(Some(el)) = document.query_selector("canvas") {
        if let Ok(canvas) = el.dyn_into::<HtmlElement>() {
            let s = canvas.style();
            let _ = s.set_property("flex", "1");
            let _ = s.set_property("min-width", "0");
            let _ = s.set_property("max-width", &format!("calc(100vw - {})", IFRAME_WIDTH));
            let _ = s.set_property("height", "100vh");
        }
    }

    // Build and inject the iframe as a flex sibling of the canvas.
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
    let s = html_el.style();
    let _ = s.set_property("width", IFRAME_WIDTH);
    let _ = s.set_property("flex-shrink", "0");
    let _ = s.set_property("height", "100vh");
    let _ = s.set_property("border", "none");
    let _ = s.set_property("background", "#000");

    let Some(body) = document.body() else { return };
    let _ = body.append_child(&html_el);
}

pub fn hide() {
    let Some(window) = web_sys::window() else { return };
    let Some(document) = window.document() else { return };

    hide_in(&document);

    // Restore <body> to its original layout.
    if let Some(body) = document.body() {
        let s = body.style();
        let _ = s.remove_property("display");
        let _ = s.remove_property("flex-direction");
        let _ = s.remove_property("align-items");
        let _ = s.remove_property("margin");
        let _ = s.remove_property("padding");
        let _ = s.remove_property("overflow");
        let _ = s.remove_property("background");
    }

    // Restore the canvas to its original sizing.
    if let Ok(Some(el)) = document.query_selector("canvas") {
        if let Ok(canvas) = el.dyn_into::<HtmlElement>() {
            let s = canvas.style();
            let _ = s.remove_property("flex");
            let _ = s.remove_property("min-width");
            let _ = s.remove_property("max-width");
            let _ = s.remove_property("height");
        }
    }
}

fn hide_in(document: &web_sys::Document) {
    if let Some(el) = document.get_element_by_id(IFRAME_ID) {
        el.remove();
    }
}
