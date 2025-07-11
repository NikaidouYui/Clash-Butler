# 带宽测速功能说明

## 功能概述

Clash Butler 现已支持对节点进行实际带宽测速，并将速度信息自动添加到节点名称中。

## 工作流程

1. **连通性测试** - 首先测试节点的基本连通性
2. **IP详情获取** - 获取节点的真实IP地址和地理位置信息
3. **服务可用性检查** - 测试 OpenAI 和 Claude 等服务的可用性
4. **🆕 带宽测速** - 对节点进行实际下载速度测试
5. **节点重命名** - 将所有信息整合到节点名称中

## 配置参数

```toml
[speed_test]
enabled = true                                              # 是否启用测速功能
url = "https://speed.cloudflare.com/__down?bytes=10485760"  # 测速文件URL（10MB）
timeout = 10000                                             # 测速超时时间（毫秒）
min_speed_mbps = 2.0                                        # 最小速度阈值（MB/s），低于此速度的节点将被过滤
```

### 参数说明

- `enabled`: 控制是否启用测速功能
- `url`: 测速下载文件的URL，建议使用 Cloudflare 的测速服务
- `timeout`: 测速超时时间，单位为毫秒
- `min_speed_mbps`: 最小速度阈值，单位为MB/s，低于此速度的节点将被自动过滤

### 推荐配置

- **快速测试**: `bytes=1048576` (1MB), `timeout=5000` (5秒)
- **标准测试**: `bytes=10485760` (10MB), `timeout=10000` (10秒)
- **详细测试**: `bytes=104857600` (100MB), `timeout=30000` (30秒)

## 节点命名格式

启用测速后，节点名称将包含速度信息：

```
${COUNTRYCODE}_${CITY}_${ISP}_${SPEED}_${SERVICES}
```

### 示例

- `HK_Hong Kong_HKT_2.5MB_OpenAI_Claude` - 香港节点，2.5MB/s，支持OpenAI和Claude
- `US_Los Angeles_Cloudflare_15.2MB_OpenAI` - 美国节点，15.2MB/s，仅支持OpenAI
- `JP_Tokyo_NTT_850KB_Claude` - 日本节点，850KB/s，仅支持Claude
- `SG_Singapore_AWS_0MB` - 新加坡节点，测速失败

## 速度单位

程序会自动选择合适的速度单位：

- **KB/s**: 小于 1MB/s 的速度
- **MB/s**: 1MB/s 到 1GB/s 之间的速度  
- **GB/s**: 大于 1GB/s 的速度

## 注意事项

1. **测速会增加处理时间** - 每个节点的测速大约需要额外的 5-30 秒
2. **网络环境影响** - 测速结果受本地网络环境影响
3. **服务器负载** - 建议使用稳定的测速服务器
4. **超时设置** - 根据网络环境合理设置超时时间
5. **🆕 速度过滤** - 低于最小速度阈值的节点将被自动过滤，不会出现在最终配置文件中

## 故障排除

### 测速失败的常见原因

1. **网络连接问题** - 检查网络连接是否稳定
2. **代理节点失效** - 节点可能已经失效
3. **超时时间过短** - 增加 timeout 值
4. **测速服务器不可达** - 更换测速URL

### 调试方法

1. 启用调试模式：`debug_mode = true`
2. 查看详细日志输出
3. 单独测试问题节点
4. 检查测速URL的可访问性

## 性能优化建议

1. **合理设置文件大小** - 根据网络环境选择合适的测试文件大小
2. **调整超时时间** - 避免过长的等待时间
3. **分批处理** - 利用现有的分组机制减少并发压力
4. **缓存结果** - 避免重复测试相同节点

## 更新日志

- **v1.0** - 基础测速功能实现
- **v1.1** - 添加智能速度单位选择
- **v1.2** - 优化错误处理和日志输出
- **v1.3** - 🆕 添加速度过滤功能，自动移除低于阈值的节点