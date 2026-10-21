 # 路由分组（Routing Groups）

定制版统一网关的 smart 分组抽象：每 `app` 的已启用分组按 `sort_order / created_at / id` 排序，组内成员按 `sort_index / added_at / provider_id` 排序，拼接为数据面的真实生效路由 `P1 -> Pn`。多分组归属制避免同 `app` 下重复计费与健康漂移。

 > EN: Smart grouping for the unified gateway — enabled groups per `app` are ordered by `sort_order / created_at / id`; members by `sort_index / added_at / provider_id`; concatenated as the effective route `P1 -> Pn`. One-provider-one-group per app prevents double billing and health drift.  
 > JA: 統一ゲートウェイの smart グループ抽象 — 有効なグループは `sort_order / created_at / id`、メンバーは `sort_index / added_at / provider_id` で整列し、有効ルート `P1 -> Pn` として連結。同一 `app` 内では 1 プロバイダー 1 グループで二重課金とヘルス分散を防止。  
 > ZH-TW: 統一閘道的 smart 分組抽象：每 `app` 已啟用分組依 `sort_order / created_at / id` 排序，成員依 `sort_index / added_at / provider_id` 排序，拼接為生效路由 `P1 -> Pn`。同 `app` 單供應商僅歸屬一組，避免重複計費與健康漂移。
 
## 数据模型

- `routing_groups(id, app_type, name, sort_order, enabled, created_at, updated_at)`：每 `app` 可多组；`id` 形如 `rg_<uuid>`，默认分组 `rg_default_<app>`。
- `routing_group_members(group_id, provider_id, app_type, sort_index, added_at)`：`PRIMARY KEY(group_id, provider_id)`，同 `app` 下归属制——同一 `provider` 仅能归属一组。
- 兼容：`providers.in_failover_queue` 在过渡期作为已启用分组聚合的镜像，供旧队列与托盘视图使用。

## 关键路径

- **创建/重命名**：`name.trim()` 非空、长度 <= 64；同 `app` 下大小写不敏感排重，前端与后端双校验 / EN: case-insensitive per-app dedup / JA: 同一 `app` 内で大文字小文字を区別せず重複排除 / ZH-TW: 同 `app` 內大小寫不敏感排重。
- **覆盖写入**：`set_routing_group_members` 先校验存在性与归属冲突，再 `DELETE + INSERT` 并更新 `updated_at`。
- **拖拽排序**：`reorder_routing_groups(appType, orderedIds)` 重写 `sort_order = index`；`reorder_routing_group_members(groupId, orderedProviderIds)` 重写 `sort_index = index`，均要求全量且不重复，非成员或跨 `app` 报错。
- **启用态**：`update(enabled)` / `delete` / `setMembers / addMember / removeMember / reorderMembers` 均在同一锁内调用 `sync_in_failover_queue_from_enabled_locked`，将已启用分组的 `provider_id` 集合写回 `providers.in_failover_queue`（无启用分组时清空）。
- **双写**：`add_to_failover_queue / remove_from_failover_queue` 同步维护 `rg_default_<app>`，`detach_from_other_groups` 保证归属搬运。
- **生效路由**：`get_enabled_routing_group_provider_ids` 单 `JOIN` 拼接已启用分组；`ProviderRouter::select_providers` 优先走该结果，空则回退 `get_failover_queue`。

## 前端体验

- **面板**：`RoutingGroupsPanel` 每 `app` 独立；支持创建/重命名（空/重名校验）/启用开关/删除/成员多选保存；`DndContext(closestCenter)` + `PointerSensor(distance 6)` + `KeyboardSensor`，`SortableContext(verticalListSortingStrategy)`，`SortableGroupRow` 与 `SortableMemberBadge`，乐观更新 + 回滚 + `invalidate`。ProxyPanel 通过 `onEffectiveRouteSizeChange` 将 `P1..Pn` 长度回传给上方的 `ResilienceSummaryBar`，同 `app` 不再各查一次 `routingGroups`。
- **生效路由预览**：已启用分组按顺序分行渲染，`P1` 实心高亮（分组标题与胶囊双重），空分组以 `border-dashed` + `添加成员` 一键引导，支持复制文本/JSON；叠加 `429 等待（脉冲）/ 限流冷却（倒计时）/ 熔断（红/半开黄）` 徽标，`title` 带详细原因。
- **韧性条联动**：`ResilienceSummaryBar` 的 `冷却中` 提示注入 `effectiveRouteSize`（`{{routeSize}} in effective route`），已由父容器直连传入时复用，未传入时兜底自拉；`等待中` 与当前 `appType` 绑定，避免跨 `app` 误显示。
- **无障碍**：`DndContext.accessibility.announcements` 补全 `onDragStart / onDragOver / onDragCancel / onDragEnd`，手柄 `aria-label / title` 与 `defaultValue` 兜底。`SortableGroupRow` 折叠/计数与 `Switch` 启用态同步可见。

## 接口

- Tauri：`list_routing_groups / create_routing_group / update_routing_group / delete_routing_group / get_routing_group / set_routing_group_members / add_routing_group_member / remove_routing_group_member / reorder_routing_groups / reorder_routing_group_members`。
- 代理：`proxyStatus.rate_limit_waiting`（`appType / providerId / waitSecs / attempt`）与 `rateLimitCooldowns` 5s 轮询；`ProviderHealth(circuit_state / last_error)` 驱动徽标。

## 多语言

- 韧性条 `coolingDownHintWithRoute` / 分组 `addMembers / renameEmpty / renameDuplicate / healthWaiting / healthCooldown / healthCircuitOpen / dragGroupHandle / dragGroupHint / dragMemberHint / dragStartHint / dragEndHint / dragCancelHint` 已在 `zh / en / ja / zh-TW` 对齐。

## 运行与兼容

- `SCHEMA_VERSION = 19`，`migrate_v18_to_v19` 将旧 `in_failover_queue = 1` 按 `app` 导入 `rg_default_<app>`。
 - 切换期保持双读双写：数据面优先分组，读取失败或空时回退旧队列；写面同时维护镜像，避免托盘与面板割裂。
 - 端口隔离：开发版 `15722`，`CARGO_TARGET_DIR` 指向临时目录，避免污染用户数据。
 
