# 小说阅读与书源管理端口/接口整理

本文档基于当前主项目代码整理，重点覆盖「小说阅读」和「书源管理」相关的网络端口、传输方式，以及前端实际依赖的后端命令接口。

> 注意：当前主项目的 `src-tauri/` 目录缺少 `Cargo.toml`、`tauri.conf.json` 和 Rust 源码，因此这里整理的是前端已经写好的后端依赖清单。真正运行这些能力时，需要 Tauri/Rust 后端或兼容的 WebSocket 后端提供对应命令。

## 1. 网络端口号

| 端口 | 协议/地址 | 用途 | 配置来源 | 是否可配置 |
| --- | --- | --- | --- | --- |
| `1420` | `http://0.0.0.0:1420` | Vite 前端开发服务器端口，用于 `pnpm run dev` / Tauri 开发模式加载前端页面。 | `vite.config.ts` 的 `server.port` | 代码固定，未暴露 UI 配置 |
| `1421` | WebSocket HMR | 设置 `TAURI_DEV_HOST` 时使用的 Vite HMR 端口。 | `vite.config.ts` 的 `server.hmr.port` | 代码固定，仅在 `TAURI_DEV_HOST` 存在时生效 |
| `7688` | `ws://{host}:7688/ws` | 浏览器模式下连接后端的默认 WebSocket 端口；所有 `booksource_*`、`bookshelf_*` 等命令会通过该 WS 通道发送。 | `useTransport.ts`、`useAppConfig.ts`、设置页高级配置 | 可通过高级设置修改 `web_server_port`，也可在 URL 参数 `?ws=` 指定完整 WS 地址 |
| `7688` | `http://{host}:7688` | B/S 模式 Web 服务器默认 HTTP 端口，开启后浏览器可直接访问前端，同时通过 `/ws` 连接后端。 | `web_server_port` 默认值 | 可通过高级设置修改 |
| `7688` | `http://{host}:7688/asset/` | 浏览器模式下本地文件资源访问基地址，例如封面、缓存资源等。 | `useFileSrc.ts` | 跟随 Web 服务端口逻辑，默认写死为 `7688` |
| `8080` | `http://{debugHost}:8080/target.js` | Chii 远程调试 target 脚本默认端口。开启远程调试后前端会注入该脚本。 | `web_remote_debug_port` 默认值 | 可在开发设置里修改 |

## 2. 传输模式

前端对后端的调用统一通过 `invokeWithTimeout` / `transportInvoke` 发起，实际传输方式由运行环境决定。

| 运行环境 | 传输方式 | 是否占用网络端口 | 说明 |
| --- | --- | --- | --- |
| Tauri 桌面端 | Tauri IPC `@tauri-apps/api/core.invoke` | 不占用业务端口 | 直接调用 Rust command，不走 `7688`。 |
| Harmony 原生壳 | `window.__legadoNative` 桥接 | 不占用业务端口 | 语义模拟 Tauri invoke/listen。 |
| 普通浏览器 / Web 模式 | WebSocket | 默认 `ws://{hostname}:7688/ws` | 后端需实现 WS 协议，接收 `{ type: "invoke", id, cmd, args }` 并返回 `{ type: "response", id, data/error }`。 |

WebSocket 自定义地址规则：

- 默认探测：`ws://{当前页面 hostname}:7688/ws`
- HTTPS 页面下默认使用：`wss://{hostname}:7688/ws`
- 可在设置页填写完整地址，例如 `ws://127.0.0.1:7688/ws`
- 也可通过 URL 参数指定：`?ws=ws://127.0.0.1:7688/ws`

## 3. 小说阅读相关后端命令接口

这些不是网络端口号，而是通过 Tauri IPC / Harmony 桥接 / WebSocket 发送给后端的命令名。小说阅读流程主要由书源命令和书架缓存命令共同组成。

### 3.1 书源解析命令

| 命令 | 主要用途 | 典型调用场景 |
| --- | --- | --- |
| `booksource_search` | 调用书源 `search()`，按关键词搜索小说。 | 搜索页、AI 测试、换源。 |
| `booksource_book_info` | 调用书源 `bookInfo()`，获取书籍详情、封面、简介、目录入口等。 | 搜索结果详情、加入书架前详情页。 |
| `booksource_chapter_list` | 调用书源 `toc()`，获取章节目录。 | 打开书籍、刷新目录、加入书架后缓存目录。 |
| `booksource_chapter_content` | 调用书源 `content()`，获取章节正文。 | 阅读器打开章节、预缓存、导出。 |
| `booksource_purchase_chapter` | 调用可选的 VIP 章节购买函数。 | 阅读 VIP 章节前用户确认购买。 |
| `booksource_call_fn` | 调用书源里的自定义函数。 | 段评数量、段评详情、点赞、回复、自定义 HTML 交互等。 |
| `booksource_explore` | 调用书源 `explore()`，获取发现页分类和推荐书籍。 | 发现页、书架推荐。 |
| `booksource_cancel` | 取消正在执行的书源任务。 | 切换章节、取消预缓存、关闭阅读任务。 |

### 3.2 书架与阅读缓存命令

| 命令 | 主要用途 | 典型调用场景 |
| --- | --- | --- |
| `bookshelf_list` | 获取书架列表。 | 书架页初始化、刷新。 |
| `bookshelf_add` | 添加书籍到书架。 | 从搜索/发现结果加入书架、本地 TXT 导入。 |
| `bookshelf_remove` | 从书架移除书籍。 | 书架删除。 |
| `bookshelf_get` | 获取单本书详情。 | 打开阅读器、插件获取书籍信息。 |
| `bookshelf_update_book` | 更新书籍元信息，可同时替换章节目录。 | 编辑书籍信息、整本换源、刷新目录。 |
| `bookshelf_update_progress` | 保存阅读进度。 | 阅读器章节索引、分页页码、滚动比例、视频播放秒数。 |
| `bookshelf_set_private` | 标记/取消隐私书籍。 | 书架隐私模式。 |
| `bookshelf_save_chapters` | 保存章节目录缓存。 | 首次打开或刷新目录后。 |
| `bookshelf_get_chapters` | 读取章节目录缓存。 | 打开书架中已有书籍。 |
| `bookshelf_save_content` | 缓存单章正文。 | 阅读后缓存、预缓存。 |
| `bookshelf_get_content` | 读取单章正文缓存。 | 离线/快速打开已缓存章节。 |
| `bookshelf_delete_content` | 删除单章正文缓存。 | 清理缓存或刷新章节。 |
| `bookshelf_get_cached_indices` | 获取已缓存正文的章节索引。 | 阅读器缓存状态、导出前检查。 |
| `bookshelf_prefetch_chapters` | 后台预缓存章节正文。 | 阅读时预加载后续章节、导出前强制缓存。 |
| `bookshelf_restore_source_switch` | 恢复最近一次整本换源。 | 换源失败或用户撤销换源。 |
| `bookshelf_save_txt_chapters` | 批量保存本地 TXT 拆分后的章节正文。 | 本地 TXT 导入。 |

### 3.3 小说阅读辅助命令

| 命令 | 主要用途 | 备注 |
| --- | --- | --- |
| `config_read` / `config_write` | 书源脚本配置读写。 | 字符串配置。 |
| `config_read_json` / `config_write_json` | 书源脚本 JSON 配置读写。 | 避免二次 JSON 编码。 |
| `config_read_bytes` / `config_write_bytes` | 字节数组配置读写。 | 适合二进制/图片类配置。 |
| `config_delete_key` / `config_clear` / `config_read_all` | 配置清理与导出。 | 按 scope 隔离。 |
| `browser_probe_*` | 浏览器探测、运行 JS、读写 Cookie。 | 某些复杂站点书源可能依赖。 |
| `booksource_http_proxy` | 由后端代发 HTTP 请求或代理资源。 | 发现页 HTML 渲染、跨域资源。 |
| `cover_cache_size` / `cover_cache_clear` | 封面缓存统计与清理。 | 设置页存储管理。 |

## 4. 书源管理相关后端命令接口

书源管理页主要负责书源目录、书源文件 CRUD、在线仓库、调试、测试、更新检测等。

### 4.1 书源目录管理

| 命令 | 主要用途 |
| --- | --- |
| `booksource_get_dir` | 获取默认书源目录绝对路径。 |
| `booksource_get_dirs` | 获取所有书源目录，包括内置目录和外部目录。 |
| `booksource_add_dir` | 添加外部书源目录。 |
| `booksource_remove_dir` | 移除外部书源目录。 |
| `booksource_pick_dir` | 打开系统目录选择器。 |
| `open_dir_in_explorer` | 在系统文件管理器中打开目录。 |

### 4.2 书源文件 CRUD

| 命令 | 主要用途 |
| --- | --- |
| `booksource_list` | 一次性列出所有已安装书源。 |
| `booksource_list_streaming` | 分批列出书源，结果通过 `booksource:batch` 事件推送。 |
| `booksource_read` | 读取单个书源 JS 内容。 |
| `booksource_save` | 保存/覆盖书源 JS 文件。 |
| `booksource_delete` | 删除单个书源文件。 |
| `booksource_delete_batch` | 批量删除书源文件。 |
| `booksource_toggle` | 修改书源启用/禁用状态。 |
| `booksource_resolve_path` | 解析书源文件真实路径。 |
| `booksource_open_in_vscode` | 用 VS Code 打开书源文件。 |

### 4.3 导入与在线仓库

| 命令 | 主要用途 |
| --- | --- |
| `booksource_import_legacy_json_text` | 将开源阅读/Legado Android JSON 书源文本转换并安装。 |
| `booksource_import_legacy_json_url` | 从 URL 下载开源阅读 JSON 书源，转换并安装。 |
| `repository_fetch` | 拉取在线书源仓库 JSON 清单。 |
| `repository_install` | 从仓库下载并安装书源 JS 文件。 |
| `repository_preview_source` | 安装前下载远程书源并解析元数据。 |
| `repository_check_source_sync` | 比较在线仓库书源与本地同 UUID 书源是否一致。 |

### 4.4 调试、测试与更新

| 命令 | 主要用途 |
| --- | --- |
| `booksource_eval` | 在指定书源上下文中执行调试代码。 |
| `js_eval` | 直接执行任意 JS 代码，用于调试 Boa/JS 环境。 |
| `script_repl_eval` | 脚本 REPL 调试。 |
| `booksource_run_tests` | 运行书源内置测试流程。 |
| `booksource_check_update` | 根据书源 `@updateUrl` 检测更新。 |
| `booksource_apply_update` | 从 `@updateUrl` 拉取新内容并覆盖本地文件。 |
| `booksource_save_draft` | AI 书源生成/修改时保存草稿。 |
| `booksource_delete_draft` | 删除 AI 书源草稿。 |

### 4.5 书源事件

| 事件名 | 来源/用途 |
| --- | --- |
| `booksource:batch` | `booksource_list_streaming` 的分批结果事件。 |
| `booksource:changed` | 单个书源发生变化后通知发现页、搜索页、日志等刷新缓存。 |
| `app:booksource-reload` | 前端主动广播书源重载请求，可按 `scope` 或 `fileName` 局部刷新。 |

## 5. Web/B/S 模式服务命令

这些命令和网络端口最直接相关，用于启动或配置 `7688` 端口上的 Web 服务。

| 命令 | 主要用途 |
| --- | --- |
| `web_server_status` | 查询 Web 服务是否运行及当前端口。 |
| `web_server_start` | 启动 Web 服务，返回实际监听端口。 |
| `web_server_stop` | 停止 Web 服务。 |
| `web_server_pick_dist_dir` | 选择外部前端构建产物目录。 |
| `get_local_ips` | 获取局域网 IP，用于展示 `http://IP:端口`。 |
| `app_config_get_all` | 读取完整应用配置，包括 `web_server_port`、远程调试端口等。 |
| `app_config_set` | 保存应用配置。 |
| `app_config_reset` | 重置单个应用配置项。 |

## 6. 后端实现提醒

如果要补齐当前主项目后端，至少需要实现两类能力：

1. Tauri 模式：在 Rust 侧注册上述 command，让 `@tauri-apps/api/core.invoke` 可直接调用。
2. Web 模式：在 `web_server_port` 指定端口，默认 `7688`，提供 HTTP 前端资源服务和 `/ws` WebSocket 服务。

WebSocket 命令协议参考：

```json
{
  "type": "invoke",
  "id": "uuid",
  "cmd": "booksource_list",
  "args": {}
}
```

响应示例：

```json
{
  "type": "response",
  "id": "uuid",
  "data": []
}
```

事件推送示例：

```json
{
  "type": "event",
  "event": "booksource:changed",
  "payload": {
    "fileName": "example.js",
    "reason": "save"
  }
}
```

## 7. 当前主项目缺口

当前主项目存在前端调用清单，但主仓库后端缺失：

- `src-tauri/tauri.conf.json` 不存在。
- `src-tauri/Cargo.toml` 不存在。
- `src-tauri/src/*.rs` 不存在。

因此，`1420` 的前端开发服务器可以启动界面，但小说阅读和书源管理的核心能力需要 `7688` WebSocket 后端或完整 Tauri 后端支撑。
