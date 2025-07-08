use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::str::FromStr;
use std::time::Duration;

use futures_util::future::select_ok;
use futures_util::future::BoxFuture;
use futures_util::FutureExt;
use reqwest::Client;
use serde_json::Value;
use tokio::time::sleep;
use tracing::{error, info};

const OPENAI_TRACE_URL: &str = "https://chat.openai.com/cdn-cgi/trace";
const CF_TRACE_URL: &str = "https://1.0.0.1/cdn-cgi/trace";

#[allow(unused)]
const CF_CN_TRACE_URL: &str = "https://cf-ns.com/cdn-cgi/trace";

// IP 查询超时时间
const TIMEOUT: Duration = Duration::from_secs(15);

type IpBoxFuture<'a> = BoxFuture<'a, Result<(IpAddr, &'a str), Box<dyn std::error::Error>>>;

pub async fn get_ip(proxy_url: &str, debug_mode: bool) -> Result<(IpAddr, &str), Box<dyn std::error::Error>> {
    if debug_mode {
        info!("开始获取 IP，代理地址: {}", proxy_url);
    }

    // 添加多个不同的IP检测服务以交叉验证
    let cf_future: IpBoxFuture = async move {
        match get_trace_info_with_proxy(proxy_url, CF_TRACE_URL, debug_mode).await {
            Ok(trace) => {
                if debug_mode {
                    info!("Cloudflare 返回 IP: {}", trace.ip);
                }
                Ok((trace.ip, "cf"))
            },
            Err(e) => {
                error!("从 Cloudflare 获取 IP 失败, {e}");
                Err(e)
            }
        }
    }
    .boxed();

    let ipify_future: IpBoxFuture = async move {
        match get_ip_by_ipify(proxy_url, debug_mode).await {
            Ok(ip) => {
                if debug_mode {
                    info!("ipify 返回 IP: {}", ip);
                }
                Ok((ip, "ipify"))
            },
            Err(e) => {
                error!("从 ipify 获取 IP 失败, {e}");
                Err(e)
            }
        }
    }
    .boxed();

    let openai_future: IpBoxFuture = async move {
        match get_trace_info_with_proxy(proxy_url, OPENAI_TRACE_URL, debug_mode).await {
            Ok(trace) => {
                if debug_mode {
                    info!("OpenAI 返回 IP: {}", trace.ip);
                }
                Ok((trace.ip, "openai"))
            },
            Err(e) => {
                error!("从 OpenAI 获取 IP 失败, {e}");
                Err(e)
            }
        }
    }
    .boxed();

    // 添加新的IP检测服务
    let httpbin_future: IpBoxFuture = async move {
        match get_ip_by_httpbin(proxy_url, debug_mode).await {
            Ok(ip) => {
                if debug_mode {
                    info!("httpbin 返回 IP: {}", ip);
                }
                Ok((ip, "httpbin"))
            },
            Err(e) => {
                if debug_mode {
                    error!("从 httpbin 获取 IP 失败, {e}");
                }
                Err(e)
            }
        }
    }
    .boxed();

    // 添加更多IP检测服务
    let ifconfig_future: IpBoxFuture = async move {
        match get_ip_by_ifconfig(proxy_url, debug_mode).await {
            Ok(ip) => {
                if debug_mode {
                    info!("ifconfig 返回 IP: {}", ip);
                }
                Ok((ip, "ifconfig"))
            },
            Err(e) => {
                if debug_mode {
                    error!("从 ifconfig 获取 IP 失败, {e}");
                }
                Err(e)
            }
        }
    }
    .boxed();

    let futures = vec![cf_future, ipify_future, openai_future, httpbin_future, ifconfig_future];
    
    // 收集所有成功的结果进行比较
    let mut all_results = Vec::new();
    for future in futures {
        if let Ok(result) = future.await {
            all_results.push(result);
        }
    }
    
    if all_results.is_empty() {
        return Err("获取不到 IP 地址，可能节点已失效，已过滤".into());
    }
    
    // 如果有多个结果，检查是否一致
    if all_results.len() > 1 {
        let first_ip = all_results[0].0;
        let mut all_same = true;
        for (ip, source) in &all_results {
            if *ip != first_ip {
                all_same = false;
                info!("IP检测结果不一致: {} 来源 {} vs {} 来源 {}",
                      ip, source, first_ip, all_results[0].1);
            }
        }
        
        if !all_same {
            info!("多个IP检测服务返回不同结果，使用第一个成功的结果");
        } else {
            info!("多个IP检测服务返回一致结果: {}", first_ip);
        }
    }
    
    let (ip, from) = all_results[0];
    info!("最终确定 IP: {} (来源: {})", ip, from);
    Ok((ip, from))
}

// clash 规则走的是国内，没走代理所以寄
#[allow(dead_code)]
async fn get_ip_by_ipip(proxy_url: &str, debug_mode: bool) -> Result<IpAddr, Box<dyn std::error::Error>> {
    let clients = create_proxy_clients(proxy_url, debug_mode)?;
    
    for (client, proxy_type) in clients {
        if debug_mode {
            info!("ipip 尝试使用 {} 代理", proxy_type);
        }
        
        match client.get("https://myip.ipip.net/ip").send().await {
            Ok(response) => {
                let body = response.text().await?;
                let value: Value = serde_json::from_str(&body)?;
                
                if let Some(ip_str) = value.get("ip").and_then(|v| v.as_str()) {
                    if let Ok(ip) = IpAddr::from_str(ip_str) {
                        info!("ipip 成功使用 {} 代理获取 IP: {}", proxy_type, ip);
                        return Ok(ip);
                    }
                }
            }
            Err(e) => {
                if debug_mode {
                    error!("ipip {} 代理失败: {}", proxy_type, e);
                }
                continue;
            }
        }
    }
    
    Err("所有代理类型都失败".into())
}

async fn get_ip_by_httpbin(proxy_url: &str, debug_mode: bool) -> Result<IpAddr, Box<dyn std::error::Error>> {
    let clients = create_proxy_clients(proxy_url, debug_mode)?;
    
    for (client, proxy_type) in clients {
        if debug_mode {
            info!("httpbin 尝试使用 {} 代理", proxy_type);
        }
        
        match client
            .get("https://httpbin.org/ip")
            .send()
            .await
        {
            Ok(response) => {
                let body = response.text().await?;
                let value: Value = serde_json::from_str(&body)?;
                
                if let Some(ip_str) = value.get("origin").and_then(|v| v.as_str()) {
                    // httpbin 可能返回多个IP，取第一个
                    let ip_str = ip_str.split(',').next().unwrap_or(ip_str).trim();
                    if let Ok(ip) = IpAddr::from_str(ip_str) {
                        info!("httpbin 成功使用 {} 代理获取 IP: {}", proxy_type, ip);
                        return Ok(ip);
                    }
                }
            }
            Err(e) => {
                if debug_mode {
                    error!("httpbin {} 代理失败: {}", proxy_type, e);
                }
                continue;
            }
        }
    }
    
    Err("所有代理类型都失败".into())
}

async fn get_ip_by_ifconfig(proxy_url: &str, debug_mode: bool) -> Result<IpAddr, Box<dyn std::error::Error>> {
    let clients = create_proxy_clients(proxy_url, debug_mode)?;
    
    for (client, proxy_type) in clients {
        if debug_mode {
            info!("ifconfig 尝试使用 {} 代理", proxy_type);
        }
        
        match client
            .get("https://ifconfig.me/ip")
            .send()
            .await
        {
            Ok(response) => {
                let body = response.text().await?;
                let ip_str = body.trim();
                if let Ok(ip) = IpAddr::from_str(ip_str) {
                    info!("ifconfig 成功使用 {} 代理获取 IP: {}", proxy_type, ip);
                    return Ok(ip);
                }
            }
            Err(e) => {
                if debug_mode {
                    error!("ifconfig {} 代理失败: {}", proxy_type, e);
                }
                continue;
            }
        }
    }
    
    Err("所有代理类型都失败".into())
}

async fn get_ip_by_ipify(proxy_url: &str, debug_mode: bool) -> Result<IpAddr, Box<dyn std::error::Error>> {
    // 尝试多种代理配置
    let clients = create_proxy_clients(proxy_url, debug_mode)?;
    
    for (client, proxy_type) in clients {
        if debug_mode {
            info!("ipify 尝试使用 {} 代理", proxy_type);
        }
        
        match client
            .get("https://api4.ipify.org/?format=json")
            .send()
            .await
        {
            Ok(response) => {
                let body = response.text().await?;
                let value: Value = serde_json::from_str(&body)?;
                
                if let Some(ip_str) = value.get("ip").and_then(|v| v.as_str()) {
                    if let Ok(ip) = IpAddr::from_str(ip_str) {
                        info!("ipify 成功使用 {} 代理获取 IP: {}", proxy_type, ip);
                        return Ok(ip);
                    }
                }
            }
            Err(e) => {
                if debug_mode {
                    error!("ipify {} 代理失败: {}", proxy_type, e);
                }
                continue;
            }
        }
    }
    
    Err("所有代理类型都失败".into())
}

fn parse_trace_info(text: String) -> TraceInfo {
    let mut map = HashMap::new();
    for line in text.lines() {
        let parts: Vec<&str> = line.split('=').collect();
        if parts.len() == 2 {
            map.insert(parts[0].to_string(), parts[1].to_string());
        }
    }

    TraceInfo {
        fl: map.get("fl").unwrap_or(&String::new()).clone(),
        h: map.get("h").unwrap_or(&String::new()).clone(),
        ip: IpAddr::from_str(&map.get("ip").unwrap_or(&String::new()).clone()).unwrap_or(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))),
        ts: map.get("ts").unwrap_or(&String::new()).clone(),
        visit_scheme: map.get("visit_scheme").unwrap_or(&String::new()).clone(),
        uag: map.get("uag").unwrap_or(&String::new()).clone(),
        colo: map.get("colo").unwrap_or(&String::new()).clone(),
        sliver: map.get("sliver").unwrap_or(&String::new()).clone(),
        http: map.get("http").unwrap_or(&String::new()).clone(),
        loc: map.get("loc").unwrap_or(&String::new()).clone(),
        tls: map.get("tls").unwrap_or(&String::new()).clone(),
        sni: map.get("sni").unwrap_or(&String::new()).clone(),
        warp: map.get("warp").unwrap_or(&String::new()).clone(),
        gateway: map.get("gateway").unwrap_or(&String::new()).clone(),
        rbi: map.get("rbi").unwrap_or(&String::new()).clone(),
        kex: map.get("kex").unwrap_or(&String::new()).clone(),
    }
}

// 创建多种代理客户端配置
fn create_proxy_clients(proxy_url: &str, debug_mode: bool) -> Result<Vec<(Client, &'static str)>, Box<dyn std::error::Error>> {
    let mut clients = Vec::new();
    
    if debug_mode {
        info!("创建代理客户端，代理地址: {}", proxy_url);
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
        }
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
        }
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
                info!("成功创建 ALL 协议代理客户端");
            }
        }
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
                    info!("成功创建 SOCKS5 代理客户端: {}", socks_url);
                }
            }
        }
    }
    
    if clients.is_empty() {
        return Err("无法创建任何代理客户端".into());
    }
    
    if debug_mode {
        info!("总共创建了 {} 个代理客户端", clients.len());
    }
    
    Ok(clients)
}

async fn get_trace_info_with_proxy(
    proxy_url: &str,
    trace_url: &str,
    debug_mode: bool,
) -> Result<TraceInfo, Box<dyn std::error::Error>> {
    let clients = create_proxy_clients(proxy_url, debug_mode)?;
    
    for (client, proxy_type) in clients {
        if debug_mode {
            info!("trace 尝试使用 {} 代理访问 {}", proxy_type, trace_url);
        }
        
        let mut attempts = 0;
        let max_attempts = 2; // 减少每个代理类型的重试次数
        
        while attempts < max_attempts {
            match client.get(trace_url).send().await {
                Ok(res) => {
                    let body = res.text().await?;
                    info!("trace 成功使用 {} 代理获取信息", proxy_type);
                    return Ok(parse_trace_info(body));
                }
                Err(e) => {
                    if attempts + 1 == max_attempts {
                        if debug_mode {
                            error!("trace {} 代理失败: {}", proxy_type, e);
                        }
                        break;
                    }
                    attempts += 1;
                    if debug_mode {
                        info!("trace {} 代理第 {} 次尝试失败，重试中...", proxy_type, attempts);
                    }
                    sleep(Duration::from_millis(500)).await; // 减少等待时间
                }
            }
        }
    }
    
    Err("所有代理类型都失败".into())
}

#[derive(Debug)]
#[allow(unused)]
struct TraceInfo {
    fl: String,
    h: String,
    ip: IpAddr,
    ts: String,
    visit_scheme: String,
    uag: String,
    colo: String,
    sliver: String,
    http: String,
    loc: String,
    tls: String,
    sni: String,
    warp: String,
    gateway: String,
    rbi: String,
    kex: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROXY_URL: &str = "http://127.0.0.1:7999";

    #[tokio::test]
    #[ignore]
    async fn test_get_ip() {
        let result = get_ip(PROXY_URL).await;
        println!("{:?}", result.unwrap())
    }

    #[tokio::test]
    #[ignore]
    async fn test_get_trace_info_with_proxy() {
        let result = get_trace_info_with_proxy(PROXY_URL, OPENAI_TRACE_URL).await;
        println!("{:?}", result);
        let result = get_trace_info_with_proxy(PROXY_URL, CF_TRACE_URL).await;
        println!("{:?}", result);
        let result = get_trace_info_with_proxy(PROXY_URL, CF_CN_TRACE_URL).await;
        println!("{:?}", result);
    }

    #[tokio::test]
    #[ignore]
    async fn test_get_ip_from_ipip() {
        let result = get_ip_by_ipip(PROXY_URL).await;
        println!("{:?}", result);
        let result = get_ip_by_ipify(PROXY_URL).await;
        println!("{:?}", result);
    }
}
