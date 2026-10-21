 # Routing Groups — Index
 
 路由分组文档导航 / Routing Groups Documentation Index / ルーティンググループ索引
 
 - **中文（主）**：[docs/routing-groups.md](./routing-groups.md) —— 统一网关 smart 分组、全量语义与前后端联动
 - **English**：[docs/routing-groups.en.md](./routing-groups.en.md)
 - **日本語**：[docs/routing-groups.ja.md](./routing-groups.ja.md)
 - **繁中**：[docs/routing-groups.zh-TW.md](./routing-groups.zh-TW.md)
 
 ## 四语对照（摘要）
 
 | 主题 | zh | en | ja | zh-TW |
 | --- | --- | --- | --- | --- |
 | smart 分组抽象 | 定制版统一网关，每 `app` 的启用分组按顺序拼接 `P1 -> Pn`，归属制防重计费 | Smart grouping per `app`, concatenated `P1 -> Pn`, exclusive ownership | `app` ごとの有効グループを連結し帰属制で二重課金を防止 | 每 `app` 依序拼接 `P1 -> Pn`，歸屬制防重計費 |
 | 关键路径 | 创建/重命名/覆盖/拖拽/启用/双写/生效路由单 JOIN | Create/rename/overwrite/drag/enabled/double-write/effective JOIN | 作成/リネーム/上書き/ドラッグ/有効/二重書き込み/有効ルート | 建立/重新命名/覆寫/拖曳/啟用/雙寫/生效路由 |
 | 前端 | 每 `app` 面板 + 生效预览 + 韧性条 + 无障碍 | Per-app panel + preview + resilience + a11y | `app` ごとパネル + プレビュー + レジリエンス + アクセシビリティ | 每 `app` 面板 + 預覽 + 韌性條 + 無障礙 |
 | 兼容 | `SCHEMA_VERSION 19` 迁移与双读双写 | `SCHEMA_VERSION 19` migration & double R/W | `SCHEMA_VERSION 19` 移行と二重読み書き | `SCHEMA_VERSION 19` 遷移與雙讀雙寫 |
 
 ## 维护
 
 - 新增键时需四语 `coolingDownHintWithRoute / addMembers / rename* / health* / drag*` 同步。
 - 单测约定：混合/幂等/并发三类覆盖 `sync_in_failover_queue` 与 `rg_default_<app>` 双写。
 
