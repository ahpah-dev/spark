//! A bright, friendly look with big buttons and big text.

use egui::{Color32, CornerRadius, FontId, Stroke, TextStyle, Visuals, vec2};

pub const PURPLE: Color32 = Color32::from_rgb(110, 76, 230);
pub const PURPLE_DARK: Color32 = Color32::from_rgb(70, 46, 160);
pub const GREEN: Color32 = Color32::from_rgb(40, 178, 92);
pub const GREEN_DARK: Color32 = Color32::from_rgb(22, 130, 66);
pub const PANEL: Color32 = Color32::from_rgb(243, 240, 255);
pub const PANEL_DARK: Color32 = Color32::from_rgb(228, 222, 250);
pub const INK: Color32 = Color32::from_rgb(40, 34, 70);

/// The Spark icon, for the window's title bar and the task bar.
pub fn window_icon() -> std::sync::Arc<egui::IconData> {
    let img = image::load_from_memory(include_bytes!("../assets/icon.png")).map(|i| i.to_rgba8()).unwrap_or_else(|_| image::RgbaImage::new(1, 1));
    std::sync::Arc::new(egui::IconData { width: img.width(), height: img.height(), rgba: img.into_raw() })
}

pub fn apply(ctx: &egui::Context) {
    let mut v = Visuals::light();
    v.panel_fill = PANEL;
    v.window_fill = Color32::from_rgb(252, 250, 255);
    v.extreme_bg_color = Color32::WHITE;
    v.faint_bg_color = Color32::from_rgb(236, 232, 252);
    v.override_text_color = Some(INK);
    v.selection.bg_fill = PURPLE;
    v.selection.stroke = Stroke::new(1.0, Color32::WHITE);
    v.window_corner_radius = CornerRadius::same(14);
    v.menu_corner_radius = CornerRadius::same(10);
    v.window_stroke = Stroke::new(1.5, Color32::from_rgb(190, 180, 235));

    let radius = CornerRadius::same(9);
    v.widgets.noninteractive.corner_radius = radius;
    v.widgets.inactive.corner_radius = radius;
    v.widgets.hovered.corner_radius = radius;
    v.widgets.active.corner_radius = radius;
    v.widgets.open.corner_radius = radius;

    v.widgets.inactive.weak_bg_fill = Color32::from_rgb(255, 255, 255);
    v.widgets.inactive.bg_fill = Color32::from_rgb(255, 255, 255);
    v.widgets.inactive.bg_stroke = Stroke::new(1.5, Color32::from_rgb(200, 192, 240));
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(232, 224, 255);
    v.widgets.hovered.bg_fill = Color32::from_rgb(232, 224, 255);
    v.widgets.hovered.bg_stroke = Stroke::new(2.0, PURPLE);
    v.widgets.active.weak_bg_fill = Color32::from_rgb(210, 198, 255);
    v.widgets.active.bg_fill = Color32::from_rgb(210, 198, 255);
    v.widgets.active.bg_stroke = Stroke::new(2.0, PURPLE_DARK);
    ctx.set_visuals(v);

    ctx.global_style_mut(|style| {
        style.text_styles.insert(TextStyle::Heading, FontId::proportional(24.0));
        style.text_styles.insert(TextStyle::Body, FontId::proportional(16.0));
        style.text_styles.insert(TextStyle::Button, FontId::proportional(16.0));
        style.text_styles.insert(TextStyle::Small, FontId::proportional(13.0));
        style.text_styles.insert(TextStyle::Monospace, FontId::monospace(15.0));
        style.spacing.item_spacing = vec2(8.0, 8.0);
        style.spacing.button_padding = vec2(12.0, 7.0);
        style.spacing.interact_size.y = 30.0;
        style.spacing.slider_width = 120.0;
    });
}
