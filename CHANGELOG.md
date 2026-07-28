# 更新日志

本文件记录 VibeHub 的版本更新。版本号遵循 `MAJOR.MINOR.PATCH`。

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
