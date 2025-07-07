# GitHub Actions 定时任务和邮件通知配置指南

## 📋 概述

已为您的 Clash Butler 项目配置了每日定时运行的 GitHub Actions 工作流，包含以下功能：

- ⏰ **定时执行**：每天北京时间下午 23:00 自动运行
- 🔧 **手动触发**：支持在 GitHub Actions 页面手动运行
- 📧 **邮件通知**：执行成功或失败都会发送邮件提醒
- 📊 **详细报告**：包含节点数量、执行时间、文件大小等信息
- 📁 **日志保存**：自动上传运行日志，保留 7 天
- 🛠️ **智能下载**：自动获取最新版本的 Clash Meta，支持备用下载方案
- 🔍 **错误诊断**：详细的错误信息和故障排除建议

## 🔧 最新修复

### v2.1 更新内容（2025-07-07）

1. **彻底修复 Clash Meta 问题** ✅
   - 发现项目已自带 `clash-meta/mihomo` 文件（28MB）
   - 移除不必要的下载步骤，直接使用现有文件
   - 简化工作流程，提高执行效率和稳定性

2. **优化工作流程**
   - 移除不必要的系统依赖安装
   - 添加文件存在性检查和权限设置
   - 参考现有 CI 工作流的成功经验

3. **增强错误处理**
   - 详细的错误信息捕获和报告
   - 超时检测和退出码分析
   - 更好的日志输出和调试信息

4. **改进邮件通知**
   - 成功邮件包含配置文件大小、直接订阅链接
   - 失败邮件包含具体错误原因和排查建议
   - 添加执行ID便于追踪问题

## ⚙️ 邮件配置步骤

### 1. 设置 GitHub Secrets

在您的 GitHub 仓库中设置以下 Secrets：

1. 进入仓库页面 → `Settings` → `Secrets and variables` → `Actions`
2. 点击 `New repository secret` 添加以下配置：

#### 必需的 Secrets：

| Secret 名称 | 说明 | 示例值 |
|------------|------|--------|
| `SMTP_SERVER` | SMTP 服务器地址 | `smtp.gmail.com` |
| `SMTP_PORT` | SMTP 端口 | `587` `465`|
| `SMTP_USERNAME` | 发送邮箱用户名 | `your-email@gmail.com` |
| `SMTP_PASSWORD` | 邮箱密码或应用专用密码 | `your-app-password` |
| `EMAIL_TO` | 接收通知的邮箱 | `your-email@gmail.com` |
| `EMAIL_FROM` | 发送方邮箱 | `Clash Butler <your-email@gmail.com>` |

### 2. 常用邮箱服务配置

#### Gmail 配置
```
SMTP_SERVER: smtp.gmail.com
SMTP_PORT: 587
SMTP_USERNAME: your-email@gmail.com
SMTP_PASSWORD: 应用专用密码 (需要开启两步验证)
```

**Gmail 应用专用密码设置**：
1. 开启两步验证
2. 访问 [Google 账户设置](https://myaccount.google.com/security)
3. 生成应用专用密码用于 SMTP

#### QQ 邮箱配置
```
SMTP_SERVER: smtp.qq.com
SMTP_PORT: 587
SMTP_USERNAME: your-email@qq.com
SMTP_PASSWORD: 授权码 (在QQ邮箱设置中生成)
```

#### 163 邮箱配置
```
SMTP_SERVER: smtp.163.com
SMTP_PORT: 587
SMTP_USERNAME: your-email@163.com
SMTP_PASSWORD: 授权码 (在163邮箱设置中生成)
```

#### Outlook 配置
```
SMTP_SERVER: smtp-mail.outlook.com
SMTP_PORT: 587
SMTP_USERNAME: your-email@outlook.com
SMTP_PASSWORD: 邮箱密码
```

### 3. 测试配置

配置完成后，您可以：

1. **手动触发测试**：
   - 进入 `Actions` 页面
   - 选择 `Daily Clash Butler Update` 工作流
   - 点击 `Run workflow` 手动运行

2. **查看执行结果**：
   - 在 Actions 页面查看运行状态
   - 下载日志文件查看详细信息
   - 检查邮箱是否收到通知邮件

## 📧 邮件通知内容

### 成功通知邮件包含：
- ✅ 执行成功状态
- ⏱️ 开始和结束时间
- 📊 可用节点数量
- 🔗 仓库和配置文件链接

### 失败通知邮件包含：
- ❌ 执行失败状态
- ⏱️ 执行时间
- 🔗 查看日志的链接
- 💡 故障排除建议

## 🔧 自定义配置

### 修改执行时间

编辑 `.github/workflows/daily-update.yml` 文件中的 cron 表达式：

```yaml
schedule:
  # 每天北京时间早上 8:00 (UTC 00:00)
  - cron: '0 0 * * *'
  
  # 其他时间示例：
  # 每天北京时间下午 2:00 (UTC 06:00)
  # - cron: '0 6 * * *'
  
  # 每 12 小时运行一次
  # - cron: '0 */12 * * *'
```

### 修改超时时间

默认设置 30 分钟超时，可以在工作流中修改：

```yaml
if timeout 1800 cargo run --release > run_output.log 2>&1; then
# 1800 秒 = 30 分钟，可根据需要调整
```

## 🚀 工作流特性

1. **智能重试**：程序失败时会记录详细日志
2. **资源优化**：使用 Rust 缓存加速构建
3. **安全性**：所有敏感信息通过 GitHub Secrets 管理
4. **可观测性**：完整的执行日志和状态报告
5. **自动提交**：成功时自动更新 `clash.yaml` 文件

## 🔍 故障排除

### 常见问题：

1. **邮件发送失败**
   - 检查 SMTP 配置是否正确
   - 确认邮箱服务商的 SMTP 设置
   - 验证应用专用密码是否有效

2. **程序执行超时**
   - 检查订阅链接是否可访问
   - 考虑增加超时时间
   - 查看详细日志排查问题

3. **节点数量为 0**
   - 检查 `conf/config.toml` 中的订阅链接
   - 确认网络连接正常
   - 查看测试配置是否合理

### 查看日志：

1. 进入 Actions 页面
2. 点击具体的运行记录
3. 展开相应的步骤查看详细日志
4. 下载 `clash-butler-logs` 工件获取完整日志

## 📝 注意事项

- 确保仓库有足够的 Actions 运行时间配额
- 定期检查邮件通知是否正常
- 建议定期更新订阅链接
- 注意保护好邮箱密码等敏感信息

---

配置完成后，您的 Clash Butler 将每天自动运行并通过邮件通知您结果！🎉