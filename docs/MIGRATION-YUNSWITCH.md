# 从 CC Switch / 旧开发版迁移到 YunSwitch

YunSwitch 延续现有数据库格式，但采用独立应用身份，不自动寻找或接管 `~/.cc-switch`、`~/.cc-switch-dev`。发布构建使用 `~/.yunswitch`，`pnpm dev` 使用 `~/.yunswitch-dev`。Windows 上 `~` 通常是 `%USERPROFILE%`。

## 推荐步骤

1. 在旧应用中关闭所有代理接管，再退出旧应用；如需停用旧版自启动，在旧应用中操作。确认 CLI 不再指向已停止的旧代理。
2. 使用旧应用的「导出数据库」保存 SQL 备份，同时备份原应用配置目录和实际 CLI 配置。备份可能含 API 密钥，保存到自己的私有位置，不提交仓库。
3. 启动 YunSwitch，在「设置 → 数据管理」导入 SQL 备份。新目录中已经有数据时，先导出新数据再导入。导入行为以界面的确认提示为准。
4. 核对供应商、模型别名、路由组及端口，再逐个开启所需接管。导入数据库保留旧端口；可在设置中改为 YunSwitch 的 `15731`（开发版 `15732`），并核对 CLI 最终配置。
5. Claude Desktop 使用新的 YunSwitch profile。先用旧应用恢复官方配置并重启 Claude Desktop，再从 YunSwitch 启用所需路由；新应用不会主动删除旧 profile。

两款应用可以安装共存，但不要同时接管同一个 CLI 配置。应用数据隔离不等于真实 CLI 配置隔离；发布构建会按用户选择管理实际 CLI 文件。

## 无法从旧应用导出时

完全退出旧应用后，备份整个原目录，再把备份中的 `cc-switch.db` 放入一个新的 YunSwitch 配置目录。数据库存在 `-wal` / `-shm` 文件时应先确保数据库已经正常关闭；不要仅复制正在写入的主数据库。

不要把 YunSwitch 的应用目录直接设置为旧应用正在使用的目录。保留原目录作为回退副本，先用副本验证迁移。`settings.json` 中的 CLI 目录等设备设置可在新应用里重新填写；SQL 导入不保证转移所有设备设置、Skills 文件或 Tauri Store 的目录覆盖。

## 开发与免安装包

`corepack pnpm dev` 自动加载开发身份覆盖。直接执行 `tauri dev` 时也要加 `--config src-tauri/tauri.dev.conf.json`，保证注册深链和目录一致。调试构建同样需要该覆盖配置。

免安装 ZIP 中的 `portable.ini` 仅表示免安装模式，应用数据仍在 `~/.yunswitch`，不会自动写入 ZIP 解压目录。自动更新当前关闭，可从本项目 Releases 手动获取后续版本。
