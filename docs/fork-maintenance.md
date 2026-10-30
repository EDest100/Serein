# Custom call and sharing fork

This fork retains Serein's upstream history and MIT OR Apache-2.0 licensing. Keep both
license files and third-party notices with redistributed builds.

The Windows picker has separate Screens and Apps tabs, with two preview tiles per row.
Each preview is a local, memory-only snapshot above the source name. Refresh takes new
snapshots. Only one discovery worker runs; closing the picker cancels remaining snapshots.
At most 64 sources, two queued snapshots and 64 textures of at most 320 x 180 are retained.
Unavailable or protected sources show an explicit placeholder and remain selectable.
This adds temporary native capture work while the picker is open; it is not a continuous
thumbnail feed. No snapshot is sent to Discord or saved to disk by this path.

Windows window discovery uses the same native filters for picking and starting capture:
visible, titled, top-level windows, excluding Serein and cloaked/helper windows. An explicit
`WS_EX_APPWINDOW` is allowed even when `WS_EX_TOOLWINDOW` is present. Display discovery
failure no longer prevents app discovery, and displays cannot consume every source slot.
The exact cause of an empty Apps list on an installed release has not been reproduced live.

Desktop audio is enabled by default on Windows, with a visible switch. It shares other
applications' playback (including when sharing one app), excluding Serein's entire process
tree to avoid call echo. The process-loopback backend requires Windows build 20348 or newer;
the picker checks the actual OS build and disables audio on older Windows. Microphone
settings remain independent. Audio initialization failures stop sharing with an error;
there is no silent fallback that captures the call itself.

Quality presets remain 480p, 720p, 1080p and 15/30/60 FPS. Custom dimensions must be even,
between 2 and 1920 pixels wide and 2 and 1080 high; custom FPS is an integer from 1 to 120.
These bounds retain the existing application video budget. Odd dimensions disable
Share with a validation message. Width, height and FPS reach the existing native capture
pacing, scaling, hardware/software H.264 encoders and stream signaling. Bitrate scales
with pixel count and FPS, bounded to 250 kbit/s–16 Mbit/s. A configured rate is a target;
the source refresh rate, capture events, encoder and network can limit delivered FPS.

The 1080p output cap is an application compatibility and memory-bound choice retained when
adding custom sizes, not a demonstrated maximum of H.264 or the Windows encoder. Upstream
`Settings::valid` accepted only the 480p/720p/1080p presets; the incoming video decoder caps
width at 1920 and pixels at 1920 x 1080 (`video_receive.rs`), and Linux raw-frame validation
also caps frames at 1920 x 1080 (`screen/gstreamer.rs`). Windows capture already accepts
source frames up to 3840 x 2160 and scales them for output; the Media Foundation encoder
uses the requested dimensions and the shared raw-byte bound rather than a literal 1080p
check. Larger custom output would need a coordinated review of validation, Linux handling,
receiver bounds, memory use and encoder/receiver tests. The validation remains in place.

The Windows branded in-app title strip and navigation arrows are removed. Native Windows
decorations provide dragging, minimize, maximize and close. Mouse side-button history navigation
remains available. Call controls are compact, with a direct Deafen/Undeafen control beside
screen sharing instead of a menu containing only that action.

## Update this fork without replacing its changes

`origin` is `https://github.com/EDest100/Serein.git`; `upstream` is
`https://github.com/ViceVerse-cz/Serein.git`. The custom branch is `feat/call-sharing`,
initially based on upstream commit `1b2f8942910436f9a488a0cf0f806776f02a074f`.

The in-app updater is deliberately source-only in this fork (`SOURCE_ONLY_FORK` in
`apps/desktop/src/updater.rs`). All non-demo update requests return before checking,
downloading or installing upstream binaries. Its settings explain how to update. Do not
enable the stock release updater: those binaries do not contain this fork's changes.
Synthetic demo update previews continue to work without network or installation.

From a clean checkout, update as follows in PowerShell:

```powershell
git switch feat/call-sharing
git status --short
git fetch upstream
git switch -c update/upstream-YYYY-MM-DD
git merge upstream/main
# Resolve any conflicts, preserving the custom capture/UI changes and source-only updater.
cargo xtask check
cargo check --locked -p serein --features demo
cargo xtask package
git diff feat/call-sharing..HEAD
git switch feat/call-sharing
git merge --ff-only update/upstream-YYYY-MM-DD
git push origin feat/call-sharing
```

Run with the pinned Rust toolchain and required native build tools from the repository
instructions. Inspect the merged updater and these customization files before building.
Do not run this sequence with unrelated uncommitted changes. Keep the merge commit so the
next upstream merge knows which changes are already incorporated; do not reset the branch
to upstream. Future upstream changes may require manual conflict resolution and re-testing.

Replace only this fork's executable/package with the rebuilt custom artifact after quitting
it. Do not install a stock upstream release over the custom build. No automatic merge,
public release publishing or custom binary update feed is configured by this change.

## Offline and live verification

Use `cargo run --locked -p serein --features demo -- --demo --demo-call --demo-screen-share`
to inspect the synthetic picker, and `--demo-light` for light mode. The demo sources and
thumbnails are synthetic; Share is disabled. Resize to the minimum 760 x 520 and exercise
scrolling, Screens/Apps, quality presets, custom values, and the call control bar.

Live capture, desktop audio, protected/minimized windows and remote viewing require a
separately authorized private developer session. Compilation and synthetic fixtures do not
establish live Discord interoperability. No live call or audio capture was used for this
implementation. Native screenshots and release performance/package comparisons require a
successful executable build and sufficient local disk space.

Initial verification on Windows build 22631, Rust 1.98.1 (2026-10-30):

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` and `git diff --check` | Passed |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | Passed |
| `cargo check --locked --offline -p serein --features demo` | Passed |
| `cargo check --locked --offline -p serein --no-default-features` | Passed |
| `cargo test --locked --offline -p client-core screen::tests` | 1 passed |
| `xtask policy` | Passed using the already-built repository xtask executable |
| `cargo xtask check` | Format and Clippy passed; test compilation stopped with OS error 112 (disk full) |
| Executable build / native screenshots | Both the initial demo build and a final build with debug symbols disabled and `-j 2` stopped with OS error 112; no executable or native images produced |
| `cargo xtask package` / release size and runtime comparison | Not run: insufficient disk space established by the failed build and tests |
| Live app/window previews, desktop audio, remote viewers | Not tested; no live capture or Discord session used |

Checks used `CARGO_PROFILE_DEV_DEBUG=0` and `CARGO_INCREMENTAL=0` after the first failed build
to reduce disk use; the focused test also used `CARGO_PROFILE_TEST_DEBUG=0`. The repository's
pinned dependency lockfile and toolchain were retained. Four upstream demo role initializers
were missing the `managed` field; this fork supplies `false` so the demo check compiles.
The remaining new UI/window/updater regression tests were type-checked by workspace Clippy,
but were not executed because the workspace test build exceeded available disk space.
The final symbol-free build started with about 2.3 GiB free and still ran out of space while
building dependencies. Failed task build output was removed afterward; verification logs remain
beside the checkout. Provide additional free disk space or a larger build volume before retrying
the executable, workspace tests and package. No unrelated files or running tasks were cleaned.
