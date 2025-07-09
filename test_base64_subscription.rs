use proxrs::sub::SubManager;

fn main() {
    // 示例1: 测试Base64订阅内容
    let base64_subscription = "c3M6Ly9ZV1Z6TFRFeU9DMW5ZMjA2WkRsak5UYzNNekk0Wm1Jek5EbG1aUT09QDEyMC4yMzIuNzMuNjg6NDA2NzYjJUYwJTlGJTg3JUFEJUYwJTlGJTg3JUIwSEs=";
    
    println!("=== 测试Base64订阅解析 ===");
    match SubManager::parse_content(base64_subscription.to_string()) {
        Ok(proxies) => {
            println!("✅ 成功解析Base64订阅，节点数量: {}", proxies.len());
            for (i, proxy) in proxies.iter().enumerate() {
                println!("  {}. 节点名称: {}", i + 1, proxy.get_name());
                println!("     服务器: {}", proxy.get_server());
            }
        }
        Err(e) => {
            println!("❌ Base64订阅解析失败: {}", e);
        }
    }

    // 示例2: 测试多行Base64内容（模拟真实订阅）
    let multi_line_base64 = r#"c3M6Ly9ZV1Z6TFRFeU9DMW5ZMjA2WkRsak5UYzNNekk0Wm1Jek5EbG1aUT09QDEyMC4yMzIuNzMuNjg6NDA2NzYjJUYwJTlGJTg3JUFEJUYwJTlGJTg3JUIwSEs=
c3M6Ly9ZV1Z6TFRFeU9DMW5ZMjA2WkRsak5UYzNNekk0Wm1Jek5EbG1aUT09QDEyMC4yMzIuNzMuNjg6NDcwMzQjJUYwJTlGJTg3JUFEJUYwJTlGJTg3JUIwSEs="#;

    println!("\n=== 测试多行Base64订阅解析 ===");
    match SubManager::parse_content(multi_line_base64.to_string()) {
        Ok(proxies) => {
            println!("✅ 成功解析多行Base64订阅，节点数量: {}", proxies.len());
            for (i, proxy) in proxies.iter().enumerate() {
                println!("  {}. 节点名称: {}", i + 1, proxy.get_name());
            }
        }
        Err(e) => {
            println!("❌ 多行Base64订阅解析失败: {}", e);
        }
    }

    // 示例3: 测试YAML格式（对比）
    let yaml_subscription = r#"
proxies:
  - name: "测试节点"
    type: ss
    server: 1.2.3.4
    port: 443
    cipher: aes-256-gcm
    password: test123
"#;

    println!("\n=== 测试YAML订阅解析（对比） ===");
    match SubManager::parse_content(yaml_subscription.to_string()) {
        Ok(proxies) => {
            println!("✅ 成功解析YAML订阅，节点数量: {}", proxies.len());
            for (i, proxy) in proxies.iter().enumerate() {
                println!("  {}. 节点名称: {}", i + 1, proxy.get_name());
            }
        }
        Err(e) => {
            println!("❌ YAML订阅解析失败: {}", e);
        }
    }

    // 示例4: 测试链接格式（对比）
    let links_subscription = r#"ss://YWVzLTI1Ni1nY206UUlHVVo3VkRQWk9BU0M5SEAxMjAuMjQxLjQ1LjUwOjE3MDAx#US-01
ss://YWVzLTI1Ni1nY206UUlHVVo3VkRQWk9BU0M5SEAxMjAuMjQxLjQ1LjUwOjE3MDAy#US-02"#;

    println!("\n=== 测试链接订阅解析（对比） ===");
    match SubManager::parse_content(links_subscription.to_string()) {
        Ok(proxies) => {
            println!("✅ 成功解析链接订阅，节点数量: {}", proxies.len());
            for (i, proxy) in proxies.iter().enumerate() {
                println!("  {}. 节点名称: {}", i + 1, proxy.get_name());
            }
        }
        Err(e) => {
            println!("❌ 链接订阅解析失败: {}", e);
        }
    }
}