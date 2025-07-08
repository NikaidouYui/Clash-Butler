use std::time::Duration;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use tracing::{error, info};
use reqwest::Client;
use reqwest::StatusCode;

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/102.0.5005.63 Safari/537.36";
const TIMEOUT: Duration = Duration::from_secs(10);

#[allow(dead_code)]
fn build_clients(proxy_url: &str, debug_mode: bool) -> Result<Vec<(Client, &'static str)>> {
    let mut clients = Vec::new();
    
    if debug_mode {
        info!("尝试为 {} 创建代理客户端", proxy_url);
    }
    
    // 尝试 HTTP 代理
    if let Ok(http_proxy) = reqwest::Proxy::http(proxy_url) {
        if let Ok(client) = Client::builder()
            .timeout(TIMEOUT)
            .proxy(http_proxy)
            .build()
        {
            clients.push((client, "HTTP"));
            if debug_mode {
                info!("成功创建 HTTP 代理客户端");
            }
        } else if debug_mode {
            error!("创建 HTTP 代理客户端失败");
        }
    } else if debug_mode {
        error!("创建 HTTP 代理失败");
    }
    
    // 尝试 HTTPS 代理
    if let Ok(https_proxy) = reqwest::Proxy::https(proxy_url) {
        if let Ok(client) = Client::builder()
            .timeout(TIMEOUT)
            .proxy(https_proxy)
            .build()
        {
            clients.push((client, "HTTPS"));
            if debug_mode {
                info!("成功创建 HTTPS 代理客户端");
            }
        } else if debug_mode {
            error!("创建 HTTPS 代理客户端失败");
        }
    } else if debug_mode {
        error!("创建 HTTPS 代理失败");
    }
    
    // 尝试所有协议代理
    if let Ok(all_proxy) = reqwest::Proxy::all(proxy_url) {
        if let Ok(client) = Client::builder()
            .timeout(TIMEOUT)
            .proxy(all_proxy)
            .build()
        {
            clients.push((client, "ALL"));
            if debug_mode {
                info!("成功创建 ALL 代理客户端");
            }
        } else if debug_mode {
            error!("创建 ALL 代理客户端失败");
        }
    } else if debug_mode {
        error!("创建 ALL 代理失败");
    }
    
    // 如果是 HTTP URL，尝试转换为 SOCKS5
    if proxy_url.starts_with("http://") {
        let socks_url = proxy_url.replace("http://", "socks5://");
        if let Ok(socks_proxy) = reqwest::Proxy::all(&socks_url) {
            if let Ok(client) = Client::builder()
                .timeout(TIMEOUT)
                .proxy(socks_proxy)
                .build()
            {
                clients.push((client, "SOCKS5"));
                if debug_mode {
                    info!("成功创建 SOCKS5 代理客户端");
                }
            } else if debug_mode {
                error!("创建 SOCKS5 代理客户端失败");
            }
        } else if debug_mode {
            error!("创建 SOCKS5 代理失败");
        }
    }
    
    if clients.is_empty() {
        if debug_mode {
            error!("无法为 {} 创建任何代理客户端", proxy_url);
        }
        return Err(anyhow!("无法创建任何代理客户端"));
    }
    
    if debug_mode {
        info!("为 {} 成功创建 {} 个代理客户端", proxy_url, clients.len());
    }
    
    Ok(clients)
}

fn build_client(proxy_url: &str) -> Result<Client> {
    Client::builder()
        .proxy(reqwest::Proxy::all(proxy_url).context("Failed to create proxy configuration")?)
        .timeout(TIMEOUT)
        .build()
        .context("Failed to build HTTP client")
}

// 使用已测试的代理客户端检测Claude
pub async fn claude_is_ok_with_clients(clients: &Vec<(Client, &'static str)>, debug_mode: bool) -> Result<()> {
    let url = "https://claude.ai/login";
    
    for (client, proxy_type) in clients {
        if debug_mode {
            info!("Claude 尝试使用 {} 代理", proxy_type);
        }
        
        match client
            .get(url)
            .header("User-Agent", USER_AGENT)
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                let text = resp.text().await?;
                if debug_mode {
                    info!("Claude {} 代理响应状态: {}", proxy_type, status);
                }
                if text.contains("unavailable") {
                    if debug_mode {
                        error!("Claude {} 代理返回 unavailable", proxy_type);
                    }
                    continue; // 尝试下一个代理类型
                }
                // 将 403 Forbidden 视为可访问（网络连通但权限限制）
                if status.is_success() || status == StatusCode::FORBIDDEN {
                    if debug_mode {
                        info!("Claude {} 代理测试成功", proxy_type);
                    }
                    return Ok(());
                }
            }
            Err(e) => {
                if debug_mode {
                    error!("Claude {} 代理请求失败: {}", proxy_type, e);
                }
                continue; // 尝试下一个代理类型
            }
        }
    }
    
    Err(anyhow!("所有代理类型都失败"))
}

// 使用已测试的代理客户端检测OpenAI
pub async fn openai_is_ok_with_clients(clients: &Vec<(Client, &'static str)>, debug_mode: bool) -> Result<()> {
    let url = "https://auth.openai.com/favicon.ico";
    
    for (client, proxy_type) in clients {
        if debug_mode {
            info!("OpenAI 尝试使用 {} 代理", proxy_type);
        }
        
        match client
            .get(url)
            .header("User-Agent", USER_AGENT)
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                if debug_mode {
                    info!("OpenAI {} 代理响应状态: {}", proxy_type, status);
                }
                if status == StatusCode::OK {
                    if debug_mode {
                        info!("OpenAI {} 代理测试成功", proxy_type);
                    }
                    return Ok(());
                }
            }
            Err(e) => {
                if debug_mode {
                    error!("OpenAI {} 代理请求失败: {}", proxy_type, e);
                }
                continue; // 尝试下一个代理类型
            }
        }
    }
    
    Err(anyhow!("所有代理类型都失败"))
}

// 保留原有函数以保持向后兼容
#[allow(dead_code)]
pub async fn claude_is_ok(proxy_url: &str, debug_mode: bool) -> Result<()> {
    let clients = build_clients(proxy_url, debug_mode)?;
    claude_is_ok_with_clients(&clients, debug_mode).await
}

#[allow(dead_code)]
pub async fn openai_is_ok(proxy_url: &str, debug_mode: bool) -> Result<()> {
    let clients = build_clients(proxy_url, debug_mode)?;
    openai_is_ok_with_clients(&clients, debug_mode).await
}

#[allow(dead_code)]
pub async fn youtube_music_is_ok(proxy_url: &str) -> Result<bool> {
    let url = "https://music.youtube.com/generate_204";
    let client = build_client(proxy_url)?;
    let resp = client
        .head(url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .with_context(|| "Failed to send request to OpenAI")?;
    let status = resp.status();
    if status == StatusCode::NO_CONTENT {
        return Ok(true);
    }
    Err(anyhow!("error status code: {}", status))
}

mod test {
    #[tokio::test]
    #[ignore]
    async fn test_claude_is_ok() {
        let result = super::claude_is_ok("http://localhost:7890", true).await;
        println!("{:?}", result);
    }

    #[tokio::test]
    #[ignore]
    async fn test_openai_is_ok() {
        let result = super::openai_is_ok("http://localhost:7890", true).await;
        println!("{:?}", result);
    }
}
