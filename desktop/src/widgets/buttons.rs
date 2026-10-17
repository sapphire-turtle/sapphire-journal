//! Icon buttons, as the journal screen's toolbars use them.

use sapphire_framework::gui::egui;

/// Square icon-only button (16×16) with a tooltip.
pub fn icon_btn(
    ui: &mut egui::Ui,
    src: egui::ImageSource<'static>,
    tooltip: &str,
) -> egui::Response {
    let tint = ui.visuals().text_color();
    let img = egui::Image::new(src)
        .fit_to_exact_size(egui::vec2(16.0, 16.0))
        .tint(tint);
    ui.add(egui::Button::image(img)).on_hover_text(tooltip)
}

/// Toggleable icon button — renders with a pressed/highlighted background
/// when `active` is true.
pub fn icon_toggle_btn(
    ui: &mut egui::Ui,
    src: egui::ImageSource<'static>,
    active: bool,
    tooltip: &str,
) -> egui::Response {
    let tint = ui.visuals().text_color();
    let img = egui::Image::new(src)
        .fit_to_exact_size(egui::vec2(16.0, 16.0))
        .tint(tint);
    ui.add(egui::Button::image(img).selected(active))
        .on_hover_text(tooltip)
}

/// Icon + text button (14×14 icon to align with the label height).
pub fn icon_text_btn(
    ui: &mut egui::Ui,
    src: egui::ImageSource<'static>,
    text: &str,
) -> egui::Response {
    let tint = ui.visuals().text_color();
    let img = egui::Image::new(src)
        .fit_to_exact_size(egui::vec2(14.0, 14.0))
        .tint(tint);
    ui.add(egui::Button::image_and_text(img, text))
}
