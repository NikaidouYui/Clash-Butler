## Clash Butler

现在 Clash 配置文件如日中天，各种节点都有 Clash 配置文件格式，不过 Clash
对于用户界面的开发迭代并没有很快。

想之前用得最舒服的一个电脑端的代理软件还得是 [V2rayN](https://github.com/2dust/v2rayN)
，支持节点测速，测延迟，删除导出，自动排序等等（指节点管理这一块）。

作为一个「忠实的白嫖节点的人」，Clash 节点不允许做删除和新增，只能添加额外的配置，在大佬发新的节点会导致配置列表就会巨长，管理成本变高。

并且分享的节点基本是日抛类型，很快就会失效，不过一个订阅中个别链接又是可用的，
此时就急需一个工具来测速合并多个配置文件，且为了更好和 Clash 客户端配合，生成的链接需要固定的，似乎没有这方面的工具，不如咱就写一个吧？！

![design.png](docs/design.png)

> [!IMPORTANT]
> 作为 Rust 初学者，这个项目一定会被做成好玩的模样，期待一起讨论一起学习 🎉

<p align="center">
  <img alt="vscode" src="https://img.shields.io/badge/Visual%20Studio%20Code-0078d7.svg?style=flat-square&logo=visual-studio-code&logoColor=white" >
  <img alt="Rust" src="https://img.shields.io/badge/Rust 2021-%23000000.svg?style=flat-square&logo=rust&logoColor=white" >
  <img alt="MacOS" src="https://img.shields.io/badge/Sequoia%2015.0-000000?style=flat-square&logo=macos&logoColor=F0F0F0" />
</p>

## 快速开始

### 使用 GitHub Actions（推荐）

#### 基础使用
1. Fork 当前项目。
2. 在自己项目中点击 Actions ，同意并打开 GitHub Actions 功能。
3. 在自己项目中 [`conf/config.toml`](conf/config.toml:1) 中填写需要合并的链接，提交 commit 之后会自动触发构建。
4. 等待 Actions 结束，项目会生成两个配置文件：
   - **`clash-fast.yaml`** - 快速模式配置（仅测试连通性，保留更多节点包括香港节点）
   - **`clash.yaml`** - 完整处理配置（经过 OpenAI/Claude 测试和节点重命名）

#### 🆕 双配置文件模式
为了解决香港等地区节点被过度过滤的问题，现在程序会同时生成两种配置文件：

**快速模式配置 (`clash-fast.yaml`)**：
- ✅ 仅测试基础连通性（Google 204 测试）
- ✅ 保留更多可用节点，包括香港、台湾等地区节点
- ✅ 适合需要更多节点选择的用户
- ⚡ 处理速度更快

**完整处理配置 (`clash.yaml`)**：
- 🔍 经过 OpenAI/Claude 可用性测试
- 🏷️ 节点按地理位置和 ISP 重命名
- 🚀 **新增带宽测速功能**：节点名称包含实际速度信息（如 `HK_1.5MB`）
- 🎯 节点质量更高，但数量可能较少
- 📊 包含详细的节点信息标注

**使用建议**：
- 如果需要访问特定地区（如香港）的服务，推荐使用 `clash-fast.yaml`
- 如果需要高质量的 AI 服务节点，推荐使用 `clash.yaml`

#### 🆕 定时自动更新 + 邮件通知
项目现已支持每日定时自动更新节点配置，并通过邮件通知执行结果：

- ⏰ **每天自动运行**：北京时间早上 8:00 自动执行节点测速和筛选
- 📧 **邮件通知**：执行成功或失败都会发送详细的邮件报告
- 🔧 **手动触发**：支持在 GitHub Actions 页面随时手动运行
- 📊 **详细统计**：包含可用节点数量、执行时间等信息

**配置步骤**：
1. 按照 [GitHub Actions 配置指南](GITHUB_ACTIONS_SETUP.md) 设置邮件通知
2. 在仓库 Settings → Secrets 中添加邮箱 SMTP 配置
3. 完成后即可享受全自动的节点更新服务

> [!TIP]
> 详细配置说明请查看：[**GITHUB_ACTIONS_SETUP.md**](GITHUB_ACTIONS_SETUP.md)

## 🚀 新功能：带宽测速

现在支持对节点进行实际带宽测速，并将速度信息添加到节点名称中！

### 配置说明

在 [`conf/config.toml`](conf/config.toml) 中配置测速参数：

```toml
# 带宽测速配置
[speed_test]
enabled = true                                              # 是否启用测速功能
url = "https://speed.cloudflare.com/__down?bytes=10485760"  # 测速文件URL（10MB）
timeout = 10000                                             # 测速超时时间（毫秒）
```

### 功能特点

- 🎯 **智能测速**：在IP详情获取完成后自动进行带宽测速
- 📊 **速度标注**：节点名称自动包含实际速度（如 `HK_Hong Kong_1.5MB_OpenAI`）
- 🔧 **灵活配置**：可自定义测速文件大小和超时时间
- 📈 **多单位显示**：自动选择合适的单位（KB/MB/GB）

### 节点命名格式

启用测速后，节点名称格式为：
```
${COUNTRYCODE}_${CITY}_${ISP}_${SPEED}_${SERVICES}
```

示例：
- `HK_Hong Kong_HKT_2.5MB_OpenAI_Claude`
- `US_Los Angeles_Cloudflare_15.2MB_OpenAI`
- `JP_Tokyo_NTT_850KB_Claude`

### 本地构建

> [!WARNING]
> 精力有限，目前仅支持 MacOS 使用

1. 修改 config.toml，加入自己订阅地址
    ```yaml
   # 待测速的订阅节点
   # 支持网络地址 https://xxx
   # 支持本地地址（绝对地址）/User/xxx/xx.yml
   # 支持单个订阅链接，ss://xxx
   subs = [
      "https://xxx",
      "/User/xxx/xx.yml",
      "ss://xxx",
   ]
   ```

2. (可选) 关闭 clash tun 模式或全局模式
3. 使用 `cargo run` 启动，即可自动开始节点测速过滤

预计先写 CLI 批量跑完现有节点筛选节点的功能，再考虑后续写成 Web 部署自动化形式