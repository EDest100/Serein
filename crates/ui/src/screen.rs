//! Screen selection is a local intent; the desktop owns capture and stream credentials.
use client_core::{
	State,
	screen::{Settings, Source, SourceId},
	voice::Phase,
};
use model::Id;

pub enum Request {
	Start(Settings),
	Stop,
}

pub struct ScreenUi {
	pub context: Option<(u64, Id, u64)>,
	pub open: bool,
	pub sources: Vec<Source>,
	pub selected: Option<SourceId>,
	pub refresh_requested: bool,
	pub request: Option<Request>,
	pub busy: bool,
	pub status: &'static str,
	pub capture_status: Option<&'static str>,
	pub supported: bool,
	pub audio_supported: bool,
	pub scanning: bool,
	pub thumbnails: Vec<(SourceId, egui::TextureHandle)>,
	pub preview: Option<egui::TextureHandle>,
	width: u32,
	height: u32,
	fps: u32,
	custom_resolution: bool,
	custom_fps: bool,
	apps: bool,
	cursor: bool,
	audio: bool,
}
impl Default for ScreenUi {
	fn default() -> Self {
		Self {
			context: None,
			open: false,
			sources: Vec::new(),
			selected: None,
			refresh_requested: false,
			request: None,
			busy: false,
			status: "",
			capture_status: None,
			supported: false,
			audio_supported: false,
			scanning: false,
			thumbnails: Vec::new(),
			preview: None,
			width: if cfg!(target_os = "linux") {
				1280
			} else {
				1920
			},
			height: if cfg!(target_os = "linux") { 720 } else { 1080 },
			fps: 30,
			custom_resolution: false,
			custom_fps: false,
			apps: false,
			cursor: true,
			audio: cfg!(any(target_os = "macos", target_os = "windows")),
		}
	}
}
impl ScreenUi {
	pub fn set_sources(&mut self, mut sources: Vec<Source>) {
		sources.truncate(client_core::screen::MAX_SOURCES);
		self.sources = sources;
		self.thumbnails.clear();
		self.selected = self
			.selected
			.filter(|id| {
				self.sources
					.iter()
					.any(|source| source.id == *id && self.visible_source(source.id))
			})
			.or_else(|| {
				self.sources
					.iter()
					.find(|source| self.visible_source(source.id))
					.map(|source| source.id)
			});
	}
	fn visible_source(&self, id: SourceId) -> bool {
		!cfg!(target_os = "windows") || matches!(id, SourceId::Window(_)) == self.apps
	}
	pub(crate) fn launch(&mut self, state: &State) {
		let Some(call) = &state.voice.active else {
			return;
		};
		if self.busy {
			self.request = Some(Request::Stop);
			return;
		}
		self.context = Some((state.generation, call.channel, call.request));
		self.open = true;
		self.selected = None;
		self.sources.clear();
		self.thumbnails.clear();
		if state.demo {
			self.sources = if cfg!(target_os = "macos") {
				vec![Source {
					id: SourceId::SystemPicker,
					name: "Choose with the macOS system picker".into(),
				}]
			} else {
				vec![
					Source {
						id: SourceId::Display(1),
						name: "Display 1 · Synthetic preview".into(),
					},
					Source {
						id: SourceId::Window(2),
						name: "Project notes · Synthetic window".into(),
					},
					Source {
						id: SourceId::Display(3),
						name: "Display 2 · Synthetic preview".into(),
					},
					Source {
						id: SourceId::Window(4),
						name: "Design review · Synthetic window".into(),
					},
				]
			};
			self.selected = self
				.sources
				.iter()
				.find(|source| self.visible_source(source.id))
				.map(|source| source.id);
			self.status = "Offline preview · no screen is captured";
		} else {
			self.refresh_requested = true;
			self.status = "Looking for screens and windows…";
		}
	}
	fn settings(&self) -> Option<Settings> {
		let source = self.selected.filter(|id| {
			self.sources
				.iter()
				.any(|s| s.id == *id && self.visible_source(s.id))
		})?;
		let settings = Settings {
			source,
			width: self.width,
			height: self.height,
			fps: self.fps,
			cursor: self.cursor,
			audio: self.audio && self.audio_supported,
		};
		settings.valid().then_some(settings)
	}
	pub(super) fn show(&mut self, ctx: &egui::Context, state: &State) {
		let current = state
			.voice
			.active
			.as_ref()
			.filter(|c| c.phase != Phase::Failed)
			.map(|c| (state.generation, c.channel, c.request));
		if self.context.is_some() && current != self.context {
			self.preview = None;
			self.open = false;
			self.sources.clear();
			self.thumbnails.clear();
			self.selected = None;
			self.request = None;
			self.refresh_requested = false;
		}
		if !self.open {
			self.thumbnails.clear();
			return;
		}
		let mut cancel = false;
		let mut share = false;
		if state.demo && self.thumbnails.is_empty() {
			for (index, source) in self.sources.iter().enumerate() {
				let mut pixels = vec![egui::Color32::from_rgb(27, 32, 46); 320 * 180];
				for y in 12..168 {
					for x in 12..308 {
						pixels[y * 320 + x] = if x < 68 {
							egui::Color32::from_rgb(42, 51, 72)
						} else if y < 36 {
							egui::Color32::from_rgb(42, 86, 130)
						} else if y % 28 < 8 && x < 270 - index * 10 {
							egui::Color32::from_rgb(75, 91, 112)
						} else {
							egui::Color32::from_rgb(32, 39, 54)
						};
					}
				}
				self.thumbnails.push((
					source.id,
					ctx.load_texture(
						format!("synthetic-source-{index}"),
						egui::ColorImage::new([320, 180], pixels),
						egui::TextureOptions::LINEAR,
					),
				));
			}
		}
		let response = crate::dialog::Dialog::new(
			"screen-share-settings",
			crate::i18n::translate("screen-show-share-your-screen"),
		)
		.subtitle(crate::i18n::translate(
			"screen-show-choose-what-people-in-this-call-can-see",
		))
		.width(560.0)
		.show(ctx, |d| {
			d.scroll(260.0, |ui| self.body(ui, state));
			d.footer(|ui| {
				let allowed = !state.demo
					&& self.supported
					&& !self.busy && self.settings().is_some()
					&& state.voice.active.as_ref().is_some_and(|call| {
						matches!(call.phase, Phase::Connected | Phase::Waiting)
							&& state.can_stream(call.channel)
					});
				ui.add_enabled_ui(allowed, |ui| {
					share = crate::dialog::action(
						ui,
						"screen-show-share-screen",
						crate::dialog::Action::Primary,
					)
					.clicked();
				});
				cancel |=
					crate::dialog::action(ui, "screen-show-cancel", crate::dialog::Action::Neutral)
						.clicked();
			});
		});
		cancel |= response.close;
		if share && !cancel {
			self.request = self.settings().map(Request::Start);
		}
		if cancel || share {
			self.open = false;
		}
	}

	/// Source list, then the capture options Discord exposes without an entitlement.
	fn body(&mut self, ui: &mut egui::Ui, state: &State) {
		let colors = crate::design::palette(ui);
		ui.horizontal(|ui| {
			ui.label(crate::design::eyebrow(
				ui,
				crate::i18n::translate("screen-body-screen-or-window"),
				colors.muted,
			));
			ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
				if ui
					.add_enabled_ui(
						!state.demo && !self.scanning && !cfg!(target_os = "linux"),
						|ui| {
							crate::design::text_action(
								ui,
								&crate::i18n::translate("screen-body-refresh"),
							)
						},
					)
					.inner
					.clicked()
				{
					self.selected = None;
					self.sources.clear();
					self.thumbnails.clear();
					self.refresh_requested = true;
					self.status = "Looking for screens and windows…";
				}
			});
		});
		ui.add_space(6.0);
		if cfg!(target_os = "windows") || state.demo {
			ui.horizontal(|ui| {
				for (apps, name) in [(false, "Screens"), (true, "Apps")] {
					if segment(ui, name, self.apps == apps).clicked() {
						self.apps = apps;
						self.selected = self
							.sources
							.iter()
							.find(|source| matches!(source.id, SourceId::Window(_)) == apps)
							.map(|source| source.id);
					}
				}
				if self.scanning {
					ui.spinner();
				}
			});
			ui.add_space(6.0);
		}
		egui::ScrollArea::vertical()
			.id_salt("screen-sources")
			.max_height(254.0)
			.show(ui, |ui| self.source_list(ui));
		ui.add_space(14.0);
		ui.label(crate::design::eyebrow(
			ui,
			crate::i18n::translate("screen-body-quality"),
			colors.muted,
		));
		ui.add_space(6.0);
		ui.horizontal_wrapped(|ui| {
			ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
			for (width, height) in [(854, 480), (1280, 720), (1920, 1080)] {
				if segment(
					ui,
					&format!("{height}p"),
					!self.custom_resolution && self.height == height,
				)
				.clicked()
				{
					self.custom_resolution = false;
					self.width = width;
					self.height = height;
				}
			}
			if segment(ui, "Custom", self.custom_resolution).clicked() {
				self.custom_resolution = true;
			}
		});
		if self.custom_resolution {
			ui.horizontal(|ui| {
				ui.label("Width");
				ui.add(
					egui::DragValue::new(&mut self.width)
						.range(2..=client_core::screen::MAX_WIDTH)
						.speed(2),
				);
				ui.label("Height");
				ui.add(
					egui::DragValue::new(&mut self.height)
						.range(2..=client_core::screen::MAX_HEIGHT)
						.speed(2),
				);
			});
			ui.label(
				egui::RichText::new("Even dimensions, up to 1920 × 1080 pixels.")
					.small()
					.color(colors.muted),
			);
		}
		ui.add_space(8.0);
		ui.label(crate::design::eyebrow(
			ui,
			crate::i18n::translate("screen-body-frame-rate"),
			colors.muted,
		));
		ui.add_space(6.0);
		ui.horizontal_wrapped(|ui| {
			ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
			for fps in [15, 30, 60] {
				if segment(
					ui,
					&format!("{fps} fps"),
					!self.custom_fps && self.fps == fps,
				)
				.clicked()
				{
					self.custom_fps = false;
					self.fps = fps;
				}
			}
			if segment(ui, "Custom", self.custom_fps).clicked() {
				self.custom_fps = true;
			}
		});
		if self.custom_fps {
			ui.horizontal(|ui| {
				ui.label("Frames per second");
				ui.add(
					egui::DragValue::new(&mut self.fps)
						.range(1..=client_core::screen::MAX_FPS)
						.suffix(" fps"),
				);
			});
		}
		if self.selected.is_some() && self.settings().is_none() {
			ui.colored_label(
				colors.danger,
				"Choose even dimensions within the supported range and 1–120 FPS.",
			);
		}
		ui.add_space(4.0);
		ui.add(
			egui::Label::new(
				egui::RichText::new(crate::i18n::translate(
					"screen-body-quality-selection-does-not-require-nitro",
				))
				.size(12.0)
				.color(colors.muted),
			)
			.wrap(),
		);
		ui.add_space(10.0);
		crate::design::switch(
			ui,
			"screen-body-show-cursor",
			Some("screen-body-include-the-pointer-in-the-shared-video"),
			&mut self.cursor,
		);
		if self.supported || state.demo {
			ui.add_space(6.0);
			ui.add_enabled_ui(self.audio_supported || state.demo, |ui| {
				crate::design::switch(
					ui,
					"screen-body-share-system-audio",
					Some(if cfg!(target_os = "macos") {
						"screen-body-send-what-your-mac-plays-along-with-the-screen-serein"
					} else {
						"screen-body-share-sound-from-other-apps-even-when-sharing-one-window"
					}),
					&mut self.audio,
				);
			});
			if !self.audio_supported && !state.demo {
				ui.label(
					egui::RichText::new(
						"Desktop audio requires Windows build 20348 or newer on Windows.",
					)
					.small()
					.color(colors.muted),
				);
			}
		}
		ui.add_space(4.0);
		ui.add(
			egui::Label::new(
				egui::RichText::new(crate::i18n::translate(
					"screen-body-your-call-microphone-keeps-its-current-settings",
				))
				.size(12.0)
				.color(colors.muted),
			)
			.wrap(),
		);
		if !self.status.is_empty() {
			ui.add_space(10.0);
			egui::Frame::new()
				.fill(colors.base)
				.corner_radius(8)
				.inner_margin(egui::Margin::symmetric(12, 10))
				.show(ui, |ui| {
					ui.set_width(ui.available_width());
					ui.add(
						egui::Label::new(
							egui::RichText::new(self.status)
								.size(12.0)
								.color(colors.muted),
						)
						.wrap(),
					);
				});
		}
	}

	fn source_list(&mut self, ui: &mut egui::Ui) {
		let colors = crate::design::palette(ui);
		let sources: Vec<_> = self
			.sources
			.iter()
			.filter(|source| self.visible_source(source.id))
			.collect();
		if sources.is_empty() {
			ui.add_space(16.0);
			ui.label(if self.scanning {
				"Looking for sources…"
			} else if self.apps {
				"No shareable app windows. Open or restore an app, then Refresh."
			} else {
				"No screens available. Try Refresh."
			});
			ui.add_space(16.0);
			return;
		}
		let width = ((ui.available_width() - 10.0) / 2.0).max(80.0);
		for pair in sources.chunks(2) {
			ui.horizontal(|ui| {
				ui.spacing_mut().item_spacing.x = 10.0;
				for source in pair {
					let selected = self.selected == Some(source.id);
					let size = egui::vec2(width, (width - 12.0) * 9.0 / 16.0 + 44.0);
					let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
					if !ui.is_rect_visible(rect) {
						continue;
					}
					let fill = if selected {
						colors.selected
					} else if response.hovered() || response.has_focus() {
						colors.hover
					} else {
						colors.base
					};
					ui.painter().rect_filled(rect, 8, fill);
					ui.painter().rect_stroke(
						rect,
						8,
						egui::Stroke::new(
							1.0,
							if selected || response.has_focus() {
								colors.accent
							} else {
								colors.border
							},
						),
						egui::StrokeKind::Inside,
					);
					let preview = egui::Rect::from_min_size(
						rect.min + egui::vec2(6.0, 6.0),
						egui::vec2(width - 12.0, (width - 12.0) * 9.0 / 16.0),
					);
					ui.painter().rect_filled(preview, 4, colors.chat);
					if let Some((_, texture)) =
						self.thumbnails.iter().find(|(id, _)| *id == source.id)
					{
						let fit = egui::Rect::from_center_size(
							preview.center(),
							texture.size_vec2()
								* (preview.width() / texture.size_vec2().x)
									.min(preview.height() / texture.size_vec2().y),
						);
						ui.painter().image(
							texture.id(),
							fit,
							egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
							egui::Color32::WHITE,
						);
					} else {
						ui.painter().text(
							preview.center(),
							egui::Align2::CENTER_CENTER,
							if self.scanning {
								"Loading preview…"
							} else {
								"Preview unavailable"
							},
							egui::FontId::proportional(12.0),
							colors.muted,
						);
					}
					let text = egui::Rect::from_min_max(
						egui::pos2(rect.left() + 10.0, preview.bottom() + 7.0),
						rect.right_bottom() - egui::vec2(10.0, 6.0),
					);
					let mut label_ui = ui.new_child(egui::UiBuilder::new().max_rect(text));
					label_ui.add(
						egui::Label::new(
							crate::design::medium(ui, &source.name, 13.0).color(colors.text_strong),
						)
						.truncate(),
					);
					response.widget_info(|| {
						egui::WidgetInfo::selected(
							egui::Role::RadioButton,
							true,
							selected,
							&source.name,
						)
					});
					if response.on_hover_text(&source.name).clicked() {
						self.selected = Some(source.id);
					}
				}
			});
			ui.add_space(6.0);
		}
	}
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
	use super::*;
	#[test]
	fn app_refresh_never_silently_selects_a_screen_or_a_stale_window() {
		let mut picker = ScreenUi {
			apps: true,
			selected: Some(SourceId::Window(99)),
			..Default::default()
		};
		let display = Source {
			id: SourceId::Display(1),
			name: "Screen".into(),
		};
		picker.set_sources(vec![
			display.clone(),
			Source {
				id: SourceId::Window(2),
				name: "App".into(),
			},
		]);
		assert_eq!(picker.selected, Some(SourceId::Window(2)));
		picker.set_sources(vec![display]);
		assert!(picker.selected.is_none());
		assert!(picker.settings().is_none());
	}
}

/// Compact segmented choice used by the quality and frame-rate rows.
fn segment(ui: &mut egui::Ui, label: &str, selected: bool) -> egui::Response {
	let label = crate::i18n::translate_if_key(label);
	let colors = crate::design::palette(ui);
	let galley = ui.painter().layout_no_wrap(
		label.to_owned(),
		egui::FontId::new(13.0, crate::design::medium_family(ui.ctx())),
		colors.text_strong,
	);
	let (rect, response) = ui.allocate_exact_size(
		egui::vec2(galley.size().x + 24.0, 32.0),
		egui::Sense::click(),
	);
	let fill = if selected {
		colors.accent
	} else if response.hovered() || response.has_focus() {
		colors.hover
	} else {
		colors.base
	};
	ui.painter().rect_filled(rect, 8, fill);
	let color = if selected {
		colors.accent_text
	} else {
		colors.text
	};
	ui.painter().galley(
		egui::pos2(
			rect.center().x - galley.size().x * 0.5,
			rect.center().y - galley.size().y * 0.5,
		),
		galley,
		color,
	);
	response.widget_info(|| {
		egui::WidgetInfo::selected(egui::Role::RadioButton, true, selected, &label)
	});
	response
}
