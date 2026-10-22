import { invoke } from "@tauri-apps/api/core";

export interface ModelAlias {
  id: string;
  appType: string;
  alias: string;
  providerId: string;
  targetModel: string;
  priority: number;
  enabled: boolean;
  createdAt: number;
  updatedAt: number;
}

/** 一个逻辑模型及其在各渠道上的物理名（UI 编辑单元）。 */
export interface ModelAliasGroup {
  alias: string;
  /** 渠道 → 该渠道的物理模型名 */
  entries: Array<{ providerId: string; targetModel: string }>;
}

/** 把扁平的 alias 记录按逻辑模型聚合成 UI 需要的分组结构。 */
export function groupModelAliases(aliases: ModelAlias[]): ModelAliasGroup[] {
  const byAlias = new Map<string, ModelAliasGroup>();
  for (const item of aliases) {
    let group = byAlias.get(item.alias);
    if (!group) {
      group = { alias: item.alias, entries: [] };
      byAlias.set(item.alias, group);
    }
    group.entries.push({
      providerId: item.providerId,
      targetModel: item.targetModel,
    });
  }
  return [...byAlias.values()].sort((a, b) => a.alias.localeCompare(b.alias));
}

export const modelAliasesApi = {
  async list(appType: string): Promise<ModelAlias[]> {
    return invoke("list_model_aliases", { appType });
  },

  /** 整体替换某逻辑模型的渠道映射；空数组等于删除该逻辑模型。 */
  async setEntries(
    appType: string,
    alias: string,
    entries: Array<[string, string]>,
  ): Promise<void> {
    return invoke("set_model_alias_entries", { appType, alias, entries });
  },

  async upsert(
    appType: string,
    alias: string,
    providerId: string,
    targetModel: string,
  ): Promise<ModelAlias> {
    return invoke("upsert_model_alias", {
      appType,
      alias,
      providerId,
      targetModel,
    });
  },

  async remove(appType: string, id: string): Promise<void> {
    return invoke("delete_model_alias", { appType, id });
  },

  async hasAny(appType: string): Promise<boolean> {
    return invoke("has_model_aliases", { appType });
  },
};
