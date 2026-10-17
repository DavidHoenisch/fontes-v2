//! Ephemeral, app-only backdrop blur. No filesystem, network or OS-screen capture.
use eframe::egui::{self, Color32, ColorImage};
use fontes_engine::ReadingTheme;

#[derive(Clone, Copy)]
pub(super) struct Capture(pub u64);
#[derive(Default)]
pub(super) struct Backdrop {
    pub stamp: u64,
    active: bool,
    rect: Option<egui::Rect>,
    theme: Option<ReadingTheme>,
    texture: Option<egui::TextureHandle>,
    receiver: Option<std::sync::mpsc::Receiver<(u64, ColorImage)>>,
}
impl Backdrop {
    /// Capture one clean application frame before the sheet. Unsupported readback
    /// falls back to the ordinary tinted scrim on the next frame without blocking.
    #[cfg(test)]
    pub fn ready(&self) -> bool {
        self.texture.is_some()
    }
    pub fn prepare(&mut self, ctx: &egui::Context, open: bool, theme: ReadingTheme) -> bool {
        if !open {
            if self.active {
                self.stamp = self.stamp.wrapping_add(1);
            }
            self.active = false;
            self.texture = None;
            self.receiver = None;
            self.rect = None;
            return false;
        }
        let rect = ctx.viewport_rect();
        if !self.active || self.rect != Some(rect) {
            self.active = true;
            self.rect = Some(rect);
            self.theme = Some(theme);
            self.texture = None;
            self.stamp = self.stamp.wrapping_add(1);
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(
                Capture(self.stamp),
            )));
            ctx.request_repaint();
            return false;
        }
        if self.theme != Some(theme) {
            // Never paint a stale palette behind freshly selected sheet colors.
            self.texture = None;
            self.theme = Some(theme);
            self.stamp = self.stamp.wrapping_add(1);
        }
        let image = ctx.input(|i| {
            i.events.iter().find_map(|event| {
                if let egui::Event::Screenshot {
                    user_data, image, ..
                } = event
                    && user_data.data.as_ref()?.downcast_ref::<Capture>()?.0 == self.stamp
                {
                    Some(image.clone())
                } else {
                    None
                }
            })
        });
        if let Some(receiver) = &self.receiver {
            match receiver.try_recv() {
                Ok((stamp, image)) => {
                    if stamp == self.stamp {
                        self.texture = Some(ctx.load_texture(
                            "fontes-sheet-backdrop",
                            image,
                            egui::TextureOptions::LINEAR,
                        ));
                    }
                    self.receiver = None;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => self.receiver = None,
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
            }
        }
        if let Some(image) = image
            && self.receiver.is_none()
        {
            let (sender, receiver) = std::sync::mpsc::channel();
            self.receiver = Some(receiver);
            let stamp = self.stamp;
            let wake = ctx.clone();
            std::thread::spawn(move || {
                if let Some(image) = blur(&image) {
                    let _ = sender.send((stamp, image));
                }
                wake.request_repaint();
            });
        }
        if let Some(texture) = &self.texture {
            egui::Area::new(egui::Id::new("sheet-backdrop-blur"))
                .order(egui::Order::Middle)
                .interactable(false)
                .movable(false)
                .constrain(false)
                .fixed_pos(rect.min)
                .show(ctx, |ui| {
                    ui.set_min_size(rect.size());
                    ui.painter().with_clip_rect(ctx.content_rect()).image(
                        texture.id(),
                        rect,
                        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                });
        }
        true
    }
}
fn blur(source: &ColorImage) -> Option<ColorImage> {
    let [sw, sh] = source.size;
    if sw == 0 || sh == 0 || sw.checked_mul(sh)? > 16_000_000 || source.pixels.len() != sw * sh {
        return None;
    }
    let scale = (sw.max(sh) as f32 / 512.0).max(2.0);
    let w = ((sw as f32 / scale).ceil() as usize).clamp(1, 512);
    let h = ((sh as f32 / scale).ceil() as usize).clamp(1, 512);
    let mut pixels = (0..w * h)
        .map(|i| {
            let x = ((i % w) as f32 * scale) as usize;
            let y = ((i / w) as f32 * scale) as usize;
            source.pixels[y.min(sh - 1) * sw + x.min(sw - 1)]
        })
        .collect::<Vec<_>>();
    let mut scratch = pixels.clone();
    // Three bounded box passes approximate the reference's soft Gaussian blur.
    for _ in 0..3 {
        for vertical in [false, true] {
            for y in 0..h {
                for x in 0..w {
                    let mut sum = [0_u32; 3];
                    for delta in -2_isize..=2 {
                        let xx = if vertical {
                            x
                        } else {
                            x.saturating_add_signed(delta).min(w - 1)
                        };
                        let yy = if vertical {
                            y.saturating_add_signed(delta).min(h - 1)
                        } else {
                            y
                        };
                        let c = pixels[yy * w + xx];
                        sum[0] += u32::from(c.r());
                        sum[1] += u32::from(c.g());
                        sum[2] += u32::from(c.b());
                    }
                    scratch[y * w + x] = Color32::from_rgb(
                        (sum[0] / 5) as u8,
                        (sum[1] / 5) as u8,
                        (sum[2] / 5) as u8,
                    );
                }
            }
            std::mem::swap(&mut pixels, &mut scratch);
        }
    }
    Some(ColorImage::new([w, h], pixels))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn blur_is_bounded_and_softens_edges_without_changing_solid_surfaces() {
        let solid = ColorImage::filled([80, 60], Color32::from_rgb(68, 45, 41));
        let result = blur(&solid).unwrap();
        assert!(result.pixels.iter().all(|p| *p == solid.pixels[0]));
        let mut split = ColorImage::filled([80, 60], Color32::WHITE);
        for y in 0..60 {
            for x in 0..40 {
                split.pixels[y * 80 + x] = Color32::BLACK;
            }
        }
        let result = blur(&split).unwrap();
        assert!(result.pixels.iter().any(|p| p.r() > 0 && p.r() < 255));
        assert!(result.size.iter().all(|n| *n <= 512));
        let wide = blur(&ColorImage::filled([1025, 33], Color32::WHITE)).unwrap();
        assert!(wide.size.iter().all(|n| *n <= 512));
        let mut oversized = ColorImage::filled([1, 1], Color32::WHITE);
        oversized.size = [100_000, 100_000];
        assert!(blur(&oversized).is_none());
    }
}
