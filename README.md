# overview

English · [中文](docs/zh/README.md)

Browse and switch Zellij sessions, tabs, and panes from a floating window.

Press `Ctrl+y` to open, move with `hjkl`, and jump with `e`. When there are many items, press `s` to search a title fragment, then press the tip next to a match. `Space` is the layer leader: `s` for sessions, `t` back to tabs, `p` for panes in the current tab.

The layout switches between framed cards, compact cards, and a scrolling list based on title length and the floating pane size.

<video src="asset/open.mov" controls width="720"></video>

## Install

Zellij 0.44 or newer is required.

### From a Release

Download the WASM from
[Releases](https://github.com/www159-used/zellij-overview/releases),
rename it, and put it in the plugin directory:

```bash
mkdir -p ~/.config/zellij/plugins
mv overview-v*.wasm ~/.config/zellij/plugins/overview.wasm
```

### From source

`rust-toolchain.toml` pins Rust `1.95.0` and the `wasm32-wasip1` target:

```bash
git clone https://github.com/www159-used/zellij-overview.git
cd zellij-overview
cargo wasm
mkdir -p ~/.config/zellij/plugins
cp target/wasm32-wasip1/release/overview.wasm ~/.config/zellij/plugins/
```

## Keybinding

Add this to `keybinds` in `~/.config/zellij/config.kdl`.
Use a `file:` URL, not a plugin alias, so instance identity stays consistent.

```kdl
shared {
    bind "Ctrl y" {
        LaunchPlugin "file:~/.config/zellij/plugins/overview.wasm" {
            floating true
            skip_plugin_cache true
        }
    }
}
```

On first launch Zellij asks for permission to read and change session state. Allow it. Press `Ctrl+y` again to close overview.

## Usage

The footer shows the common keys. Press `?` inside overview for the full help.

### Select and jump

1. Press `Ctrl+y` to open overview.
2. Move the selection with `h` `j` `k` `l` or the arrow keys.
3. Press `e` or `Enter` to jump to the selected tab, session, or pane.

Movement stops at both ends of the list; it does not wrap. Use `gg` / `G` to jump to the first or last item. In the scrolling list, the camera follows when the cursor leaves the window.

### Flash search

<video src="asset/flash.mov" controls width="720"></video>

Suppose you have tabs named `notes`, `feature/geo-db`, and `logs`:

1. Press `s` to start search. The footer becomes `FLASH`.
2. Type `geo`.
3. Matches highlight and get a tip such as `a`.
4. Press `a` to jump to `feature/geo-db` immediately, without Enter.

`Backspace` deletes the query. `Esc` leaves search and returns to normal mode.

### Three layers: session / tab / pane

`Space` is the layer leader. After you press it, the footer becomes `SPACE  s sessions   t tabs   p panes`.

| Keys | Action |
| --- | --- |
| `Space` `s` | Enter the session layer |
| `Space` `t` | Return to the tab layer |
| `Space` `p` | Enter the pane layer for the current tab |
| `Esc` | Cancel the leader without changing layers |

In normal mode, `s` still starts Flash search. You only enter the session layer after `Space`.

#### Session

<video src="asset/sessions.mov" controls width="720"></video>

1. Press `Space` `s`. The footer shows `SESSIONS`.
2. Pick a target with `hjkl` or Flash `s`, then press `e` or a tip to switch sessions.
3. `q` / `Esc` returns to the tab layer. Press it once more to close overview.

The session layer lists live sessions only. Each card shows that session's tab count. WASM has no filesystem, so there is no previous session across overview launches.

#### Pane

<video src="asset/pane.mov" controls width="720"></video>

1. Press `Space` `p`. The footer shows `PANES`.
2. The list includes selectable, unsuppressed panes in the current tab, and excludes overview itself. Floating panes are marked `float`.
3. Press `e` or a tip to focus that pane. Overview then closes.
4. `q` / `Esc` returns to the tab layer.

### Go to the previous item

<video src="asset/previous.mov" controls width="720"></video>

A card marked `[-]` is the previously focused tab or pane. Press `-` to jump there.

The previous tab comes from Zellij tab history; the previous pane comes from pane history. The session layer has no reliable previous item yet.

### Status marks

- Purple border or `›`: the keyboard selection
- `●`: the tab or session you are actually on
- `[-]`: the tab or pane `-` will return to
- `float`: a floating pane

### Adaptive layout

<video src="asset/scroll.mov" controls width="720"></video>

- When there is room, card width follows the title and wraps
- Cards on the same row may have different widths, but they stay the same height
- When space is tight, borders and padding drop
- When every item cannot stay readable at once, the view becomes a single scrolling column; `↑` / `↓` on the right mean more items are off-screen
- `hjkl` moves the selection; the camera follows; both ends stop
- Resizing the terminal or the floating pane reflows the layout
- Up and down pick the card whose horizontal center is nearest on the adjacent row

### Keys

| Key | Action |
| --- | --- |
| `h` `j` `k` `l` | Move the selection (normal mode) |
| Arrow keys | Move the selection (normal and search modes) |
| `Ctrl+d` / `Ctrl+u` | Half page down / up (normal mode) |
| `Ctrl+f` / `Ctrl+b` | Full page down / up (normal mode) |
| `gg` / `G` | First / last item |
| `zt` / `zz` / `zb` | Align the current item to the top / center / bottom in a scrolling list |
| `PageDown` / `PageUp` | Full page down / up |
| `e` / `Enter` | Jump to the selected tab, session, or pane (normal mode) |
| `Space` | Layer leader; then `s` / `t` / `p` for session / tab / pane |
| `s` | Start Flash search |
| Any character | Query or tip (search mode) |
| `Backspace` | Delete search input (search mode) |
| `Esc` | Cancel search or the leader; from the session or pane layer, return to tabs |
| `-` | Previous tab or pane (normal mode) |
| `q` / `Esc` | Go back a layer or close overview (normal mode) |
| `?` | Toggle full help (normal mode; typed as input in search mode) |
| `Ctrl+y` | Close overview when pressed again |

## Known limits

Zellij shows or hides the floating layer as a whole. Opening overview also reveals other floating panes that were hidden; closing it restores that layer to the state from before overview opened. The plugin API cannot show only one interactive floating pane.

## Develop

```bash
cargo fmt --check
cargo lint
# Host-side core tests; do not use the WASM target
cargo test --lib
cargo wasm
zellij -l zellij.kdl
```

The development layout loads `target/wasm32-wasip1/release/overview.wasm`. The UI is painted through a Ratatui buffer; the Zellij adapter reads tabs, sessions, and panes and performs the jump.

## Release

Install [`cargo-release`](https://github.com/crate-ci/cargo-release), dry-run, then publish:

```bash
cargo install cargo-release
cargo release patch
cargo release patch --execute
```

You can replace `patch` with `minor`, `major`, or an explicit version.

Pushing a `v*` tag makes GitHub Actions verify the project, build the WASM, compute SHA-256, and create a GitHub Release.

## License

This project is licensed under the [GNU Affero General Public License v3.0 only](LICENSE).
