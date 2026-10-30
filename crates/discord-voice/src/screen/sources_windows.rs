#![allow(unsafe_code)]

use windows::{
	Win32::{
		Foundation::{HWND, LPARAM},
		Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute},
		UI::WindowsAndMessaging::{
			EnumWindows, GWL_EXSTYLE, GWL_STYLE, GetWindowLongPtrW, GetWindowThreadProcessId,
			IsWindowVisible, WS_CHILD, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
		},
	},
	core::BOOL,
};
use windows_capture::window::Window;

fn eligible(
	visible: bool,
	own_process: bool,
	child: bool,
	tool: bool,
	app: bool,
	cloaked: bool,
) -> bool {
	visible && !own_process && !child && (!tool || app) && !cloaked
}

pub(super) fn windows() -> Result<Vec<Window>, &'static str> {
	unsafe extern "system" fn visit(hwnd: HWND, data: LPARAM) -> BOOL {
		let windows = unsafe { &mut *(data.0 as *mut Vec<Window>) };
		if windows.len() >= super::MAX_SOURCES {
			return BOOL(1);
		}
		let mut pid = 0;
		let mut cloaked = 0u32;
		unsafe {
			GetWindowThreadProcessId(hwnd, Some(&mut pid));
			let _ = DwmGetWindowAttribute(
				hwnd,
				DWMWA_CLOAKED,
				(&mut cloaked as *mut u32).cast(),
				std::mem::size_of::<u32>() as u32,
			);
			let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
			let extended = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
			if eligible(
				IsWindowVisible(hwnd).as_bool(),
				pid == std::process::id(),
				style & WS_CHILD.0 != 0,
				extended & WS_EX_TOOLWINDOW.0 != 0,
				extended & WS_EX_APPWINDOW.0 != 0,
				cloaked != 0,
			) {
				let window = Window::from_raw_hwnd(hwnd.0);
				if window.title().is_ok_and(|title| !title.trim().is_empty())
					&& window
						.width()
						.is_ok_and(|width| width > 0 && width <= super::MAX_SOURCE_WIDTH as i32)
					&& window
						.height()
						.is_ok_and(|height| height > 0 && height <= super::MAX_SOURCE_HEIGHT as i32)
				{
					windows.push(window);
				}
			}
		}
		BOOL(1)
	}
	let mut windows = Vec::with_capacity(super::MAX_SOURCES);
	unsafe {
		EnumWindows(
			Some(visit),
			LPARAM((&mut windows as *mut Vec<Window>) as isize),
		)
	}
	.map_err(|_| "Application windows could not be enumerated")?;
	Ok(windows)
}

#[cfg(test)]
mod tests {
	use super::eligible;
	#[test]
	fn app_window_style_survives_tool_filter_without_exposing_hidden_helpers() {
		assert!(eligible(true, false, false, false, false, false));
		assert!(eligible(true, false, false, true, true, false));
		assert!(!eligible(true, false, false, true, false, false));
		assert!(!eligible(false, false, false, false, true, false));
		assert!(!eligible(true, true, false, false, true, false));
		assert!(!eligible(true, false, true, false, true, false));
		assert!(!eligible(true, false, false, false, true, true));
	}
}
