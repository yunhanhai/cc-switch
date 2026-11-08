# YunSwitch 独立发布准备

首个独立预发布源码版本为 `3.20.1-yunswitch.1`。仓库改名保留原历史，两边默认分支为 `main`。本次验证与产物记录见[预发布说明](releases/3.20.1-yunswitch.1.md)。

## 已落实

- [x] 源码版本统一，YunSwitch 名称、应用 ID、深链、独立安装 UpgradeCode 与 Flatpak ID 已配置。
- [x] release / debug 分别使用 `.yunswitch` / `.yunswitch-dev`，新库端口独立，原有端口保留。
- [x] 自启动登记与 Claude Desktop profile 分配独立名称和 ID。
- [x] 四语品牌与本人仓库入口一致，保留 MIT 原始来源，继承促销文案不冒称本项目合作关系。
- [x] [旧配置迁移指南](MIGRATION-YUNSWITCH.md)已编写，免安装模式的真实数据位置已说明。
- [x] 自动更新关闭，继承公钥移除；R2 镜像默认关闭并要求本人显式配置。
- [x] 发布流程校验独立预发布标签与源码版本，全部构建成功后才创建 GitHub 预发布；Apple 签名、公证及 updater 签名需要明确启用。

## 仍需完成

- [ ] 将本次代码提交同步到两边 PR，完成本次跨平台 CI。
- [ ] 在隔离 Windows 环境验证首次安装、实际启动、卸载和旧数据库导入；本机构建及 MSI 元数据检查不能代替交互验收。
- [ ] 对实际发布的 macOS、Linux / ARM64 包分别记录构建与安装检查，不能以 Windows 结果代替。
- [ ] 如需自动更新，配置 YunSwitch 自有公钥、私钥和地址，并验证签名与更新链路。
- [ ] 如需可信平台签名，为 Windows 配置 Authenticode、macOS 配置 Developer ID 与公证。当前没有此类验证声明。
- [ ] 验收后创建对应 Git 标签与公开预发布，发布说明引用实际完成的验证。

CI 固定 Node 与 Rust 工具链；Windows 大测试目标使用 32 MB 编译线程栈，后端串行编译。验证结果以对应提交的运行记录为准。
