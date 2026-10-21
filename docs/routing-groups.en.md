 # Routing Groups
 
 Smart grouping for the unified gateway. Enabled groups per `app` are ordered by `sort_order / created_at / id`, members by `sort_index / added_at / provider_id`, and concatenated as the effective route `P1 -> Pn`. One-provider-one-group per `app` prevents double billing and health drift.
 
 > Index: [routing-groups.index.md](./routing-groups.index.md) | zh [routing-groups.md](./routing-groups.md) | ja [routing-groups.ja.md](./routing-groups.ja.md) | zh-TW [routing-groups.zh-TW.md](./routing-groups.zh-TW.md)
 
 ## Data Model
 
 - `routing_groups(id, app_type, name, sort_order, enabled, created_at, updated_at)`: multiple groups per `app`; `id` like `rg_<uuid>`, default `rg_default_<app>`.
 - `routing_group_members(group_id, provider_id, app_type, sort_index, added_at)`: `PRIMARY KEY(group_id, provider_id)`, exclusive ownership — a provider belongs to at most one group within the same `app`.
 - Compat: `providers.in_failover_queue` mirrors the enabled-group aggregation during the transition for the legacy queue and tray views.
 
 ## Critical Paths
 
 - **Create / Rename**: `name.trim()` non-empty and <= 64; case-insensitive dedup per `app` (validated in frontend and backend).
 - **Overwrite**: `set_routing_group_members` validates existence and ownership conflicts, then `DELETE + INSERT` and bumps `updated_at`.
 - **Drag sort**: `reorder_routing_groups(appType, orderedIds)` rewrites `sort_order = index`; `reorder_routing_group_members(groupId, orderedProviderIds)` rewrites `sort_index = index`. Both require the full set without duplicates; non-member or cross-`app` ids fail.
 - **Enabled**: `update(enabled) / delete / setMembers / addMember / removeMember / reorderMembers` all call `sync_in_failover_queue_from_enabled_locked` under the same lock to write the enabled `provider_id` set back to `providers.in_failover_queue` (clears when empty).
 - **Double write**: `add_to_failover_queue / remove_from_failover_queue` maintain `rg_default_<app>` with `detach_from_other_groups` for ownership migration.
 - **Effective route**: `get_enabled_routing_group_provider_ids` concatenates enabled groups with a single `JOIN`; `ProviderRouter::select_providers` prefers it and falls back to `get_failover_queue`.
 
 ## Frontend
 
 - **Panel**: `RoutingGroupsPanel` per `app`; create/rename (empty/duplicate checks), enable switch, delete, multi-select save; `DndContext(closestCenter)` + `PointerSensor(distance 6)` + `KeyboardSensor`, `SortableContext(verticalListSortingStrategy)`, `SortableGroupRow` / `SortableMemberBadge`, optimistic update + rollback + `invalidate`. `ProxyPanel` forwards `P1..Pn` length via `onEffectiveRouteSizeChange` to `ResilienceSummaryBar` so the same `app` does not query `routingGroups` twice.
 - **Effective route preview**: enabled groups rendered row by row, `P1` solid highlight (both group header and pill); empty groups show `border-dashed` + `Add members` CTA; copy as text/JSON; badges for `429 waiting (pulse) / rate-limit cooldown (countdown) / circuit (red / half-open amber)` with `title` details.
 - **Resilience linkage**: `ResilienceSummaryBar` injects `effectiveRouteSize` into the `Cooling down` hint (`{{routeSize}} in effective route`), reusing the parent value or fetching as fallback; `Waiting` is scoped to `appType`.
 - **A11y**: `DndContext.accessibility.announcements` covers `onDragStart / onDragOver / onDragCancel / onDragEnd`; handles have `aria-label / title` with `defaultValue` fallback; collapsed state, counts and `Switch` are visible.
 
 ## Interfaces
 
 - Tauri: `list_routing_groups / create_routing_group / update_routing_group / delete_routing_group / get_routing_group / set_routing_group_members / add_routing_group_member / remove_routing_group_member / reorder_routing_groups / reorder_routing_group_members`.
 - Proxy: `proxyStatus.rate_limit_waiting` (`appType / providerId / waitSecs / attempt`) and `rateLimitCooldowns` polled every 5s; `ProviderHealth(circuit_state / last_error)` drives badges.
 
 ## i18n
 
 - `coolingDownHintWithRoute` and `addMembers / renameEmpty / renameDuplicate / healthWaiting / healthCooldown / healthCircuitOpen / dragGroupHandle / dragGroupHint / dragMemberHint / dragStartHint / dragEndHint / dragCancelHint` are aligned across `zh / en / ja / zh-TW`.
 
 ## Runtime & Compat
 
 - `SCHEMA_VERSION = 19`, `migrate_v18_to_v19` imports legacy `in_failover_queue = 1` into `rg_default_<app>` per `app`.
 - Transitional double-read/double-write: data plane prefers groups, falls back to legacy queue on miss/empty; writes keep both mirrors to avoid tray/panel drift.
 - Port isolation: dev `15722`, `CARGO_TARGET_DIR` in temp dir to avoid polluting user data.
 
