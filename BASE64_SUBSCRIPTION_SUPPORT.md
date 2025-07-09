# Base64 订阅类型支持

本项目已为获取节点sub时的订阅添加了完整的Base64订阅类型支持。

## 功能特性

### 1. 智能订阅类型检测
系统现在能够自动检测订阅内容的类型：
- **YAML格式**: 包含 `proxies:` 或 `Proxies:` 关键字的配置文件
- **Base64格式**: Base64编码的代理链接集合
- **链接格式**: 直接的代理链接列表
- **未知格式**: 自动尝试所有解析方法

### 2. 增强的Base64解析
- 支持单行和多行Base64内容
- 自动处理不同的换行符格式（`\n`, `\r\n`）
- 智能Base64内容识别
- 详细的解析日志输出

### 3. 容错机制
- 当主要格式解析失败时，自动尝试其他格式
- 提供详细的错误信息和调试输出
- 支持混合格式的订阅内容

## 代码实现

### 核心组件

#### 1. 订阅类型枚举
```rust
#[derive(Debug, PartialEq)]
pub enum SubscriptionType {
    Yaml,
    Base64,
    Links,
    Unknown,
}
```

#### 2. 智能类型检测
```rust
fn detect_subscription_type(content: &str) -> SubscriptionType
```
- 检查YAML关键字
- 分析链接格式比例
- 验证Base64编码特征

#### 3. 增强的Base64解析
```rust
fn parse_base64_content(content: &str) -> Result<Vec<Proxy>, Box<dyn std::error::Error>>
```
- 支持多种分隔符
- 详细的解析日志
- 错误处理和调试信息

#### 4. 容错解析机制
```rust
fn try_other_formats(content: &str) -> Result<Vec<Proxy>, Box<dyn std::error::Error>>
fn try_all_formats(content: &str) -> Result<Vec<Proxy>, Box<dyn std::error::Error>>
```

## 使用示例

### Base64订阅解析
```rust
use proxrs::sub::SubManager;

// Base64编码的订阅内容
let base64_subscription = "c3M6Ly9ZV1Z6TFRFeU9DMW5ZMjA2WkRsak5UYzNNekk0Wm1Jek5EbG1aUT09QDEyMC4yMzIuNzMuNjg6NDA2NzYjJUYwJTlGJTg3JUFEJUYwJTlGJTg3JUIwSEs=";

match SubManager::parse_content(base64_subscription.to_string()) {
    Ok(proxies) => {
        println!("成功解析 {} 个代理节点", proxies.len());
        for proxy in proxies {
            println!("节点: {}", proxy.get_name());
        }
    }
    Err(e) => {
        println!("解析失败: {}", e);
    }
}
```

### 多行Base64内容
```rust
let multi_line_base64 = r#"c3M6Ly9ZV1Z6TFRFeU9DMW5ZMjA2WkRsak5UYzNNekk0Wm1Jek5EbG1aUT09QDEyMC4yMzIuNzMuNjg6NDA2NzYjJUYwJTlGJTg3JUFEJUYwJTlGJTg3JUIwSEs=
c3M6Ly9ZV1Z6TFRFeU9DMW5ZMjA2WkRsak5UYzNNekk0Wm1Jek5EbG1aUT09QDEyMC4yMzIuNzMuNjg6NDcwMzQjJUYwJTlGJTg3JUFEJUYwJTlGJTg3JUIwSEs="#;

let proxies = SubManager::parse_content(multi_line_base64.to_string())?;
```

## 配置支持

在 `conf/config.toml` 中，Base64订阅链接可以直接添加到 `subs` 数组中：

```toml
subs = [
    "https://example.com/base64-subscription",
    "https://raw.githubusercontent.com/user/repo/main/base64-nodes.txt",
    # 其他订阅链接...
]
```

## 测试验证

项目包含完整的测试套件来验证Base64订阅支持：

```bash
# 测试订阅类型检测
cargo test test_detect_subscription_type -- --nocapture

# 测试Base64解析
cargo test test_parse_base64_subscription -- --nocapture

# 测试Base64内容识别
cargo test test_is_likely_base64 -- --nocapture

# 测试增强的内容解析
cargo test test_enhanced_parse_content -- --nocapture
```

## 日志输出

启用调试模式后，系统会输出详细的解析信息：

```
检测到订阅类型: Base64
Base64 解码后内容长度: 1024
Base64 解码后内容预览: ss://YWVzLTI1Ni1nY206...
成功解析代理: HK-01
成功解析代理: HK-02
Base64 解析完成，成功解析 20 个代理
成功解析 Base64 格式订阅，节点数量: 20
```

## 兼容性

- 完全向后兼容现有的YAML和链接格式订阅
- 支持混合格式的订阅源
- 自动容错和格式检测
- 保持原有的API接口不变

## 性能优化

- 智能类型检测减少不必要的解析尝试
- 优化的Base64解码处理
- 详细但不冗余的日志输出
- 高效的错误处理机制

通过这些改进，Clash-Butler现在能够完美支持Base64格式的订阅，为用户提供更加灵活和可靠的节点获取体验。