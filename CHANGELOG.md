# 更新日志

本文件记录 VibeHub 的版本更新。版本号遵循 `MAJOR.MINOR.PATCH`。

## v3.3.2

- 修复 Agent 配置版本号跨 Tauri IPC 传递时的 JavaScript 64 位整数精度截断 Bug：将 SHA-256 哈希提取位数从 16 位十六进制调整为 12 位（48-bit 安全整数），彻底解决保存配置时持续误报 `configuration changed after it was read; reload before continuing` 的冲突拦截问题。
- 修复 Windows 宿主环境运行时探测因 WSL 不可用时抛出 `wsl.exe` 退出错误导致整个运行环境列表为空的问题，现在在没有 WSL 的 Windows 机器上也能稳定识别原生 Windows 宿主环境。
- 新增 Agent 配置面板「复制启动指令」功能：支持一键复制当前选中 Profile 的终端命令行启动指令（支持 OpenCode、Claude Code、Codex 及其专属 Profile 参数）。

## v3.3.1

- 修复 Windows 上 Workspace State 保存时对临时文件二次打开导致的 `Access denied`：写入与 `fsync` 现在使用同一个打开的文件句柄，并保留 Unix 权限元数据。
- 修复 Windows 上 Agent 声明、有效声明与嵌套 Git 项目投影路径使用 `\` 而非 `/` 的问题。
- Windows CI 测试覆盖补齐：Agent Profiles fixture 按平台生成 OpenCode 配置目录，host MCP launcher 断言改为解析配置值，symlink 测试限定 Unix。
- 修复 Windows 上 `src-tauri` 缺少 `windows-sys` 目标依赖导致 Release 构建失败的问题。

## v3.3.0

### Agent Profiles

- 新增 OpenCode、Claude Code 与 Codex 三类 Agent 的配置管理：Provider、Base URL、Key 引用、模型、默认/小模型与 variants/思考档位。
- Claude Code 以 Profile 抽象管理多份 settings；Codex 管理原生 Profile 与 `--profile` 临时启动；OpenCode 直接管理当前配置并支持临时启动。
- 提供带 revision、备份与回滚的无损读写、Secret 隔离、并发保护与自动 sidecar 生命周期。
- macOS / Windows / WSL 独立配置发现与安全启动边界；未知 Schema 或协议 fail-closed。

### V3 Session–Task 路由与多项目 MCP

- 新增轻量 Session–Task 绑定契约：所有 Task 级写入都要求显式 binding 与 binding revision，session_open 保留兼容的创建即绑定路径。
- `task_candidates` / `task_route` 提供可审计的任务路由决策；不同项目只能读写各自的 project_id，跨项目写入与资源读取返回结构化拒绝。
- MCP 采用单实例单项目、canonical project root 与 fail-closed 身份校验；初始化和迁移会同步受信项目级 MCP 配置，不再把具体项目根写入全局配置。
- 新增多项目隔离冒烟测试，覆盖同名项目、嵌套 Git root、symlink alias 与未受信项目启动拒绝。

### Workspace State

- 持久化项目标签顺序、active project 与 V3 cockpit 的视图/任务/节点上下文。
- 后端使用 revision、原子写入与损坏回退；启动时恢复多项目标签并校验项目身份。

### Guided Intake 与计划视图

- 任务创建可直接承载带 scope、依赖、criterion 覆盖与 role 的初始实现计划。
- 旧 bootstrap 节点不再进入有效计划；计划图、时间线与完成门禁统一使用 effective nodes。
- 新增 initial plan 创建界面、节点 origin/role 契约与对应视图字段。

### 文档与测试

- 三语 README 增加视觉导览，截图迁移到 `assets/readme`。
- V3 契约、MCP、i18n 与标签栏检查覆盖新表面；多项目 MCP 冒烟测试适配 binding 门禁。

### 已知例外

- Windows 原生手测项 `A07`、`E07`、`F07–F10`、`G05` 仍须在发布后使用 v3.3.0 正式 Release 的 Windows artifact 与真实 SHA-256 手测，发布时保持待验收状态。
- Apple / Windows 代码签名为可选增强：凭据缺失时产物为 macOS ad-hoc 签名与 Windows unsigned，不等同于正式签名。

## v3.2.0

### Token 用量统计与费用报告重构

- 新增带版本的模型定价目录，并对 model id 做规范化归一，费用由真实 token 数推导，不再信任 provider 上报的 0 值。
- 引入以文件 mtime 为键的用量缓存与刷新摘要，重复扫描结果稳定可复现。
- 增加 transcript 体积、行数与单条消息 token 三重防护，遇到不合理数据 fail-closed 而不是静默计入总量。
- 重构用量面板：精确/缩放 token 显示、费用来源标注、异常诊断与审计明细下钻。
- 新增 `usage-overview` 视图契约，纳入 V3 契约检查。

### V3 阻塞可解释性与修复引导

- 完成门禁失败改为结构化 blocker：携带 severity、provenance、`missing_facts` 与可执行的 repair action。
- 校验 evidence ref，占位字符串无法清空门禁。
- `progress_evidence` 成为可见的 checklist 项，直接点名仍缺少 progress 的 session。
- session gap 恢复计入投影：已记录 gap recovery 的中断 session 不再被静默判定为缺少 progress。
- release handoff 证据可推导出精确修复动作（Windows/IDE 版本、artifact 与 SHA-256、`criterion_review` 命令），替代通用模板文案。
- 重新生成 node-brief 与 view 契约及 fixtures，blocker 面板与 node brief 面板渲染完整阻塞详情。

### 工作流闭环与 Project Memory

- V3 workflow closure 强制：任务需具备 plan / session / progress / result / review 记录才能进入完成流程。
- 明确 Agent 工具调用时序规范；Project Memory 仅经 typed command 写入，disputed / stale / secret 默认不注入。

### 持续集成

- `Build and Test` 与 `Release` workflow 加入 Rust 构建缓存，缩短流水线时间。

### 清理

- 移除 6 个 scoped-reader 重构遗留的死转发函数。
- 移除 5 个从未被引用的 V3 面板组件，其视图契约保持不变。
- 移除 4 个未被引用的 V2 模块。

### 已知例外

- Windows 原生手测项 `A07`、`E07`、`F07–F10`、`G05` 需在发布后使用本版本正式 Release 的 Windows artifact 与真实 SHA-256 在 Windows 主机完成，发布时保持待验收状态。
- Apple / Windows 代码签名为可选增强：凭据缺失时产物为 macOS ad-hoc 签名与 Windows unsigned，不等同于正式签名。

## v3.1.0

- V3 workflow closure 与 Project Memory 提升为正式能力，并同步全仓版本与发布门禁。

## v3.0.6 及更早

- 历史版本请参阅 GitHub Releases 与 `git log`。
