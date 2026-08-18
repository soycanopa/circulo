use circulo_i18n::Catalog;
use gpui::{
    div, px, Context, FontWeight, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled,
};

use crate::shell::AppShell;
use crate::theme::{ACCENT, ACCENT_SURFACE, BORDER, TEXT, TEXT_MUTED};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingPermission {
    pub permission_id: String,
    pub summary: String,
}

pub fn permission_banner(
    pending: &PendingPermission,
    catalog: &Catalog,
    cx: &mut Context<AppShell>,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(10.))
        .w_full()
        .px(px(14.))
        .py(px(12.))
        .bg(ACCENT_SURFACE)
        .border_1()
        .border_color(BORDER)
        .rounded(px(10.))
        .child(
            div()
                .text_size(px(13.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(TEXT)
                .child(catalog.get("permission.request.title").to_string()),
        )
        .child(
            div()
                .text_size(px(12.))
                .text_color(TEXT_MUTED)
                .child(pending.summary.clone()),
        )
        .child(
            div()
                .flex()
                .gap(px(8.))
                .child(
                    div()
                        .id("permission-allow")
                        .px(px(12.))
                        .py(px(6.))
                        .rounded(px(8.))
                        .bg(ACCENT)
                        .text_size(px(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(TEXT)
                        .cursor_pointer()
                        .hover(|style| style.opacity(0.9))
                        .on_click(cx.listener(|this, _, _, cx| this.reply_permission(true, cx)))
                        .child(catalog.get("permission.allow").to_string()),
                )
                .child(
                    div()
                        .id("permission-deny")
                        .px(px(12.))
                        .py(px(6.))
                        .rounded(px(8.))
                        .border_1()
                        .border_color(BORDER)
                        .text_size(px(12.))
                        .text_color(TEXT)
                        .cursor_pointer()
                        .hover(|style| style.bg(BORDER))
                        .on_click(cx.listener(|this, _, _, cx| this.reply_permission(false, cx)))
                        .child(catalog.get("permission.deny").to_string()),
                ),
        )
}
