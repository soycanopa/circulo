//! GPUI views for Circulo: sidebar, transcript, composer and right panel.
//!
//! Presentation only. This crate knows the protocol and its own view state, and
//! nothing else: no provider, no CLI, no database, no file system.
//!
//! What exists today is the window shell — the three regions of `docs/ui.md`
//! with the starting graphite tokens — so the app opens and the layout can be
//! built against something real. The sidebar, the virtualized transcript, the
//! permission card and the composer land in Phases 1 and 4 of
//! `docs/implement.md`.

#![forbid(unsafe_code)]

use gpui::{Context, IntoElement, Render, Window, div, prelude::*, px, rgb};

/// Starting graphite tokens from `docs/ui.md`. Exact values will be refined
/// against Waku screenshots; keep them in one place so that stays a one-file
/// change.
const CANVAS: u32 = 0x1a1a1a;
const SURFACE: u32 = 0x141414;
const BORDER: u32 = 0x2a2a2a;
const TEXT: u32 = 0xe2e2e2;
const MUTED: u32 = 0x8a8a8a;
const ACCENT: u32 = 0xe2795b;

/// The root view of the window: app bar, sidebar, transcript, composer.
pub struct Shell;

impl Shell {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Shell {
    fn default() -> Self {
        Self::new()
    }
}

impl Render for Shell {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(CANVAS))
            .text_color(rgb(TEXT))
            .child(app_bar())
            .child(div().flex().flex_1().child(sidebar()).child(transcript()))
            .child(composer())
    }
}

fn app_bar() -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_2()
        .px_4()
        .py_2()
        .border_b_1()
        .border_color(rgb(BORDER))
        .child(div().text_sm().child("Circulo"))
        .child(div().text_sm().text_color(rgb(MUTED)).child("no project"))
}

fn sidebar() -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .w(px(240.0))
        .h_full()
        .p_3()
        .border_r_1()
        .border_color(rgb(BORDER))
        .child(div().text_xs().text_color(rgb(MUTED)).child("PROJECTS"))
        .child(
            div()
                .text_sm()
                .text_color(rgb(MUTED))
                .child("No projects yet."),
        )
}

fn transcript() -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .flex_1()
        .justify_center()
        .items_center()
        .gap_2()
        .child(div().text_xl().child("No session selected"))
        .child(
            div()
                .text_sm()
                .text_color(rgb(MUTED))
                .child("Add a project to start an agent session."),
        )
}

fn composer() -> impl IntoElement {
    div().px_4().pb_4().child(
        div()
            .p_3()
            .rounded_lg()
            .border_1()
            .border_color(rgb(BORDER))
            .bg(rgb(SURFACE))
            .text_sm()
            .text_color(rgb(ACCENT))
            .child("Message the agent…"),
    )
}
