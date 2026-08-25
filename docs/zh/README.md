# zellij-overview

[English](../../README.md) · 中文

Zellij 里用来跳 session、跳 tab 的浮动窗口。

`Alt+y` 打开。上面是钉着的 tab，中间是各个 session，下面是当前 session 里没钉的 tab。在 session 上按 `e`（或 Flash 给的字母）先进它的 tab 列表，再按一次才真的跳过去。当前 session 的 tab 不用钻，直接跳。项多了按 `s` 搜标题。

<img src="../../asset/open.gif" alt="打开 overview" width="720">

## 安装

Zellij 0.44 及以上。

### Release

从 [Releases](https://github.com/www159-used/zellij-overview/releases) 下 WASM，改名放到插件目录：

```bash
mkdir -p ~/.config/zellij/plugins
mv zellij-overview-v*.wasm ~/.config/zellij/plugins/zellij-overview.wasm
```

### 自己编

`rust-toolchain.toml` 锁了 Rust `1.95.0` 和 `wasm32-wasip1`：

```bash
git clone https://github.com/www159-used/zellij-overview.git
cd zellij-overview
./scripts/install.sh
```

编完会拷到 `~/.config/zellij/plugins/zellij-overview.wasm`。要换路径就设 `OVERVIEW_PLUGIN_PATH`。

## 快捷键

写进 `~/.config/zellij/config.kdl` 的 `keybinds`。用 `file:` 路径，别走插件 alias，否则 Zellij 会认成另一个实例。

```kdl
shared {
    bind "Alt y" {
        LaunchPlugin "file:~/.config/zellij/plugins/zellij-overview.wasm" {
            floating true
        }
    }
}
```

日常不要加 `skip_plugin_cache`：开了之后每次打开都会强制重编译 WASM，大约卡一秒。只有开发热更新时才加 `skip_plugin_cache true`，或者改完用 `zellij action start-or-reload-plugin` 重载。

第一次打开会要权限，允许就行。再按一次 `Alt+y` 关掉。

## 怎么跳

底下那一行是常用键。完整说明按 `?`。

### Flash

<img src="../../asset/flash.gif" alt="按 s 搜 log，再按字母跳到 logs" width="720">

`s` 开搜，打标题里的几个字。只剩一个匹配就立刻跳。多个匹配时卡片上会有字母，没对上的沉进遮罩。按那个字母就跳，不用 Enter。比如打 `log`，再按提示的键到 `logs`。`Esc` 退回普通模式。字母落在 session 上，效果和 `e` 一样，先看它的 tab。

### 换 session

<img src="../../asset/drill.gif" alt="先进入别的 session 的 tab 列表，再跳" width="720">

在 session 上按 `e`，overview 还开着，只换成那个 session 的 tab。再在 tab 上按 `e` 才过去。`Esc` 或 `q` 先回到刚打开时那一页。

### 钉

<img src="../../asset/pin.gif" alt="把别的 session 的 tab 钉在最前，一次跳走" width="720">

在 tab 上按 `p`，它会排到最前面。别的 session 的钉后面跟着 session 名，打开就能直接跳，不用再钻一层。

### 刚才那个

<img src="../../asset/previous.gif" alt="按 - 回到上一个 tab" width="720">

带 `[-]` 的就是按 `-` 会去的地方。同一个 session 里是上一个 tab。刚从别的 session 过来时，先按 `-` 进入那个 session 的列表，再按一次才落到上次的 tab。那张 tab 如果已经钉着，按一次 `-` 就够。

## 限制

Zellij 的浮动层是整层显隐的。打开 overview 时，藏着的其它浮窗也会出来；关掉后按打开前的状态收回去。

## 开发

```bash
cargo fmt --check
cargo lint
# 测核心行为，不要加 wasm target
cargo test --lib   # cargo t（含 e2e/scenes）
cargo e2e          # 只跑 host 场景
cargo run --bin overview-tui -- pin logs
cargo run --bin overview-tui -- focus logs jump
cargo run --bin overview-tui -- --replay e2e/scenes/pin-partial-open.scene
cargo run --features tui --bin overview-tui   # cargo tui
./scripts/e2e-zellij.sh
cargo wasm
zellij -l zellij.kdl
```

`Overview::decide`（`Key` 进，`Action` 出）在 `src/tests/`。布局、渲染、配色、用量、浮窗尺寸跟各自模块。

`e2e/scenes/` 是 host 场景：按插件打开的顺序喂事件（先 session 快照，再 `/cache/pins`，再 TabUpdate），期望写在场景里。钉走和按 `p` 同一条路（场景里 `pin logs`，或 `overview-tui pin logs`），不往假盘里塞。`focus logs` / `jump` 是选中和按 `e`，期望写 `focused` / `action`。带 `[-]` 的外 session 钉会藏掉对应 session 卡（`pin-previous-hides-session.scene`）。`overview-tui --replay` 不用 TTY；`--features tui` 是同一块板的本机交互。`./scripts/e2e-zellij.sh` 在一次性 session 里加载 WASM，Zellij 版本一变先在这里露馅。没装 `zellij` 就跳过；要强制失败就设 `OVERVIEW_E2E_ZELLIJ_REQUIRED=1`。

开发布局加载的是 `target/wasm32-wasip1/release/zellij-overview.wasm`。颜色在 `src/theme.css`，编进插件。改色不用重编：拷到插件的 `/cache/theme.css`，重开 overview。关掉时会在 `/cache/usage.jsonl` 记一条本机用量，`./scripts/usage-summary.sh` 可以汇总。

## 许可证

[MIT License](../../LICENSE)。
