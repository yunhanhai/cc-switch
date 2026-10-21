import { invoke } from "@tauri-apps/api/core";

export interface RoutingGroup {
  id: string;
  appType: string;
  name: string;
  sortOrder: number;
  enabled: boolean;
  createdAt: number;
  updatedAt: number;
}

export interface RoutingGroupMember {
  groupId: string;
  providerId: string;
  appType: string;
  sortIndex: number;
  addedAt: number;
}

export interface RoutingGroupWithMembers {
  group: RoutingGroup;
  members: RoutingGroupMember[];
}

export const routingGroupsApi = {
  async list(appType: string): Promise<RoutingGroupWithMembers[]> {
    return invoke("list_routing_groups", { appType });
  },
  async create(appType: string, name: string): Promise<RoutingGroup> {
    return invoke("create_routing_group", { appType, name });
  },
  async update(
    id: string,
    patch: { name?: string; enabled?: boolean },
  ): Promise<RoutingGroup> {
    return invoke("update_routing_group", {
      id,
      name: patch.name,
      enabled: patch.enabled,
    });
  },
  async remove(id: string): Promise<void> {
    return invoke("delete_routing_group", { id });
  },
  async get(id: string): Promise<RoutingGroupWithMembers | null> {
    return invoke("get_routing_group", { id });
  },
  async setMembers(groupId: string, providerIds: string[]): Promise<void> {
    return invoke("set_routing_group_members", { groupId, providerIds });
  },
  async addMember(groupId: string, providerId: string): Promise<void> {
    return invoke("add_routing_group_member", { groupId, providerId });
  },
  async removeMember(groupId: string, providerId: string): Promise<void> {
    return invoke("remove_routing_group_member", { groupId, providerId });
  },
};
