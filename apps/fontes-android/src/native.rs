use eframe::{App, egui};
use std::{path::PathBuf, time::Duration};
use winit::platform::android::activity::AndroidApp;

/// Platform input adaptation stays at the Android edge; the shared UI uses egui's
/// content rectangle for all panels, overlays and hit testing.
pub(super) struct AndroidFontesApp {
    ui: fontes_ui_egui::FontesApp,
    app: AndroidApp,
    bars: super::system_bars::SystemBars,
}

impl AndroidFontesApp {
    pub(super) fn new(
        cc: &eframe::CreationContext<'_>,
        app: AndroidApp,
        data_dir: PathBuf,
    ) -> Self {
        Self {
            ui: fontes_ui_egui::FontesApp::new(cc, None, data_dir),
            app,
            bars: Default::default(),
        }
    }
}

impl App for AndroidFontesApp {
    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        self.ui.clear_color(visuals)
    }
    fn raw_input_hook(&mut self, ctx: &egui::Context, input: &mut egui::RawInput) {
        // winit 0.30 drops ContentRectChanged instead of requesting a redraw.
        // Refresh while idle too, so system-bar/IME changes are eventually observed.
        ctx.request_repaint_after(Duration::from_millis(250));
        let Some(window) = self.app.native_window() else {
            return;
        };
        let content = self.app.content_rect();
        let pixels_per_point = input
            .viewports
            .get(&input.viewport_id)
            .and_then(|v| v.native_pixels_per_point)
            .map_or_else(|| ctx.pixels_per_point(), |scale| scale * ctx.zoom_factor());
        let surface = [window.width(), window.height()];
        let mut margins = super::insets::content_insets(
            [content.left, content.top, content.right, content.bottom],
            surface,
            pixels_per_point,
        );
        if let Some(sample) = self.bars.poll(&self.app, ctx).filter(|s| s.size == surface)
            && let Some(bars) =
                super::insets::content_insets(sample.content, surface, pixels_per_point)
        {
            let native = margins.unwrap_or([0.0; 4]);
            margins = Some(std::array::from_fn(|i| native[i].max(bars[i])));
        }
        if let Some([left, top, right, bottom]) = margins {
            input.safe_area_insets = Some(egui::SafeAreaInsets(egui::epaint::MarginF32 {
                left,
                top,
                right,
                bottom,
            }));
        }
    }

    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.ui.logic(ctx, frame);
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.ui.ui(ui, frame);
    }
}
