use crate::dock::{Dock, PanelButtons};
use gpui::{Context, Entity, IntoElement, ParentElement, Render, Styled, Window, px};
use ui::prelude::*;

pub struct ActivityBar {
    left: Entity<PanelButtons>,
    bottom: Entity<PanelButtons>,
    right: Entity<PanelButtons>,
}

impl ActivityBar {
    pub fn new(
        left: Entity<PanelButtons>,
        bottom: Entity<PanelButtons>,
        right: Entity<PanelButtons>,
    ) -> Self {
        Self {
            left,
            bottom,
            right,
        }
    }
}

fn dock_has_panels(panel_buttons: &Entity<PanelButtons>, cx: &mut Context<ActivityBar>) -> bool {
    let dock_entity: Entity<Dock> = panel_buttons.read(cx).dock_entity();
    !dock_entity.read(cx).panel_entries_is_empty()
}

impl Render for ActivityBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let has_left = dock_has_panels(&self.left, cx);
        let has_bottom = dock_has_panels(&self.bottom, cx);
        let has_right = dock_has_panels(&self.right, cx);

        v_flex()
            .w(px(44.))
            .h_full()
            .flex_shrink_0()
            .py_2()
            .gap_2()
            .bg(cx.theme().colors().status_bar_background)
            .border_r_1()
            .border_color(cx.theme().colors().border_variant)
            .when(has_left, |this| this.child(self.left.clone()))
            .when(has_bottom, |this| this.child(self.bottom.clone()))
            .when(has_right, |this| this.child(self.right.clone()))
    }
}
