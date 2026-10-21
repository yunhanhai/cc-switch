 # 路由分組（Routing Groups）
 
 統一閘道的 smart 分組抽象：每 `app` 的已啟用分組依 `sort_order / created_at / id` 排序，成員依 `sort_index / added_at / provider_id` 排序，拼接為生效路由 `P1 -> Pn`。同 `app` 單供應商僅歸屬一組，避免重複計費與健康漂移。
 
 > 索引：[routing-groups.index.md](./routing-groups.index.md) | zh [routing-groups.md](./routing-groups.md) | en [routing-groups.en.md](./routing-groups.en.md) | ja [routing-groups.ja.md](./routing-groups.ja.md)
 
 ## 資料模型
 
 - `routing_groups(id, app_type, name, sort_order, enabled, created_at, updated_at)`：每 `app` 可多組；`id` 形如 `rg_<uuid>`，預設 `rg_default_<app>`。
 - `routing_group_members(group_id, provider_id, app_type, sort_index, added_at)`：`PRIMARY KEY(group_id, provider_id)`，同 `app` 歸屬制——同一供應商僅能歸屬一組。
 - 相容：`providers.in_failover_queue` 於過渡期作為已啟用分組聚合的鏡像，供舊佇列與托盤視圖使用。
 
 ## 關鍵路徑
 
 - **建立/重新命名**：`name.trim()` 非空且長度 <= 64；同 `app` 內大小寫不敏感排重，前後端雙校驗。
 - **覆寫寫入**：`set_routing_group_members` 先校驗存在性與歸屬衝突，再 `DELETE + INSERT` 並更新 `updated_at`。
 - **拖曳排序**：`reorder_routing_groups(appType, orderedIds)` 重寫 `sort_order = index`；`reorder_routing_group_members(groupId, orderedProviderIds)` 重寫 `sort_index = index`，皆要求全量且不重複，非成員或跨 `app` 報錯。
 - **啟用態**：`update(enabled) / delete / setMembers / addMember / removeMember / reorderMembers` 皆在同一鎖內呼叫 `sync_in_failover_queue_from_enabled_locked`，將已啟用分組的 `provider_id` 集合寫回 `providers.in_failover_queue`（無啟用分組時清空）。
 - **雙寫**：`add_to_failover_queue / remove_from_failover_queue` 同步維護 `rg_default_<app>`，`detach_from_other_groups` 保證歸屬搬運。
 - **生效路由**：`get_enabled_routing_group_provider_ids` 以單 `JOIN` 拼接已啟用分組；`ProviderRouter::select_providers` 優先採用，空時回退 `get_failover_queue`。
 
 ## 前端體驗
 
 - **面板**：`RoutingGroupsPanel` 每 `app` 獨立；支援建立/重新命名（空/重名校驗）/啟用開關/刪除/成員多選保存；`DndContext(closestCenter)` + `PointerSensor(distance 6)` + `KeyboardSensor`，`SortableContext(verticalListSortingStrategy)`，`SortableGroupRow` 與 `SortableMemberBadge`，樂觀更新 + 回滾 + `invalidate`。`ProxyPanel` 透過 `onEffectiveRouteSizeChange` 將 `P1..Pn` 長度回傳給上方的 `ResilienceSummaryBar`，同 `app` 不再各查一次 `routingGroups`。
 - **生效路由預覽**：已啟用分組依序分行渲染，`P1` 實心高亮（分組標題與膠囊雙重），空分組以 `border-dashed` + `新增成員` 一鍵引導，支援複製文字/JSON；疊加 `429 等待（脈衝）/ 限流冷卻（倒數）/ 熔斷（紅/半開黃）` 徽標，`title` 帶詳細原因。
 - **韌性條連動**：`ResilienceSummaryBar` 的 `冷卻中` 提示注入 `effectiveRouteSize`（`{{routeSize}} in effective route`），已由父容器直連傳入時複用，未傳入時兜底自拉；`等待中` 與當前 `appType` 綁定，避免跨 `app` 誤顯示。
 - **無障礙**：`DndContext.accessibility.announcements` 補全 `onDragStart / onDragOver / onDragCancel / onDragEnd`，手柄 `aria-label / title` 與 `defaultValue` 兜底。
 
 ## 介面
 
 - Tauri：`list_routing_groups / create_routing_group / update_routing_group / delete_routing_group / get_routing_group / set_routing_group_members / add_routing_group_member / remove_routing_group_member / reorder_routing_groups / reorder_routing_group_members`。
 - 代理：`proxyStatus.rate_limit_waiting`（`appType / providerId / waitSecs / attempt`）與 `rateLimitCooldowns` 5s 輪詢；`ProviderHealth(circuit_state / last_error)` 驅動徽標。
 
 ## 多語言
 
 - 韌性條 `coolingDownHintWithRoute` / 分組 `addMembers / renameEmpty / renameDuplicate / healthWaiting / healthCooldown / healthCircuitOpen / dragGroupHandle / dragGroupHint / dragMemberHint / dragStartHint / dragEndHint / dragCancelHint` 已於 `zh / en / ja / zh-TW` 對齊。
 
 ## 執行與相容
 
 - `SCHEMA_VERSION = 19`，`migrate_v18_to_v19` 將舊 `in_failover_queue = 1` 按 `app` 匯入 `rg_default_<app>`。
 - 切換期保持雙讀雙寫：資料面優先分組，讀取失敗或空時回退舊佇列；寫面同時維護鏡像，避免托盤與面板割裂。
 - 連接埠隔離：開發版 `15722`，`CARGO_TARGET_DIR` 指向暫存目錄，避免污染使用者資料。
 
