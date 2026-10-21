import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Check,
  ChevronsUpDown,
  FolderCog,
  FolderOpen,
  Plus,
  X,
} from "lucide-react";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import type { AppId } from "@/lib/api/types";
import {
  useApplyProfileMutation,
  useClearProfileMutation,
  useCreateProfileMutation,
  useProfilesQuery,
} from "@/lib/query/profiles";
import { ProfileManageDialog } from "./ProfileManageDialog";
import {
  APP_PROFILE_SCOPE,
  hasScopeSnapshot,
  getScopesWithSnapshot,
} from "./scope";
import type { CurrentProfileIds, ProfileScope } from "@/lib/api/profiles";

const CURRENT_ID_KEY: Record<ProfileScope, keyof CurrentProfileIds> = {
  claude: "claude",
  "claude-desktop": "claudeDesktop",
  codex: "codex",
};

const SCOPE_I18N_SHORT: Record<ProfileScope, string> = {
  claude: "Claude",
  "claude-desktop": "Desktop",
  codex: "Codex",
};

interface ProfileSwitcherProps {
  activeApp: AppId;
}

/**
 * 项目 Profile 切换器（header 左侧入口）
 *
 * 项目列表全应用共享（用户拥有的项目就那几个），但切换按分组进行：
 * Claude 组（Claude Code 的供应商/MCP/Skills/记忆文件 + Claude Desktop
 * 的供应商）与 Codex 组各自指向自己的当前项目、只应用组内快照。
 * 与右侧 AppSwitcher（仅切换查看的应用）语义不同。
 */
export function ProfileSwitcher({ activeApp }: ProfileSwitcherProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [isCreateOpen, setIsCreateOpen] = useState(false);
  const [isManageOpen, setIsManageOpen] = useState(false);
  const [newName, setNewName] = useState("");

  const { data } = useProfilesQuery();
  const applyMutation = useApplyProfileMutation();
  const clearMutation = useClearProfileMutation();
  const createMutation = useCreateProfileMutation();

  // Profile 仅作用于受支持的应用——在其他应用的标签页展示会误导用户
  // 以为当前应用也被切换了，因此只在有所属分组的应用页面渲染
  const scope = APP_PROFILE_SCOPE[activeApp];
  if (!scope) {
    return null;
  }

  const profiles = data?.profiles ?? [];
  const currentId = data?.currentIds?.[CURRENT_ID_KEY[scope]] ?? null;
  const currentProfile = profiles.find((p) => p.id === currentId);

  const handleApply = (id: string) => {
    setOpen(false);
    if (id !== currentId) {
      applyMutation.mutate({ id, scope });
    }
  };

  const closeCreateDialog = () => {
    setIsCreateOpen(false);
    setNewName("");
  };

  const handleCreate = () => {
    const name = newName.trim();
    if (!name) return;
    createMutation.mutate({ name, scope }, { onSuccess: closeCreateDialog });
  };

  return (
    <>
      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger asChild>
          <button
            type="button"
            role="combobox"
            aria-expanded={open}
            title={t(`profiles.switcherTooltip.${scope}`)}
            className={cn(
              "inline-flex h-8 items-center gap-1.5 rounded-lg px-2.5 text-sm font-medium transition-colors",
              "hover:bg-black/5 dark:hover:bg-white/5",
              currentProfile ? "text-foreground" : "text-muted-foreground",
            )}
          >
            <FolderOpen className="h-4 w-4 shrink-0 opacity-70" />
            <span className="max-w-[9rem] truncate">
              {currentProfile?.name ?? t("profiles.none")}
            </span>
            <ChevronsUpDown className="h-3.5 w-3.5 shrink-0 opacity-50" />
          </button>
        </PopoverTrigger>
        <PopoverContent
          side="bottom"
          align="start"
          sideOffset={6}
          className="z-[100] w-72 p-0"
        >
          <Command label={t("profiles.searchPlaceholder")}>
            <CommandInput placeholder={t("profiles.searchPlaceholder")} />
            <CommandList>
              <CommandEmpty>{t("profiles.empty")}</CommandEmpty>
              {profiles.length > 0 && (
                <CommandGroup>
                  {profiles.map((profile) => {
                    const hasSnapshot = hasScopeSnapshot(profile, scope);
                    const capturedScopes = getScopesWithSnapshot(profile);
                    const missingHint = !hasSnapshot
                      ? t("profiles.noSnapshotHint", {
                          defaultValue:
                            "本侧尚未保存，切换时不改动配置，只标记为当前项目",
                        })
                      : undefined;
                    return (
                      <CommandItem
                        key={profile.id}
                        value={profile.id}
                        keywords={[profile.name]}
                        title={missingHint}
                        onSelect={() => handleApply(profile.id)}
                        className="gap-0 py-2"
                      >
                        <Check
                          className={cn(
                            "mr-2 h-4 w-4 shrink-0",
                            currentId === profile.id
                              ? "opacity-100"
                              : "opacity-0",
                          )}
                        />
                        <span className="min-w-0 flex-1 truncate">
                          {profile.name}
                        </span>
                        {!hasSnapshot ? (
                          <span
                            className="ml-2 max-w-[7.5rem] shrink-0 truncate text-right text-xs text-amber-600 dark:text-amber-400"
                            title={missingHint}
                          >
                            {t("profiles.noSnapshotForScope")}
                          </span>
                        ) : capturedScopes.length < 3 ? (
                          <span
                            className="ml-2 shrink-0 text-[10px] text-muted-foreground"
                            title={t("profiles.capturedScopesHint", {
                              defaultValue: "已拍快照的分组",
                              scopes: capturedScopes
                                .map((s) => SCOPE_I18N_SHORT[s])
                                .join(" · "),
                            })}
                          >
                            {capturedScopes
                              .map((s) => SCOPE_I18N_SHORT[s])
                              .join(" · ")}
                          </span>
                        ) : null}
                      </CommandItem>
                    );
                  })}
                </CommandGroup>
              )}
              <div className="mx-1 my-1 h-px bg-border" />
              <CommandGroup>
                <CommandItem
                  value="__create__"
                  keywords={[t("profiles.createFromCurrent")]}
                  onSelect={() => {
                    setOpen(false);
                    setIsCreateOpen(true);
                  }}
                >
                  <Plus className="mr-2 h-4 w-4 shrink-0" />
                  {t("profiles.createFromCurrent")}
                </CommandItem>
                {currentId && (
                  <CommandItem
                    value="__clear__"
                    keywords={[t("profiles.none")]}
                    onSelect={() => {
                      setOpen(false);
                      clearMutation.mutate(scope);
                    }}
                  >
                    <X className="mr-2 h-4 w-4 shrink-0" />
                    {t("profiles.none")}
                  </CommandItem>
                )}
                {profiles.length > 0 && (
                  <CommandItem
                    value="__manage__"
                    keywords={[t("profiles.manage")]}
                    onSelect={() => {
                      setOpen(false);
                      setIsManageOpen(true);
                    }}
                  >
                    <FolderCog className="mr-2 h-4 w-4 shrink-0" />
                    {t("profiles.manage")}
                  </CommandItem>
                )}
              </CommandGroup>
            </CommandList>
          </Command>
        </PopoverContent>
      </Popover>

      <Dialog
        open={isCreateOpen}
        onOpenChange={(open) => {
          if (!open) closeCreateDialog();
        }}
      >
        <DialogContent className="max-w-sm" zIndex="alert">
          <DialogHeader className="space-y-3 border-b-0 bg-transparent pb-0">
            <DialogTitle>{t("profiles.createFromCurrent")}</DialogTitle>
            <DialogDescription>
              <span>{t(`profiles.createDescription.${scope}`)}</span>
              {profiles.length > 0 && (
                <span className="mt-2 block text-xs text-muted-foreground">
                  {t("profiles.createScopeHint", {
                    defaultValue:
                      "仅保存当前分组的配置，其余分组保持原样；已有同名项目将在重命名时提示。",
                  })}
                </span>
              )}
            </DialogDescription>
          </DialogHeader>
          <div className="px-6 pt-3">
            <Input
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              placeholder={t("profiles.namePlaceholder")}
              autoFocus
              onKeyDown={(e) => {
                if (e.key === "Enter") handleCreate();
              }}
            />
            {profiles.some(
              (p) =>
                p.name.trim().toLowerCase() === newName.trim().toLowerCase() &&
                newName.trim(),
            ) && (
              <p className="mt-2 text-xs text-amber-600 dark:text-amber-400">
                {t("profiles.duplicateNameHint", {
                  defaultValue: "已存在同名项目，创建后请注意区分。",
                })}
              </p>
            )}
          </div>
          <DialogFooter className="flex gap-2 border-t-0 bg-transparent pt-2 sm:justify-end">
            <Button variant="outline" onClick={closeCreateDialog}>
              {t("common.cancel")}
            </Button>
            <Button
              onClick={handleCreate}
              disabled={!newName.trim() || createMutation.isPending}
            >
              {t("common.confirm")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <ProfileManageDialog
        isOpen={isManageOpen}
        onClose={() => setIsManageOpen(false)}
      />
    </>
  );
}
