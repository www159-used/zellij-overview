# overview

English · [中文](docs/zh/README.md)

Browse and switch Zellij sessions and tabs from a floating window.

Press `Ctrl+y` to open, move with `hjkl`, and jump with `e`. Sessions sit at the front of the same board as the current session's tabs. When there are many items, press `s` to search a title fragment, then press the tip next to a match.

The layout switches between framed cards, compact cards, and a scrolling column based on title length and the floating pane size.

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
./scripts/install.sh
```

This builds the WASM and copies it to `~/.config/zellij/plugins/overview.wasm`. Override the destination with `OVERVIEW_PLUGIN_PATH`.

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
3. Press `e` or `Enter` to jump to the selected session or tab.

Movement stops at both ends of the list; it does not wrap. Use `gg` / `G` to jump to the first or last item. In the scrolling list, the camera follows when the cursor leaves the window.

### Flash search

<video src="asset/flash.mov" controls width="720"></video>

Suppose you have tabs named `notes`, `feature/geo-db`, and `logs`:

1. Press `s` to start search. The footer becomes `FLASH`.
2. Type `geo`.
3. Matches highlight and get a tip such as `a`.
4. Press `a` to jump to `feature/geo-db` immediately, without Enter.

`Backspace` deletes the query. `Esc` leaves search and returns to normal mode.

### Sessions and tabs on one board

Live sessions are pinned at the front of the same board as the current session's tabs. There is no layer leader and no pane list.

- Session cards start with `◆` and show that session's tab count. Their frames use a cold border when there is room.
- Tab cards keep the existing titles and marks.
- `e` or a Flash tip on a session switches to it; on a tab, it jumps to that tab.
- The footer shows the current session name.

When the pane is wide enough, everything is framed cards that wrap. When it is too narrow, the board becomes one scrolling column: sessions stay marked `◆`, tabs are indented.

### Go to the previous tab

<video src="asset/previous.mov" controls width="720"></video>

A card marked `[-]` is where `-` will return. After a tab switch it is the previous tab; after a session jump it is the session you left (Zellij clears that session's tab history when you leave).

### Status marks

- Purple border or `›`: the keyboard selection
- `●`: the session or tab you are actually on
- `◆`: a session card
- `[-]`: the tab or session `-` will return to

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
| `e` / `Enter` | Jump to the selected session or tab (normal mode) |
| `s` | Start Flash search |
| Any character | Query or tip (search mode) |
| `Backspace` | Delete search input (search mode) |
| `Esc` | Cancel search, or close overview (normal mode) |
| `-` | Previous tab, or the session you jumped from (normal mode) |
| `q` / `Esc` | Close overview (normal mode) |
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
