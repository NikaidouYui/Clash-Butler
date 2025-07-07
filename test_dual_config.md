# 双配置文件功能测试说明

## 修改内容总结

### 1. 代码修改 (src/main.rs)
- 无论 `fast_mode` 设置如何，都会生成快速模式配置文件 `clash-fast.yaml`
- 当 `fast_mode = false` 时，还会额外生成完整处理的 `clash.yaml`
- 当 `fast_mode = true` 时，只生成快速模式配置文件

### 2. GitHub Actions 修改 (.github/workflows/daily-update.yml)
- 统计两个配置文件的节点数量
- 提交两个配置文件到仓库
- 邮件通知包含两个配置文件的详细信息

### 3. README 更新
- 添加了双配置文件模式的说明
- 解释了两种配置文件的区别和使用场景

## 预期效果

### 快速模式配置 (clash-fast.yaml)
- 包含所有通过连通性测试的节点
- 保留香港、台湾等地区节点
- 节点名称保持原始名称
- 适合需要更多节点选择的场景

### 完整处理配置 (clash.yaml)
- 只包含通过 OpenAI/Claude 测试的节点
- 节点按地理位置重命名（如：US_Phoenix_Oracle_OpenAI）
- 节点质量更高但数量可能较少
- 适合需要高质量 AI 服务的场景

## 测试建议

1. 运行程序后检查是否生成了两个配置文件
2. 对比两个文件的节点数量差异
3. 验证快速模式配置是否包含更多香港节点
4. 检查完整处理配置的节点命名是否正确

## 配置文件访问链接

- 快速模式：`https://github.com/用户名/仓库名/raw/master/clash-fast.yaml`
- 完整处理：`https://github.com/用户名/仓库名/raw/master/clash.yaml`