# 测速功能故障排除指南

## 常见错误及解决方案

### 1. "error decoding response body" 错误

**错误描述**: 在进行带宽测速时出现响应体解码错误

**可能原因**:
- 网络连接不稳定
- 代理节点响应异常
- 测速服务器返回非预期内容
- 请求超时导致响应不完整

**解决方案**:

#### 方案1: 调整配置参数
```toml
[speed_test]
enabled = true
url = "https://speed.cloudflare.com/__down?bytes=524288"  # 减小测试文件到512KB
timeout = 20000  # 增加超时时间到20秒
```

#### 方案2: 使用备用测速服务器
程序已内置多个备用URL，会自动尝试：
- `https://speed.cloudflare.com/__down?bytes=1048576` (1MB)
- `https://speed.cloudflare.com/__down?bytes=524288` (512KB)
- `http://speedtest.ftp.otenet.gr/files/test1Mb.db` (备用服务器)

#### 方案3: 临时禁用测速功能
如果测速功能影响正常使用，可以临时禁用：
```toml
[speed_test]
enabled = false
```

### 2. 测速超时错误

**错误描述**: 测速请求超时

**解决方案**:
1. 增加超时时间：`timeout = 30000` (30秒)
2. 减小测试文件大小：`bytes=262144` (256KB)
3. 检查代理节点是否正常工作

### 3. 测速结果异常

**错误描述**: 测速结果显示为0MB或异常高的数值

**解决方案**:
1. 检查网络环境是否稳定
2. 验证代理节点是否正常工作
3. 尝试手动访问测速URL确认可用性

### 4. 所有测速尝试失败

**错误描述**: "All speed test attempts failed"

**解决方案**:
1. 检查网络连接
2. 验证代理设置是否正确
3. 尝试更换测速URL
4. 检查防火墙设置

## 调试步骤

### 1. 启用详细日志
确保配置文件中启用了调试模式：
```toml
debug_mode = true
```

### 2. 检查日志输出
查看程序输出的详细错误信息，特别关注：
- 网络连接错误
- HTTP状态码
- 响应内容类型

### 3. 手动测试
可以手动访问测速URL验证可用性：
```bash
curl -x http://127.0.0.1:7999 "https://speed.cloudflare.com/__down?bytes=1048576"
```

### 4. 网络环境检查
- 确保本地网络连接正常
- 检查是否有防火墙阻止连接
- 验证代理端口是否正确

## 配置建议

### 稳定性优先配置
```toml
[speed_test]
enabled = true
url = "https://speed.cloudflare.com/__down?bytes=524288"  # 512KB，更稳定
timeout = 20000  # 20秒超时
```

### 速度优先配置
```toml
[speed_test]
enabled = true
url = "https://speed.cloudflare.com/__down?bytes=262144"  # 256KB，更快
timeout = 10000  # 10秒超时
```

### 详细测试配置
```toml
[speed_test]
enabled = true
url = "https://speed.cloudflare.com/__down?bytes=2097152"  # 2MB，更准确
timeout = 30000  # 30秒超时
```

## 程序改进

最新版本已包含以下改进：
1. **多重试机制**: 每个URL尝试2次
2. **备用URL**: 自动尝试多个测速服务器
3. **更好的错误处理**: 详细的错误信息输出
4. **智能超时**: 根据文件大小调整超时时间
5. **用户代理**: 添加标准浏览器用户代理

## 联系支持

如果问题仍然存在，请提供以下信息：
1. 完整的错误日志
2. 配置文件内容
3. 网络环境描述
4. 使用的代理节点信息

这将帮助快速定位和解决问题。