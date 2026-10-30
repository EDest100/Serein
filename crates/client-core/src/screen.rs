//! Bounded screen-share settings and ephemeral Gateway negotiation events.
use crate::voice;
use model::Id;

pub const MAX_SOURCES: usize = 64;
pub const MAX_SOURCE_NAME_BYTES: usize = 256;
pub const MAX_WIDTH: u32 = 1920;
pub const MAX_HEIGHT: u32 = 1080;
pub const MAX_FPS: u32 = 120;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceId {
	/// macOS chooses a display/window only after an explicit Share action.
	SystemPicker,
	/// The Linux desktop chooses the source after an explicit Share action.
	Portal,
	/// Explicit whole-desktop capture on a native X11 session, without a portal.
	X11Desktop,
	Display(u64),
	Window(u64),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Source {
	pub id: SourceId,
	pub name: String,
}

#[derive(Clone, Copy, Debug)]
pub struct Settings {
	pub source: SourceId,
	pub width: u32,
	pub height: u32,
	pub fps: u32,
	pub cursor: bool,
	/// Share system audio with the screen; call microphone settings are independent.
	pub audio: bool,
}
impl Settings {
	pub fn valid(self) -> bool {
		!matches!(self.source, SourceId::Display(0) | SourceId::Window(0))
			&& (2..=MAX_WIDTH).contains(&self.width)
			&& (2..=MAX_HEIGHT).contains(&self.height)
			&& self.width.is_multiple_of(2)
			&& self.height.is_multiple_of(2)
			&& (1..=MAX_FPS).contains(&self.fps)
	}

	pub fn bit_rate(self) -> u32 {
		let pixels = u64::from(self.width) * u64::from(self.height);
		let rate = pixels
			.saturating_mul(4)
			.saturating_mul(u64::from(self.fps.max(30)))
			/ 30;
		rate.clamp(250_000, 16_000_000) as u32
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn custom_quality_is_bounded_and_scales_bitrate_without_overflow() {
		let mut settings = Settings {
			source: SourceId::Display(1),
			width: 1600,
			height: 900,
			fps: 48,
			cursor: true,
			audio: true,
		};
		assert!(settings.valid());
		let bitrate = settings.bit_rate();
		settings.fps = 96;
		assert!(settings.valid());
		assert!(settings.bit_rate() > bitrate);
		for (width, height, fps) in [
			(0, 900, 48),
			(1601, 900, 48),
			(1600, 901, 48),
			(1922, 900, 48),
			(1600, 1082, 48),
			(1600, 900, 0),
			(1600, 900, 121),
		] {
			settings.width = width;
			settings.height = height;
			settings.fps = fps;
			assert!(!settings.valid());
		}
		settings.width = u32::MAX;
		settings.height = u32::MAX;
		settings.fps = u32::MAX;
		assert!(!settings.valid());
		assert_eq!(settings.bit_rate(), 16_000_000);
		settings.source = SourceId::Window(0);
		settings.width = 1280;
		settings.height = 720;
		settings.fps = 60;
		assert!(!settings.valid());
	}
}

#[derive(Debug)]
pub enum Event {
	Created {
		rtc_server: Id,
		rtc_channel: Id,
	},
	Server {
		token: Option<voice::Secret>,
		endpoint: Option<String>,
	},
	/// The stream is gone. `reason` names Discord's cause when it sent one we recognise.
	Deleted {
		reason: Option<&'static str>,
	},
	Failed(&'static str),
	/// Discord's latest viewer count for this device's own stream (unofficial `STREAM_UPDATE`).
	Viewers(u16),
}
impl Event {
	pub(crate) fn bytes(&self) -> usize {
		match self {
			Self::Server { token, endpoint } => {
				token.as_ref().map_or(0, voice::Secret::bytes)
					+ endpoint.as_ref().map_or(0, String::capacity)
			}
			_ => 0,
		}
	}
}
