//! First-run library chooser. No paths, SQL or release parsing in the normal flow.
use super::*;
use fontes_engine::bootstrap::{self, Event, Job, Offer, Profile, Progress};

#[derive(Default)]
pub(super) struct Setup {
    pub(super) visible: bool,
    pub(super) profile: Profile,
    pub(super) job: Option<Job>,
    pub(super) progress: Option<Progress>,
    pub(super) offer: Option<Offer>,
    pub(super) error: Option<String>,
    pub(super) cancelling: bool,
    pub(super) downloading: bool,
}
impl FontesApp {
    pub(super) fn receive_setup(&mut self) {
        let mut finished = None;
        if let Some(job) = &self.setup.job {
            loop {
                match job.events.try_recv() {
                    Ok(Event::Progress(progress)) => {
                        if let Some(offer) = progress.offer.clone() {
                            self.setup.offer = Some(offer);
                        }
                        self.setup.progress = Some(progress);
                    }
                    Ok(Event::Finished(result)) => {
                        finished = Some(result);
                        break;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        finished = Some(Err(
                            "Library setup stopped unexpectedly. Please retry.".into()
                        ));
                        break;
                    }
                }
            }
        }
        if let Some(result) = finished {
            self.setup.job = None;
            self.setup.offer = None;
            self.setup.progress = None;
            self.setup.downloading = false;
            let cancelled = self.setup.cancelling;
            self.setup.cancelling = false;
            match result {
                Ok(path) => {
                    self.library_path = path.display().to_string();
                    self.open();
                }
                Err(error) if !cancelled => self.setup.error = Some(error),
                Err(_) => {}
            }
        }
    }
    fn begin_setup(&mut self, ctx: &egui::Context) {
        self.setup.error = None;
        self.error = None;
        self.setup.cancelling = false;
        self.setup.downloading = false;
        self.setup.offer = None;
        let wake = ctx.clone();
        self.setup.job = Some(bootstrap::start(
            self.data_dir.clone(),
            self.setup.profile,
            move || wake.request_repaint(),
        ));
    }
    pub(super) fn welcome(&mut self, ui: &mut egui::Ui) {
        let busy = self.setup.job.is_some() || self.pending > 0;
        egui::CentralPanel::default().frame(egui::Frame::new().fill(theme::desk()).inner_margin(0)).show(ui, |ui| {
            egui::Panel::bottom("setup-actions").exact_size(100.0).frame(egui::Frame::new().fill(theme::desk()).inner_margin(24)).show(ui, |ui| {
                if self.pending > 0 {
                    ui.horizontal(|ui| { ui.spinner(); ui.label("Opening your library…"); });
                } else if self.setup.offer.is_some() && !self.setup.downloading && !self.setup.cancelling {
                    if primary(ui, "Download and open") {
                        if let Some(job) = &self.setup.job { job.approve(); }
                        self.setup.downloading = true;
                        self.setup.progress = None;
                    }
                } else if !busy {
                    if primary(ui, if self.setup.error.is_some() { "Retry setup" } else { "Continue" }) { self.begin_setup(ui.ctx()); }
                } else if ui.add_enabled(!self.setup.cancelling, egui::Button::new("Cancel setup").min_size(egui::vec2(ui.available_width(), 48.0))).clicked() {
                    if let Some(job) = &self.setup.job { job.cancel(); }
                    self.setup.cancelling = true;
                }
            });
            egui::ScrollArea::vertical().id_salt("setup-body").show(ui, |ui| {
                let inset = ((ui.available_width() - 600.0) / 2.0).clamp(24.0, 120.0);
                egui::Frame::new().inner_margin(egui::Margin::symmetric(inset as i8, 32)).show(ui, |ui| {
                    ui.set_max_width(ui.available_width().min(600.0));
                    ui.label(scripture("A place for deeper reading.", 32.0, true));
                    ui.add_space(16.0);
                    if let Some(offer) = self.setup.offer.as_ref() && !self.setup.downloading && !self.setup.cancelling {
                        ui.heading("Your library is ready to download");
                        ui.add_space(16.0);
                        ui.label(format!("Download: {}", size(offer.download_bytes)));
                        ui.label(format!("Library on this device: {}", size(offer.installed_bytes)));
                        ui.label(format!("Space needed during setup: about {}", size(offer.temporary_bytes)));
                        ui.add_space(16.0);
                        ui.label("Fontes will download, check and open your library. After setup, you can read offline.");
                        ui.add_space(12.0);
                        caption(ui, "Includes KJV (UK Crown rights apply) and the public-domain World English Bible.");
                        ui.add_space(16.0);
                        if ui.button("Choose another library").clicked() {
                            if let Some(job) = &self.setup.job { job.cancel(); }
                            self.setup.cancelling = true;
                        }
                    } else if busy {
                        let message = if self.setup.cancelling { "Cancelling setup…" } else if self.pending > 0 { "Opening your library…" } else { self.setup.progress.as_ref().map(|p| p.message.as_str()).unwrap_or(if self.setup.downloading { "Starting your download…" } else { "Finding your library…" }) };
                        ui.label(message);
                        ui.add_space(24.0);
                        if let Some(p) = &self.setup.progress && let Some(total) = p.total {
                            let fraction = if total > 0 { p.completed as f32 / total as f32 } else { 0.0 };
                            ui.add(egui::ProgressBar::new(fraction).show_percentage());
                            caption(ui, format!("{} of {}", size(p.completed), size(total)));
                        } else { ui.spinner(); }
                        ui.add_space(16.0);
                        ui.label("You can keep this screen open while your library is prepared. Your notes stay on this device.");
                    } else {
                        if self.state.is_some() && ui.button("Back to your library").clicked() {
                            self.setup.visible = false;
                            self.choose(Destination::Read);
                        }
                        ui.label("Choose your library. We’ll take care of downloading and opening it on this device.");
                        ui.add_space(24.0);
                        for (profile, title, description) in [
                            (Profile::Reading, "Reading", "KJV and World English Bible. A smaller download for everyday reading."),
                            (Profile::Study, "Study", "Both Bibles plus Strong’s word lookup, original Hebrew and Greek, Matthew Henry, Nave’s and cross-references."),
                        ] {
                            if ui.add_sized([ui.available_width(), 48.0], egui::Button::selectable(self.setup.profile == profile, title).right_text(())).clicked() { self.setup.profile = profile; }
                            ui.label(description);
                            ui.add_space(16.0);
                        }
                        caption(ui, "You’ll see the download size before it starts. No account required. Read offline after setup.");
                        if let Some(error) = &self.setup.error {
                            ui.add_space(20.0);
                            ui.colored_label(theme::color(assets::COLOR_ERROR), "Setup couldn’t finish. Check your connection and free storage, then retry.");
                            ui.collapsing("Details", |ui| { ui.label(error); });
                        } else if let Some(error) = &self.error {
                            ui.add_space(20.0);
                            ui.label("Your previous library couldn’t be opened. Download a fresh copy to get started.");
                            ui.collapsing("Details", |ui| { ui.label(error); });
                        }
                    }
                });
            });
        });
    }
}
fn size(bytes: u64) -> String {
    format!("{:.1} MB", bytes as f64 / 1_000_000.0)
}
