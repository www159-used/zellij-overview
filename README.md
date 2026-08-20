# overview

Zellij 按需浮动 tab 总览：用 Flash.nvim 风格的 hint 直接跳转。

每个 tab 卡片只显示真实标题和外框，不展示 pane 布局。UI 使用 Ratatui Buffer 渲染，
由轻量适配器输出到 Zellij，不接管终端。

## 构建

项目通过 `rust-toolchain.toml` 固定 Rust `1.95.0`，并自动安装 `clippy`、
`rustfmt` 和 `wasm32-wasip1` target。

```bash
cargo wasm
# 产物：target/wasm32-wasip1/release/overview.wasm
cp target/wasm32-wasip1/release/overview.wasm ~/.config/zellij/plugins/overview.wasm
```

本机单测（只跑核心库，不要用 wasm target）：

```bash
cargo test --lib
```

完整本地检查：

```bash
cargo fmt --check
cargo lint
cargo test --lib
cargo wasm
```

## 发布

安装 [`cargo-release`](https://github.com/crate-ci/cargo-release)：

```bash
cargo install cargo-release
```

先预演，再执行版本升级、release commit、`v<version>` tag 和 push：

```bash
cargo release patch
cargo release patch --execute
```

也可将 `patch` 换成 `minor`、`major` 或明确版本号。tag 推送后，
GitHub Actions 会再次运行格式、Clippy 和测试，构建
`overview-v<version>.wasm`，生成 SHA-256，并创建 GitHub Release。

## 绑定

直接使用 `file:` URL，不要 alias；开发时打开插件缓存跳过，确保新 WASM 生效。

```kdl
keybinds {
    shared {
        bind "Ctrl y" {
            LaunchPlugin "file:~/.config/zellij/plugins/overview.wasm" {
                floating true
                skip_plugin_cache true
            }
        }
    }
}
```

打开后：

- `s`：进入 Flash 搜索
- 输入标题片段：高亮匹配字符，并在匹配位置显示 tip
- 输入 tip 字符：立即跳转
- `hjkl` / 方向键：移动高亮
- `-`：跳到上一次聚焦的 tab
- `e` / `Enter`：跳到高亮 tab
- `q` / `Esc`：取消 hint；普通模式下关闭
- 再按 `Ctrl+y`：关闭 overview

## 开发

```bash
cargo wasm
zellij -l zellij.kdl
```

开发布局会直接从 `target/wasm32-wasip1/release/overview.wasm` 启动插件。

> Zellij 会整体显示或隐藏 floating layer。启动 overview 时，原本隐藏的其他
> floating pane 也会暂时出现；关闭 overview 会重新隐藏该图层。

## 许可证

本项目采用 [GNU Affero General Public License v3.0 only](LICENSE)。
