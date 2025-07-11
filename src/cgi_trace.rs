use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::str::FromStr;
use std::time::Duration;

use reqwest::Client;
use serde_json::Value;
use tokio::time::sleep;
use tracing::{error, info};

// 保留用于测试的常量
#[allow(dead_code)]
const OPENAI_TRACE_URL: &str = "https://chat.openai.com/cdn-cgi/trace";
#[allow(dead_code)]
const CF_TRACE_URL: &str = "https://1.0.0.1/cdn-cgi/trace";
#[allow(dead_code)]
const CF_CN_TRACE_URL: &str = "https://cf-ns.com/cdn-cgi/trace";

// IP 查询超时时间
const TIMEOUT: Duration = Duration::from_secs(15);

#[allow(dead_code)]
pub async fn get_ip(proxy_url: &str, debug_mode: bool) -> Result<(IpAddr, &str), Box<dyn std::error::Error>> {
    get_ip_simple(proxy_url, debug_mode).await
}

// 简化的IP获取函数，直接使用代理请求 ip-api.com
#[allow(dead_code)]
pub async fn get_ip_simple(proxy_url: &str, debug_mode: bool) -> Result<(IpAddr, &str), Box<dyn std::error::Error>> {
    if debug_mode {
        info!("开始获取 IP，代理地址: {}", proxy_url);
    }

    // 创建代理客户端
    let client = Client::builder()
        .timeout(TIMEOUT)
        .proxy(reqwest::Proxy::all(proxy_url)?)
        .build()?;

    // 直接请求 ip-api.com 获取IP信息
    match client.get("http://ip-api.com/json").send().await {
        Ok(response) => {
            let body = response.text().await?;
            let value: serde_json::Value = serde_json::from_str(&body)?;
            
            if let Some(ip_str) = value.get("query").and_then(|v| v.as_str()) {
                if let Ok(ip) = IpAddr::from_str(ip_str) {
                    if debug_mode {
                        info!("✅ 成功从 ip-api.com 获取 IP: {}", ip);
                    }
                    return Ok((ip, "ip-api"));
                }
            }
            
            Err("无法从 ip-api.com 响应中解析IP地址".into())
        }
        Err(e) => {
            error!("从 ip-api.com 获取 IP 失败: {}", e);
            Err(format!("从 ip-api.com 获取 IP 失败: {}", e).into())
        }
    }
}

// 简化版本：返回IP地址、来源和已测试的代理客户端
pub async fn get_ip_with_clients(proxy_url: &str, debug_mode: bool) -> Result<(IpAddr, &str, Vec<(Client, &'static str)>), Box<dyn std::error::Error>> {
    if debug_mode {
        info!("开始获取 IP，代理地址: {}", proxy_url);
    }

    // 创建代理客户端
    let client = Client::builder()
        .timeout(TIMEOUT)
        .proxy(reqwest::Proxy::all(proxy_url)?)
        .build()?;

    // 直接请求 ip-api.com 获取IP信息
    match client.get("http://ip-api.com/json").send().await {
        Ok(response) => {
            let body = response.text().await?;
            let value: serde_json::Value = serde_json::from_str(&body)?;
            
            if let Some(ip_str) = value.get("query").and_then(|v| v.as_str()) {
                if let Ok(ip) = IpAddr::from_str(ip_str) {
                    if debug_mode {
                        info!("✅ 成功从 ip-api.com 获取 IP: {}", ip);
                    }
                    
                    // 创建一个简单的客户端列表以保持兼容性
                    let working_clients = vec![(client, "ALL")];
                    return Ok((ip, "ip-api", working_clients));
                }
            }
            
            Err("无法从 ip-api.com 响应中解析IP地址".into())
        }
        Err(e) => {
            error!("从 ip-api.com 获取 IP 失败: {}", e);
            Err(format!("从 ip-api.com 获取 IP 失败: {}", e).into())
        }
    }
}

// 获取直连IP（不使用代理）
#[allow(dead_code)]
async fn get_direct_ip(debug_mode: bool) -> Result<IpAddr, Box<dyn std::error::Error>> {
    if debug_mode {
        info!("开始获取直连IP（不使用代理）");
    }
    
    let client = Client::builder()
        .timeout(TIMEOUT)
        .build()?;
    
    // 尝试多个服务获取直连IP
    let services = vec![
        ("https://1.0.0.1/cdn-cgi/trace", "cf"),
        ("https://api4.ipify.org/?format=json", "ipify"),
        ("https://ifconfig.me/ip", "ifconfig"),
    ];
    
    for (url, service) in services {
        if debug_mode {
            info!("尝试从 {} 获取直连IP", service);
        }
        
        match service {
            "cf" => {
                match client.get(url).send().await {
                    Ok(response) => {
                        let body = response.text().await?;
                        let trace = parse_trace_info(body);
                        info!("直连IP获取成功 ({}): {}", service, trace.ip);
                        return Ok(trace.ip);
                    }
                    Err(e) => {
                        if debug_mode {
                            error!("从 {} 获取直连IP失败: {}", service, e);
                        }
                    }
                }
            }
            "ipify" => {
                match client.get(url).send().await {
                    Ok(response) => {
                        let body = response.text().await?;
                        let value: Value = serde_json::from_str(&body)?;
                        if let Some(ip_str) = value.get("ip").and_then(|v| v.as_str()) {
                            if let Ok(ip) = IpAddr::from_str(ip_str) {
                                info!("直连IP获取成功 ({}): {}", service, ip);
                                return Ok(ip);
                            }
                        }
                    }
                    Err(e) => {
                        if debug_mode {
                            error!("从 {} 获取直连IP失败: {}", service, e);
                        }
                    }
                }
            }
            "ifconfig" => {
                match client.get(url).send().await {
                    Ok(response) => {
                        let body = response.text().await?;
                        let ip_str = body.trim();
                        if let Ok(ip) = IpAddr::from_str(ip_str) {
                            info!("直连IP获取成功 ({}): {}", service, ip);
                            return Ok(ip);
                        }
                    }
                    Err(e) => {
                        if debug_mode {
                            error!("从 {} 获取直连IP失败: {}", service, e);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    
    Err("无法获取直连IP".into())
}

// clash 规则走的是国内，没走代理所以寄
#[allow(dead_code)]
async fn get_ip_by_ipip(proxy_url: &str, debug_mode: bool) -> Result<IpAddr, Box<dyn std::error::Error>> {
    let clients = create_proxy_clients(proxy_url, debug_mode).await?;
    
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

// 使用已测试的代理客户端获取IP，避免重复测试
#[allow(dead_code)]
async fn get_ip_by_working_clients(clients: &Vec<(Client, &'static str)>, service: &str, debug_mode: bool) -> Result<IpAddr, Box<dyn std::error::Error>> {
    for (client, proxy_type) in clients {
        if debug_mode {
            info!("{} 尝试使用 {} 代理", service, proxy_type);
        }
        
        let result = match service {
            "ipify" => {
                match client.get("https://api4.ipify.org/?format=json").send().await {
                    Ok(response) => {
                        let body = response.text().await?;
                        let value: Value = serde_json::from_str(&body)?;
                        value.get("ip").and_then(|v| v.as_str()).map(|s| s.to_string())
                    }
                    Err(e) => {
                        if debug_mode {
                            error!("{} {} 代理失败: {}", service, proxy_type, e);
                        }
                        continue;
                    }
                }
            }
            "httpbin" => {
                match client.get("https://httpbin.org/ip").send().await {
                    Ok(response) => {
                        let body = response.text().await?;
                        let value: Value = serde_json::from_str(&body)?;
                        value.get("origin").and_then(|v| v.as_str()).map(|s| {
                            // httpbin 可能返回多个IP，取第一个
                            s.split(',').next().unwrap_or(s).trim().to_string()
                        })
                    }
                    Err(e) => {
                        if debug_mode {
                            error!("{} {} 代理失败: {}", service, proxy_type, e);
                        }
                        continue;
                    }
                }
            }
            "ifconfig" => {
                match client.get("https://ifconfig.me/ip").send().await {
                    Ok(response) => {
                        let body = response.text().await?;
                        Some(body.trim().to_string())
                    }
                    Err(e) => {
                        if debug_mode {
                            error!("{} {} 代理失败: {}", service, proxy_type, e);
                        }
                        continue;
                    }
                }
            }
            _ => None
        };
        
        if let Some(ip_str) = result {
            if let Ok(ip) = IpAddr::from_str(&ip_str) {
                info!("{} 成功使用 {} 代理获取 IP: {}", service, proxy_type, ip);
                return Ok(ip);
            }
        }
    }
    
    Err("所有代理类型都失败".into())
}

#[allow(dead_code)]
async fn get_ip_by_httpbin(proxy_url: &str, debug_mode: bool) -> Result<IpAddr, Box<dyn std::error::Error>> {
    let clients = create_proxy_clients(proxy_url, debug_mode).await?;
    get_ip_by_working_clients(&clients, "httpbin", debug_mode).await
}

#[allow(dead_code)]
async fn get_ip_by_ifconfig(proxy_url: &str, debug_mode: bool) -> Result<IpAddr, Box<dyn std::error::Error>> {
    let clients = create_proxy_clients(proxy_url, debug_mode).await?;
    get_ip_by_working_clients(&clients, "ifconfig", debug_mode).await
}

#[allow(dead_code)]
async fn get_ip_by_ipify(proxy_url: &str, debug_mode: bool) -> Result<IpAddr, Box<dyn std::error::Error>> {
    let clients = create_proxy_clients(proxy_url, debug_mode).await?;
    get_ip_by_working_clients(&clients, "ipify", debug_mode).await
}

#[allow(dead_code)]
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
async fn create_proxy_clients(proxy_url: &str, debug_mode: bool) -> Result<Vec<(Client, &'static str)>, Box<dyn std::error::Error>> {
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
    
    // 测试代理连接并重新排序，优先使用工作的代理
    let working_clients = test_and_reorder_proxy_clients(&clients, proxy_url, debug_mode).await?;
    
    Ok(working_clients)
}

// 测试代理连接并重新排序，优先使用工作的代理
async fn test_and_reorder_proxy_clients(clients: &Vec<(Client, &'static str)>, proxy_url: &str, debug_mode: bool) -> Result<Vec<(Client, &'static str)>, Box<dyn std::error::Error>> {
    if debug_mode {
        info!("开始测试代理连接是否真正工作...");
    }
    
    // 首先获取直连IP作为基准
    let direct_client = Client::builder().timeout(TIMEOUT).build()?;
    let direct_ip = match direct_client.get("https://ifconfig.me/ip").send().await {
        Ok(response) => {
            let body = response.text().await?;
            let ip_str = body.trim();
            match IpAddr::from_str(ip_str) {
                Ok(ip) => {
                    if debug_mode {
                        info!("获取到直连IP: {}", ip);
                    }
                    Some(ip)
                }
                Err(_) => None
            }
        }
        Err(_) => None
    };
    
    // 测试每个代理客户端，记录工作状态
    let mut working_clients = Vec::new();
    let mut failed_clients = Vec::new();
    
    for (client, proxy_type) in clients {
        if debug_mode {
            info!("测试 {} 代理连接...", proxy_type);
        }
        
        let mut is_working = false;
        match client.get("https://ifconfig.me/ip").send().await {
            Ok(response) => {
                let body = response.text().await?;
                let ip_str = body.trim();
                if let Ok(proxy_ip) = IpAddr::from_str(ip_str) {
                    if let Some(direct_ip) = direct_ip {
                        if proxy_ip == direct_ip {
                            error!("⚠️ {} 代理连接测试失败：返回IP {} 与直连IP相同", proxy_type, proxy_ip);
                        } else {
                            info!("✅ {} 代理连接测试成功：代理IP {} 与直连IP {} 不同", proxy_type, proxy_ip, direct_ip);
                            is_working = true;
                        }
                    } else {
                        info!("✅ {} 代理连接测试成功：获取到代理IP {}", proxy_type, proxy_ip);
                        is_working = true;
                    }
                } else {
                    error!("❌ {} 代理连接测试失败：无法解析IP地址 '{}'", proxy_type, ip_str);
                }
            }
            Err(e) => {
                error!("❌ {} 代理连接测试失败：{}", proxy_type, e);
            }
        }
        
        // 由于不能移动Client，我们重新创建相同配置的客户端
        if let Ok(new_client) = recreate_proxy_client(proxy_type, proxy_url) {
            if is_working {
                working_clients.push((new_client, *proxy_type));
            } else {
                failed_clients.push((new_client, *proxy_type));
            }
        }
    }
    
    // 优先返回工作的客户端，然后是失败的客户端（作为备用）
    working_clients.extend(failed_clients);
    
    if working_clients.is_empty() {
        return Err("无法重新创建任何代理客户端".into());
    }
    
    if debug_mode {
        info!("代理连接测试完成，重新排序后共 {} 个代理客户端", working_clients.len());
    }
    
    Ok(working_clients)
}

// 重新创建代理客户端
fn recreate_proxy_client(proxy_type: &str, proxy_url: &str) -> Result<Client, Box<dyn std::error::Error>> {
    match proxy_type {
        "HTTP" => {
            let http_proxy = reqwest::Proxy::http(proxy_url)?;
            let client = Client::builder()
                .timeout(TIMEOUT)
                .proxy(http_proxy)
                .build()?;
            Ok(client)
        }
        "HTTPS" => {
            let https_proxy = reqwest::Proxy::https(proxy_url)?;
            let client = Client::builder()
                .timeout(TIMEOUT)
                .proxy(https_proxy)
                .build()?;
            Ok(client)
        }
        "ALL" => {
            let all_proxy = reqwest::Proxy::all(proxy_url)?;
            let client = Client::builder()
                .timeout(TIMEOUT)
                .proxy(all_proxy)
                .build()?;
            Ok(client)
        }
        "SOCKS5" => {
            let socks_url = proxy_url.replace("http://", "socks5://");
            let socks_proxy = reqwest::Proxy::all(&socks_url)?;
            let client = Client::builder()
                .timeout(TIMEOUT)
                .proxy(socks_proxy)
                .build()?;
            Ok(client)
        }
        _ => Err("未知的代理类型".into())
    }
}

// 使用已测试的代理客户端获取trace信息
#[allow(dead_code)]
async fn get_trace_info_with_working_clients(
    clients: &Vec<(Client, &'static str)>,
    trace_url: &str,
    debug_mode: bool,
) -> Result<TraceInfo, Box<dyn std::error::Error>> {
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

#[allow(dead_code)]
async fn get_trace_info_with_proxy(
    proxy_url: &str,
    trace_url: &str,
    debug_mode: bool,
) -> Result<TraceInfo, Box<dyn std::error::Error>> {
    let clients = create_proxy_clients(proxy_url, debug_mode).await?;
    get_trace_info_with_working_clients(&clients, trace_url, debug_mode).await
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
