 # ルーティンググループ（Routing Groups）
 
 統一ゲートウェイの smart グループ抽象。`app` ごとの有効なグループは `sort_order / created_at / id`、メンバーは `sort_index / added_at / provider_id` でソートし、有効ルート `P1 -> Pn` として連結します。同一 `app` 内では 1 プロバイダー 1 グループの帰属制で、二重課金とヘルス分散を防止します。
 
 > 索引: [routing-groups.index.md](./routing-groups.index.md) | zh [routing-groups.md](./routing-groups.md) | en [routing-groups.en.md](./routing-groups.en.md) | zh-TW [routing-groups.zh-TW.md](./routing-groups.zh-TW.md)
 
 ## データモデル
 
 - `routing_groups(id, app_type, name, sort_order, enabled, created_at, updated_at)`：`app` ごとに複数グループ、`id` は `rg_<uuid>`、デフォルトは `rg_default_<app>`。
 - `routing_group_members(group_id, provider_id, app_type, sort_index, added_at)`：`PRIMARY KEY(group_id, provider_id)`、同一 `app` 内では排他的帰属。
 - 互換：`providers.in_failover_queue` は移行期に有効グループの集約をミラーし、旧キューやトレイ表示に利用します。
 
 ## 主要フロー
 
 - **作成/リネーム**：`name.trim()` が非空かつ 64 文字以内、同一 `app` 内では大文字小文字を区別せず重複排除（フロントとバックの両方で検証）。
 - **上書き**：`set_routing_group_members` は存在と帰属競合を検証してから `DELETE + INSERT` し `updated_at` を更新。
 - **ドラッグ並べ替え**：`reorder_routing_groups(appType, orderedIds)` は `sort_order = index`、`reorder_routing_group_members(groupId, orderedProviderIds)` は `sort_index = index` を書き換え。いずれも全量かつ重複なしが必須で、非メンバーや `app` 跨ぎはエラー。
 - **有効/無効**：`update(enabled) / delete / setMembers / addMember / removeMember / reorderMembers` は同一ロック内で `sync_in_failover_queue_from_enabled_locked` を呼び出し、有効な `provider_id` 集合を `providers.in_failover_queue` に書き戻します（空ならクリア）。
 - **二重書き込み**：`add_to_failover_queue / remove_from_failover_queue` は `rg_default_<app>` を維持し、`detach_from_other_groups` で帰属を移行します。
 - **有効ルート**：`get_enabled_routing_group_provider_ids` は 1 回の `JOIN` で有効グループを連結、`ProviderRouter::select_providers` はそれを優先し、なければ `get_failover_queue` にフォールバックします。
 
 ## フロントエンド
 
 - **パネル**：`RoutingGroupsPanel` は `app` ごと。作成/リネーム（空/重複チェック）、有効スイッチ、削除、複数選択保存。`DndContext(closestCenter)` + `PointerSensor(distance 6)` + `KeyboardSensor`、`SortableContext(verticalListSortingStrategy)`、`SortableGroupRow` / `SortableMemberBadge`、楽観的更新 + ロールバック + `invalidate`。`ProxyPanel` は `onEffectiveRouteSizeChange` で `P1..Pn` の長さを上位の `ResilienceSummaryBar` に渡すため、同一 `app` で `routingGroups` を二重に取得しません。
 - **有効ルートプレビュー**：有効なグループを順に描画し、`P1` はヘッダーとピルの両方で強調。空グループは `border-dashed` + `メンバーを追加` の導線、テキスト/JSON コピーに対応。`429 待機（パルス）/ レート制限（カウントダウン）/ サーキット（赤/半開は琥珀）` のバッジと `title` 詳細を重ねます。
 - **レジリエンス連携**：`ResilienceSummaryBar` の `クールダウン中` は `effectiveRouteSize`（`{{routeSize}} in effective route`）を注入し、親から渡されていれば再利用、なければ自前で取得。`待機中` は `appType` に紐づきます。
 - **アクセシビリティ**：`DndContext.accessibility.announcements` は `onDragStart / onDragOver / onDragCancel / onDragEnd` を網羅し、ハンドルは `aria-label / title` と `defaultValue` フォールバックを持ちます。
 
 ## インターフェース
 
 - Tauri：`list_routing_groups / create_routing_group / update_routing_group / delete_routing_group / get_routing_group / set_routing_group_members / add_routing_group_member / remove_routing_group_member / reorder_routing_groups / reorder_routing_group_members`。
 - プロキシ：`proxyStatus.rate_limit_waiting`（`appType / providerId / waitSecs / attempt`）と `rateLimitCooldowns`（5 秒ポーリング）、`ProviderHealth(circuit_state / last_error)` がバッジを駆動します。
 
 ## 多言語
 
 - `coolingDownHintWithRoute` と `addMembers / renameEmpty / renameDuplicate / healthWaiting / healthCooldown / healthCircuitOpen / dragGroupHandle / dragGroupHint / dragMemberHint / dragStartHint / dragEndHint / dragCancelHint` は `zh / en / ja / zh-TW` で整合済みです。
 
 ## 実行と互換
 
 - `SCHEMA_VERSION = 19`、`migrate_v18_to_v19` で旧 `in_failover_queue = 1` を `app` ごとに `rg_default_<app>` へインポート。
 - 移行期は二重読み書き：データ面はグループを優先し、失敗や空なら旧キューにフォールバック、書き込みは両方を維持してトレイとパネルの乖離を防ぎます。
 - ポート分離：開発版 `15722`、`CARGO_TARGET_DIR` は一時ディレクトリでユーザーデータを汚しません。
 
