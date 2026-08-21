# overview

[English](../../README.md) · 中文

在浮动窗口里查看并切换 Zellij 的 session、tab 和 pane。

按 `Ctrl+y` 打开，用 `hjkl` 移动，按 `e` 跳转。项多时按 `s` 搜索标题，再按候选项旁的 tip 即可直接跳走。`Space` 是层 leader：再按 `s` 进 session，按 `t` 回 tab，按 `p` 进当前 tab 的 pane。

界面会按标题长度和浮窗尺寸，在有框卡片、无框紧凑和单列滚动之间切换。

<video src="../../asset/open.mov" controls width="720"></video>

## 安装

需要 Zellij 0.44 或更高版本。

### 使用 Release

从 [Releases](https://github.com/www159-used/zellij-overview/releases) 下载 WASM，重命名后放到插件目录：

```bash
mkdir -p ~/.config/zellij/plugins
mv overview-v*.wasm ~/.config/zellij/plugins/overview.wasm
```

### 从源码构建

仓库通过 `rust-toolchain.toml` 固定 Rust `1.95.0` 和 `wasm32-wasip1` target：

```bash
git clone https://github.com/www159-used/zellij-overview.git
cd zellij-overview
cargo wasm
mkdir -p ~/.config/zellij/plugins
cp target/wasm32-wasip1/release/overview.wasm ~/.config/zellij/plugins/
```

## 配置快捷键

把下面的配置加入 `~/.config/zellij/config.kdl` 的 `keybinds`。
直接使用 `file:` URL，不要通过插件 alias 启动，以保证实例识别一致。

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

第一次打开时，Zellij 会询问读取和更改会话状态的权限，选择允许即可。再按一次 `Ctrl+y` 会关闭 overview。

## 使用说明

底部只列出常用键。按 `?` 可在 overview 里看完整帮助。

### 选择并跳转

1. 按 `Ctrl+y` 打开 overview。
2. 用 `h` `j` `k` `l` 或方向键移动选中框。
3. 按 `e` 或 `Enter` 跳到选中的 tab、session 或 pane。

撞到列表两端会停住，不会绕回另一头。到头或到尾用 `gg` / `G`。滚动列表里，光标出窗时镜头会跟上。

### Flash 搜索

<video src="../../asset/flash.mov" controls width="720"></video>

假设有 `notes`、`feature/geo-db` 和 `logs` 三个 tab：

1. 按 `s` 进入搜索，footer 变成 `FLASH`。
2. 输入 `geo`。
3. 匹配项会高亮，并带上类似 `a` 的 tip。
4. 按 `a` 立刻跳到 `feature/geo-db`，不用再按 Enter。

`Backspace` 删查询。`Esc` 退出搜索，回到普通模式。

### 三层：session / tab / pane

`Space` 是层 leader。按下后 footer 变成 `SPACE  s sessions   t tabs   p panes`。

| 按键 | 作用 |
| --- | --- |
| `Space` `s` | 进入 session 层 |
| `Space` `t` | 回到 tab 层 |
| `Space` `p` | 进入当前 tab 的 pane 层 |
| `Esc` | 取消这次 leader，不换层 |

普通模式下的 `s` 仍是 Flash 搜索，只有先按 `Space` 才进 session。

#### Session

<video src="../../asset/sessions.mov" controls width="720"></video>

1. `Space` `s`，footer 显示 `SESSIONS`。
2. 用 `hjkl` 或 Flash `s` 选目标，再按 `e` 或 tip 切换 session。
3. `q` / `Esc` 回到 tab 层；再按一次才关闭 overview。

session 层只列出当前活着的 session，卡片右侧显示该 session 的 tab 数。WASM 没有文件系统，session 没有跨次打开的「上一项」。

#### Pane

<video src="../../asset/pane.mov" controls width="720"></video>

1. `Space` `p`，footer 显示 `PANES`。
2. 只列出当前 tab 里可选、未抑制的 pane，并排除 overview 自己。浮动 pane 标 `float`。
3. 按 `e` 或 tip 聚焦该 pane，overview 随后关闭。
4. `q` / `Esc` 回到 tab 层。

### 返回上一项

<video src="../../asset/previous.mov" controls width="720"></video>

标有 `[-]` 的卡片是上一次聚焦的 tab 或 pane。按 `-` 直接返回。

tab 的上一项来自 Zellij 的 tab 历史，pane 的上一项来自 pane 历史。session 层目前没有可靠的上一项。

### 状态标记

- 紫色边框或 `›`：键盘当前选中的项
- `●`：当前实际所在的 tab 或 session
- `[-]`：按 `-` 将返回的 tab 或 pane
- `float`：浮动 pane

### 自适应布局

<video src="../../asset/scroll.mov" controls width="720"></video>

- 空间充足时，卡片宽度跟随标题并自动换行
- 同一行可以有不同宽度，但卡片等高
- 空间较小时去掉边框和 padding
- 无法同时看清全部项时，改成单列滚动；右侧 `↑` / `↓` 表示还有未显示的项
- `hjkl` 移动选中项，镜头跟着走，两端停住
- 调整终端或浮窗尺寸后，布局会重新换行
- 上下移动会选邻行里横向位置最近的卡片

### 快捷键

| 按键 | 操作 |
| --- | --- |
| `h` `j` `k` `l` | 移动选中项（普通模式） |
| 方向键 | 移动选中项（普通模式和搜索模式） |
| `Ctrl+d` / `Ctrl+u` | 向下 / 向上半页（普通模式） |
| `Ctrl+f` / `Ctrl+b` | 向下 / 向上一页（普通模式） |
| `gg` / `G` | 跳到第一项 / 最后一项 |
| `zt` / `zz` / `zb` | 滚动列表里把当前项对齐顶部 / 居中 / 底部 |
| `PageDown` / `PageUp` | 向下 / 向上一页 |
| `e` / `Enter` | 跳到选中的 tab、session 或 pane（普通模式） |
| `Space` | 层 leader；再按 `s` / `t` / `p` 进 session / tab / pane |
| `s` | 进入 Flash 搜索 |
| 任意字符 | 输入查询或 tip（搜索模式） |
| `Backspace` | 删除搜索输入（搜索模式） |
| `Esc` | 取消搜索或 leader；在 session / pane 层则回到 tab 层 |
| `-` | 返回上一个 tab 或 pane（普通模式） |
| `q` / `Esc` | 回到上一层或关闭 overview（普通模式） |
| `?` | 打开或关闭完整帮助（普通模式；搜索模式下作为输入） |
| `Ctrl+y` | 再次按下时关闭 overview |

## 已知限制

Zellij 会整体显示或隐藏 floating layer。打开 overview 时，原本隐藏的其他 floating pane 也会暂时出现；关闭 overview 后，该图层会按打开前的状态恢复。目前插件 API 无法只显示一个可交互的 floating pane。

## 开发

```bash
cargo fmt --check
cargo lint
# 本机核心测试，不要使用 WASM target
cargo test --lib
cargo wasm
zellij -l zellij.kdl
```

开发布局从 `target/wasm32-wasip1/release/overview.wasm` 启动插件。界面由 Ratatui Buffer 渲染，Zellij 适配层负责读 tab / session / pane 并执行跳转。

## 发布

安装 [`cargo-release`](https://github.com/crate-ci/cargo-release)，先预演再发布：

```bash
cargo install cargo-release
cargo release patch
cargo release patch --execute
```

也可将 `patch` 换成 `minor`、`major` 或明确版本号。

推送 `v*` tag 后，GitHub Actions 会验证项目、构建 WASM、生成 SHA-256 并创建 GitHub Release。

## 许可证

本项目采用 [GNU Affero General Public License v3.0 only](../../LICENSE)。
