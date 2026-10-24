import { describe, expect, it } from "vitest";
import { groupModelAliases, type ModelAlias } from "@/lib/api/model-aliases";

/** 构造一条 alias 记录；只需被测函数读取的字段，其余给默认值。 */
function aliasOf(
  alias: string,
  providerId: string,
  targetModel: string,
): ModelAlias {
  return {
    id: `${alias}:${providerId}`,
    appType: "claude",
    alias,
    providerId,
    targetModel,
    priority: 0,
    enabled: true,
    createdAt: 0,
    updatedAt: 0,
  };
}

describe("groupModelAliases", () => {
  it("returns an empty list for no records", () => {
    expect(groupModelAliases([])).toEqual([]);
  });

  it("groups flat records by alias and keeps per-alias channel order", () => {
    const groups = groupModelAliases([
      aliasOf("sonnet", "ch_a", "claude-sonnet-4-5"),
      aliasOf("sonnet", "ch_b", "anthropic/claude-sonnet-4.5"),
      aliasOf("sonnet", "ch_c", "sonnet-latest"),
    ]);

    expect(groups).toHaveLength(1);
    expect(groups[0].alias).toBe("sonnet");
    expect(groups[0].entries).toEqual([
      { providerId: "ch_a", targetModel: "claude-sonnet-4-5" },
      { providerId: "ch_b", targetModel: "anthropic/claude-sonnet-4.5" },
      { providerId: "ch_c", targetModel: "sonnet-latest" },
    ]);
  });

  it("separates distinct aliases and sorts groups by alias", () => {
    // 输入故意乱序，验证输出按 alias 升序稳定（UI 展示顺序依赖该排序）
    const groups = groupModelAliases([
      aliasOf("opus", "ch_b", "b-opus"),
      aliasOf("sonnet", "ch_a", "a-sonnet"),
      aliasOf("fast", "ch_c", "c-fast"),
    ]);

    expect(groups.map((group) => group.alias)).toEqual([
      "fast",
      "opus",
      "sonnet",
    ]);
  });

  it("keeps only the fields the UI needs and drops record bookkeeping", () => {
    const [group] = groupModelAliases([aliasOf("sonnet", "ch_a", "s-a")]);
    expect(group.entries[0]).toEqual({
      providerId: "ch_a",
      targetModel: "s-a",
    });
    expect(Object.keys(group.entries[0]).sort()).toEqual([
      "providerId",
      "targetModel",
    ]);
  });
});
