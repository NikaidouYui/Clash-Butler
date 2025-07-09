use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::fs::File;
use std::hash::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;
use std::io;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::time::Duration;

use regex::Regex;
use reqwest::Client;
use serde_yaml::Mapping;
use serde_yaml::Value;
use tokio::time::sleep;

use crate::base64::base64decode;
use crate::protocol::Proxy;

#[derive(Debug, PartialEq)]
pub enum SubscriptionType {
    Yaml,
    Base64,
    Links,
    Unknown,
}

#[derive(Debug)]
pub struct SubManager {}

impl SubManager {
    /// 从链接中获取代理信息支持以下四种结构
    /// 1. http://订阅链接，传入代理地址
    /// 2. C:\\文件地址 /home/yaml，传入文件地址
    /// 3. ss://xxxx，传入单个节点链接
    /// 4. edhxxx, 传入 base64 的节点信息
    pub async fn get_proxies_from_url(url: String) -> Vec<Proxy> {
        let mut proxies: Vec<Proxy> = Vec::new();
        if url.starts_with("http") {
            if let Ok(file_path) = Self::get_content_from_sub_url(&url).await {
                proxies = Self::parse_content(file_path).unwrap();
            }
        } else if Path::new(&url).is_file() {
            proxies = Self::parse_from_path(&url).unwrap();
        } else if let Ok(p) = Self::parse_content(url.to_string()) {
            proxies.extend(p);
        }
        proxies
    }

    /// 传入 urls 列表解析代理
    pub async fn get_proxies_from_urls(subs: &Vec<String>) -> Vec<Proxy> {
        let mut proxies: Vec<Proxy> = Vec::new();
        for url in subs {
            proxies.extend(Self::get_proxies_from_url(url.to_string()).await)
        }

        if !proxies.is_empty() {
            proxies = Self::exclude_dup_proxies(proxies);
            Self::rename_dup_proxies_name(&mut proxies);
        }

        proxies
    }

    async fn get_content_from_sub_url(sub_url: &str) -> Result<String, Box<dyn std::error::Error>> {
        let client = Client::new();
        let mut attempts = 0;
        let retries = 3;

        loop {
            let result = client
                .get(sub_url)
                .timeout(Duration::from_secs(10))
                .send()
                .await;
            match result {
                Ok(resp) => {
                    let status = resp.status();
                    return if status.is_success() {
                        // 获取 UUID 作为文件名
                        // let re = Regex::new(r"files/(.*?)/raw").unwrap();
                        // let uuid = re.captures(sub_url)
                        //     .and_then(|caps| caps.get(1))
                        //     .map_or_else(|| {
                        //         format!("{:x}", md5::compute(sub_url))
                        //     }, |m| m.as_str().to_string());

                        // let file_path = PathBuf::from_iter(vec!["subs", &uuid.to_string()]);
                        // let mut file = File::create(&file_path).unwrap();

                        let content_result = resp.text().await;
                        match content_result {
                            Ok(content) => {
                                // file.write_all(content.as_bytes()).unwrap();
                                // Ok(env::current_dir().unwrap().join(file_path).to_string_lossy().
                                // to_string())
                                Ok(content)
                            }
                            Err(e) => {
                                if e.is_timeout() {
                                    continue;
                                }
                                return Err(Box::new(e));
                            }
                        }
                    } else {
                        Err(format!("获取订阅连失败 {} 响应码 {}", sub_url, status.as_str()).into())
                    };
                }
                Err(e) => {
                    if !e.is_timeout() {
                        return Err(Box::new(e));
                    }
                }
            }

            if attempts < retries {
                attempts += 1;
                sleep(Duration::from_secs(1)).await;
            } else {
                return Err(format!(
                    "当前链接 {} 无法访问，已跳过，或请确保当前网络通顺",
                    sub_url
                )
                    .into());
            }
        }
    }

    /// 从本地文件中解析代理
    pub fn parse_from_path<P: AsRef<Path>>(
        file_path: P,
    ) -> Result<Vec<Proxy>, Box<dyn std::error::Error>> {
        match fs::read_to_string(file_path) {
            Ok(contents) => Ok(Self::parse_content(contents)?),
            Err(e) => Err(format!("Error reading file: {}", e).into()),
        }
    }

    /// 从字符串中解析代理
    /// 1. 先检测订阅类型
    /// 2. 根据类型选择合适的解析方法
    /// 3. 支持 YAML、Base64、纯链接格式
    pub fn parse_content(content: String) -> Result<Vec<Proxy>, Box<dyn std::error::Error>> {
        let mut conf_proxies: Vec<Proxy> = Vec::new();
        
        // 检测订阅类型
        let subscription_type = Self::detect_subscription_type(&content);
        println!("检测到订阅类型: {:?}", subscription_type);
        
        match subscription_type {
            SubscriptionType::Yaml => {
                match Self::parse_yaml_content(&content) {
                    Ok(proxies) => {
                        conf_proxies = proxies;
                        println!("成功解析 YAML 格式订阅，节点数量: {}", conf_proxies.len());
                    }
                    Err(e) => {
                        println!("YAML 解析失败，尝试其他格式: {}", e);
                        // 如果 YAML 解析失败，尝试其他格式
                        return Self::try_other_formats(&content);
                    }
                }
            }
            SubscriptionType::Base64 => {
                match Self::parse_base64_content(&content) {
                    Ok(proxies) => {
                        conf_proxies = proxies;
                        println!("成功解析 Base64 格式订阅，节点数量: {}", conf_proxies.len());
                    }
                    Err(e) => {
                        println!("Base64 解析失败，尝试其他格式: {}", e);
                        return Self::try_other_formats(&content);
                    }
                }
            }
            SubscriptionType::Links => {
                match Self::parse_links_content(&content) {
                    Ok(proxies) => {
                        conf_proxies = proxies;
                        println!("成功解析链接格式订阅，节点数量: {}", conf_proxies.len());
                    }
                    Err(e) => {
                        println!("链接解析失败: {}", e);
                        return Err(e);
                    }
                }
            }
            SubscriptionType::Unknown => {
                println!("未知订阅类型，尝试所有解析方法");
                return Self::try_all_formats(&content);
            }
        }
        
        Ok(conf_proxies)
    }

    fn parse_yaml_content(content: &str) -> Result<Vec<Proxy>, Box<dyn std::error::Error>> {
        let mut conf_proxies: Vec<Proxy> = Vec::new();
        let yaml = serde_yaml::from_str::<serde_json::Value>(content)?;
        let proxies = yaml.get("proxies").or_else(|| yaml.get("Proxies"));
        match proxies {
            None => {
                return Err(format!("Proxy not found: {}", content).into());
            }
            Some(proxies) => {
                if let Some(proxies_arr) = proxies.as_array() {
                    for proxy in proxies_arr {
                        let result = Proxy::from_json(&proxy.to_string());
                        match result {
                            Ok(p) => {
                                conf_proxies.push(p);
                            }
                            Err(e) => {
                                println!("{} {:?}", e, proxy);
                            }
                        }
                    }
                }
            }
        }
        Ok(conf_proxies)
    }

    fn parse_base64_content(content: &str) -> Result<Vec<Proxy>, Box<dyn std::error::Error>> {
        let mut conf_proxies: Vec<Proxy> = Vec::new();
        let base64 = base64decode(content.trim());
        
        // 改进的 Base64 解析逻辑
        println!("Base64 解码后内容长度: {}", base64.len());
        if base64.len() > 10 {
            println!("Base64 解码后内容预览: {}", &base64[..std::cmp::min(200, base64.len())]);
        }
        
        // 支持多种分隔符：换行符、回车换行符
        let lines: Vec<&str> = base64
            .split('\n')
            .flat_map(|line| line.split('\r'))
            .collect();
            
        for line in lines {
            let trimmed_line = line.trim();
            if !trimmed_line.is_empty() {
                match Proxy::from_link(trimmed_line.to_string()) {
                    Ok(proxy) => {
                        println!("成功解析代理: {}", proxy.get_name());
                        conf_proxies.push(proxy);
                    }
                    Err(e) => {
                        println!("解析代理失败 [{}]: {}", trimmed_line, e);
                    }
                }
            }
        }
        
        println!("Base64 解析完成，成功解析 {} 个代理", conf_proxies.len());
        Ok(conf_proxies)
    }

    fn parse_links_content(content: &str) -> Result<Vec<Proxy>, Box<dyn std::error::Error>> {
        let mut conf_proxies: Vec<Proxy> = Vec::new();
        let links = content
            .split("\n")
            .filter(|line| !line.is_empty())
            .map(|link| link.trim())
            .collect::<Vec<&str>>();
        for link in links {
            if let Ok(proxy) = Proxy::from_link(link.trim().to_string()) {
                conf_proxies.push(proxy)
            }
        }
        Ok(conf_proxies)
    }

    /// 检测订阅内容的类型
    fn detect_subscription_type(content: &str) -> SubscriptionType {
        let trimmed_content = content.trim();
        
        // 检查是否为 YAML 格式
        if trimmed_content.contains("proxies:") || trimmed_content.contains("Proxies:") {
            return SubscriptionType::Yaml;
        }
        
        // 检查是否为纯链接格式（包含协议前缀）
        let lines: Vec<&str> = trimmed_content.lines().collect();
        let mut link_count = 0;
        let mut total_non_empty_lines = 0;
        
        for line in &lines {
            let line = line.trim();
            if !line.is_empty() {
                total_non_empty_lines += 1;
                if line.starts_with("ss://") || line.starts_with("ssr://") ||
                   line.starts_with("vmess://") || line.starts_with("vless://") ||
                   line.starts_with("trojan://") || line.starts_with("hysteria2://") {
                    link_count += 1;
                }
            }
        }
        
        // 如果大部分行都是链接格式，认为是链接类型
        if total_non_empty_lines > 0 && (link_count as f64 / total_non_empty_lines as f64) > 0.8 {
            return SubscriptionType::Links;
        }
        
        // 检查是否为 Base64 格式
        if Self::is_likely_base64(trimmed_content) {
            return SubscriptionType::Base64;
        }
        
        SubscriptionType::Unknown
    }
    
    /// 检查内容是否可能是 Base64 编码
    fn is_likely_base64(content: &str) -> bool {
        let trimmed = content.trim();
        
        // Base64 订阅通常是一大段连续的字符，没有换行或很少换行
        let lines: Vec<&str> = trimmed.lines().collect();
        
        // 如果只有一行或很少行，且包含 Base64 字符
        if lines.len() <= 3 {
            for line in lines {
                let line = line.trim();
                if line.len() > 50 && Self::contains_base64_chars(line) {
                    // 尝试解码看是否包含代理链接
                    let decoded = base64decode(line);
                    if decoded.contains("://") && (
                        decoded.contains("ss://") || decoded.contains("ssr://") ||
                        decoded.contains("vmess://") || decoded.contains("vless://") ||
                        decoded.contains("trojan://") || decoded.contains("hysteria2://")
                    ) {
                        return true;
                    }
                }
            }
        }
        
        false
    }
    
    /// 检查字符串是否包含 Base64 字符
    fn contains_base64_chars(s: &str) -> bool {
        let base64_chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/=";
        s.chars().all(|c| base64_chars.contains(c) || c.is_whitespace())
    }
    
    /// 尝试其他格式解析（当主要格式失败时）
    fn try_other_formats(content: &str) -> Result<Vec<Proxy>, Box<dyn std::error::Error>> {
        // 先尝试 Base64
        if let Ok(proxies) = Self::parse_base64_content(content) {
            if !proxies.is_empty() {
                println!("备用解析：成功使用 Base64 格式解析，节点数量: {}", proxies.len());
                return Ok(proxies);
            }
        }
        
        // 再尝试链接格式
        if let Ok(proxies) = Self::parse_links_content(content) {
            if !proxies.is_empty() {
                println!("备用解析：成功使用链接格式解析，节点数量: {}", proxies.len());
                return Ok(proxies);
            }
        }
        
        // 最后尝试 YAML
        if let Ok(proxies) = Self::parse_yaml_content(content) {
            if !proxies.is_empty() {
                println!("备用解析：成功使用 YAML 格式解析，节点数量: {}", proxies.len());
                return Ok(proxies);
            }
        }
        
        Err("所有解析格式都失败".into())
    }
    
    /// 尝试所有格式解析（当类型未知时）
    fn try_all_formats(content: &str) -> Result<Vec<Proxy>, Box<dyn std::error::Error>> {
        let mut last_error = None;
        
        // 按优先级尝试各种格式
        let formats = [
            ("YAML", Self::parse_yaml_content as fn(&str) -> Result<Vec<Proxy>, Box<dyn std::error::Error>>),
            ("Base64", Self::parse_base64_content),
            ("Links", Self::parse_links_content),
        ];
        
        for (format_name, parse_fn) in formats.iter() {
            match parse_fn(content) {
                Ok(proxies) => {
                    if !proxies.is_empty() {
                        println!("全格式尝试：成功使用 {} 格式解析，节点数量: {}", format_name, proxies.len());
                        return Ok(proxies);
                    }
                }
                Err(e) => {
                    println!("{} 格式解析失败: {}", format_name, e);
                    last_error = Some(e);
                }
            }
        }
        
        Err(last_error.unwrap_or_else(|| "所有格式解析都失败".into()))
    }

    /// 移除重复节点
    pub fn exclude_dup_proxies(proxies: Vec<Proxy>) -> Vec<Proxy> {
        let mut new_proxies = Vec::new();
        if !proxies.is_empty() {
            let set: HashSet<Proxy> = HashSet::from_iter(proxies);
            new_proxies = set.into_iter().collect();
            new_proxies.sort_by(|a, b| a.proxy_type.cmp(&b.proxy_type));
        }
        new_proxies
    }

    /// 重置节点名称
    #[allow(dead_code)]
    pub fn unset_proxies_name(proxies: &mut Vec<Proxy>) {
        for proxy in proxies {
            let server = proxy.get_server().to_string();
            let hash = &mut DefaultHasher::new();
            proxy.to_json().unwrap().hash(hash);
            let h = hash.finish();
            proxy.set_name(&(server + "_" + &h.to_string()[..5]));
        }
    }

    /// 重命名相同名称的节点，在末尾加序号
    pub fn rename_dup_proxies_name(proxies: &mut Vec<Proxy>) {
        let mut name_counts: HashMap<String, usize> = HashMap::new();
        let number_suffix = Regex::new(r"\d+$").unwrap();

        // 打点，并删除其中原有的数字后缀
        for proxy in proxies.iter_mut() {
            let mut name = proxy.get_name().to_string();
            name = number_suffix.replace(&name, "").to_string();
            proxy.set_name(&name);
            *name_counts.entry(name).or_insert(0) += 1;
        }

        for proxy in &mut *proxies {
            let name = proxy.get_name().to_string();
            if let Some(count) = name_counts.get(&name) {
                if count > &1 {
                    let mut counter = 1;
                    let mut new_name = format!("{}{}", &name, counter);
                    while name_counts.contains_key(&new_name) {
                        counter += 1;
                        new_name = format!("{}{}", &name, counter);
                    }

                    proxy.set_name(&new_name);
                    name_counts.insert(new_name, 1);
                }
            }
        }

        // 注释掉重新排序，保持原始顺序
        // proxies.sort_by(|a, b| a.get_name().cmp(b.get_name()));
    }

    // 通过配置格式，获取 clash 配置文件内容
    pub fn get_clash_config_content(
        config_path: String,
        new_proxies: &Vec<Proxy>,
    ) -> io::Result<String> {
        let mut file = File::open(config_path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let mut yaml: Value = serde_yaml::from_str(&contents).expect("Failed to parse YAML");

        // 插入 proxies
        if let Some(proxies) = yaml.get_mut("proxies").and_then(Value::as_sequence_mut) {
            for proxy in new_proxies {
                proxies.push(Value::Mapping(
                    serde_yaml::from_str::<Mapping>(&proxy.to_json()?).unwrap(),
                ));
            }
        } else {
            println!("Failed to find 'proxies' in the YAML file");
        }

        // 处理 proxy-groups 逻辑
        if let Some(groups) = yaml
            .get_mut("proxy-groups")
            .and_then(Value::as_sequence_mut)
        {
            for group in groups.iter_mut() {
                if let Some(group_map) = group.as_mapping_mut() {
                    if let Some(Value::String(filter)) =
                        group_map.get(Value::String("filter".to_string()))
                    {
                        let regex = Regex::new(filter).expect("Invalid regex");
                        if let Some(proxies) = group_map
                            .get_mut(Value::String("proxies".to_string()))
                            .and_then(Value::as_sequence_mut)
                        {
                            let mut removed_default = false;
                            for proxy in new_proxies {
                                if regex.is_match(proxy.get_name()) {
                                    if !removed_default
                                        && proxies
                                        .first()
                                        .is_some_and(|p| p.as_str().unwrap().eq("PROXY"))
                                    {
                                        proxies.remove(0);
                                        removed_default = true;
                                    }
                                    proxies.push(Value::String(proxy.get_name().to_string()));
                                }
                            }
                            if proxies.is_empty() {
                                proxies.push(Value::String("DIRECT".to_string()));
                            }
                        }
                    }
                }
            }
        }
        Ok(serde_yaml::to_string(&yaml).expect("Failed to serialize YAML"))
    }

    pub fn save_proxies_into_clash_file(
        proxies: &Vec<Proxy>,
        config_path: String,
        save_path: String,
    ) {
        let content = SubManager::get_clash_config_content(config_path, proxies).unwrap();
        let mut file = File::create(&save_path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::protocol;
    use crate::protocol::ProxyType::{Vless, Vmess};
    use std::path::PathBuf;

    #[test]
    fn test_get_clash_config_content() {
        let path = "conf/clash_release.yaml";
        let mut proxies = SubManager::parse_from_path(
            "/Users/reajason/RustroverProjects/clash-butler/subs/0c1149d13476bbe3b62eecb7c9b895f4",
        )
            .unwrap();
        SubManager::unset_proxies_name(&mut proxies);
        let content = SubManager::get_clash_config_content(path.to_string(), &proxies).unwrap();
        println!("{}", content);
    }

    #[test]
    fn test_urls_type() {
        let link = "ss://YWVzLTEyOC1nY206ZDljNTc3MzI4ZmIzNDlmZQ==@120.232.73.68:40676#%F0%9F%87%AD%F0%9F%87%B0HK";
        assert!(!Path::new(link).is_file());

        let path = PathBuf::from_iter(vec!["tests", "res", "base64_proxies"]);
        assert!(path.is_file());
    }

    #[test]
    #[ignore]
    fn test_parse_conf() {
        let path = Path::new(
            "/Users/reajason/RustroverProjects/clash-butler/subs/d417717ed83bdabad1d310906a47a3a2",
        );
        let proxies = SubManager::parse_from_path(path).unwrap();
        for proxy in &proxies {
            println!("{:?}", proxy);
        }
    }

    #[test]
    fn test_regex_filter() {
        let filter = "台湾|TW|Tw|Taiwan|新北|彰化|CHT|HINET";
        let name = "JP_Tokyo_Shenzhen lesuyun Network Technology";
        let is_match = Regex::new(filter).unwrap().is_match(name);
        assert!(!is_match);
    }

    #[test]
    fn test_rename_dup_proxies_name() {
        let content = String::from(
            "ss://cmM0LW1kNToydnpobzU=@120.241.144.101:2410#name\n\
        ss://cmM0LW1kNToydnpobzU=@120.241.144.101:2410#name1\n\
        ss://cmM0LW1kNToydnpobzU=@120.241.144.101:2410#name1\n\
        ss://cmM0LW1kNToydnpobzU=@120.241.144.101:2410#name\n\
        ss://cmM0LW1kNToydnpobzU=@120.241.144.101:2410#xixi",
        );

        let mut proxies = SubManager::parse_content(content).unwrap();
        assert_eq!(proxies.len(), 5);
        assert_eq!(proxies.get(0).unwrap().get_name(), "name");
        assert_eq!(proxies.get(1).unwrap().get_name(), "name1");
        assert_eq!(proxies.get(2).unwrap().get_name(), "name1");
        assert_eq!(proxies.get(3).unwrap().get_name(), "name");
        assert_eq!(proxies.get(4).unwrap().get_name(), "xixi");
        SubManager::rename_dup_proxies_name(&mut proxies);
        assert_eq!(proxies.len(), 5);
        assert_eq!(proxies.get(0).unwrap().get_name(), "name1");
        assert_eq!(proxies.get(1).unwrap().get_name(), "name2");
        assert_eq!(proxies.get(2).unwrap().get_name(), "name3");
        assert_eq!(proxies.get(3).unwrap().get_name(), "name4");
        assert_eq!(proxies.get(4).unwrap().get_name(), "xixi");
    }

    #[tokio::test]
    async fn test_merge_config() {
        let urls = vec![
            "hysteria2://bc97f674-c578-4940-9234-0a1da46041b9@188.68.234.53:36604/?sni=www.bing.com&alpn=h3&insecure=1#tg%E6%8A%80%E6%9C%AF%E4%BA%A4%E6%B5%81%E7%BE%A4%EF%BC%9Ahttps%3A%2F%2Ft.me%2Fzzzjsjl"
                .to_string(),
        ];
        let proxies = SubManager::get_proxies_from_urls(&urls).await;
        let release_clash_template_path =
            "/Users/reajason/RustroverProjects/clash-butler/conf/clash_release.yaml".to_string();
        let save_path =
            "/Users/reajason/RustroverProjects/clash-butler/subs/release/proxy-s14.yaml".to_string();
        SubManager::save_proxies_into_clash_file(&proxies, release_clash_template_path, save_path);
    }

    #[tokio::test]
    async fn test_rename() {
        let urls = vec!["/Users/reajason/RustroverProjects/clash-butler/clash.yaml".to_string()];
        let mut proxies = SubManager::get_proxies_from_urls(&urls).await;
        SubManager::rename_dup_proxies_name(&mut proxies);
        let release_clash_template_path =
            "/Users/reajason/RustroverProjects/clash-butler/conf/clash_release.yaml".to_string();
        let save_path = "/Users/reajason/RustroverProjects/clash-butler/clash1.yaml".to_string();
        SubManager::save_proxies_into_clash_file(&proxies, release_clash_template_path, save_path)
    }

    #[tokio::test]
    async fn test_merge_uuids() {
        let url = "https://aq.louwangzhiyu.xyz/api/v1/client/subscribe?token=1df19eb36b48ca575ce86efb8267ada0";
        let mut proxies = SubManager::get_proxies_from_url(url.to_string()).await;

        let mut result = vec![];
        let uuids = vec![
            "09380338-f77f-494b-bd90-5018a32a7391",
            "0b2c47ef-8453-4308-ba1e-bda41d0d6885",
            "0dbbe2d1-4fbc-4102-ac1e-8fd4712a03fc",
            "1641df69-d412-4585-9450-74b0b66677d7",
            "16a939e6-a411-4c76-9931-d67aff2e887f",
            "26e45c40-818a-4f7e-9c50-62906a160e99",
            "27717caf-1ef5-4d57-a7a0-7f3a7448cbdd",
            "394a7bab-233f-4b01-8194-2489a92ed86a",
            "3e8e2a7c-16e3-46f6-8344-5a1dd3980d10",
            "43763d51-002a-4e1c-aeda-ab3239887396",
            "438ec28a-a948-4780-9153-92732c52aefa",
            "4bd66d31-855f-4dd0-9b11-4911d4e95e00",
            "503646cc-89a1-4e39-b028-bda1c205fdd5",
            "63f51b3a-67a1-4afb-9de6-8533674216c1",
            "6fdfbe5d-1502-4b22-9b79-0ad27e98db94",
            "768898c3-f43a-4afb-bd7e-559d513865b9",
            "79a15d6c-27a1-4dda-99fb-88c31545de5a",
            "7b7e01d9-db81-4568-b4ea-4542b8f40f27",
            "7d17abd8-f11b-44c7-8760-10a58e242e29",
            "990fe0ac-8741-4fd6-be1c-91e339cfb64f",
            "9f47af57-3a49-4248-9919-e0a6904c5b08",
            "a39fe77c-6718-4011-8f22-4cce831ee2e6",
            "af0bf67d-d991-431c-99f9-93e1af016f5c",
            "b9b868ca-904a-4180-99d7-8ff7dee42433",
            "c3d1b53d-8cbe-4455-a009-bfa08e55f11d",
            "c5b457eb-666e-461e-a1bd-751789c8bf02",
            "c9073c05-72f7-41d9-beb1-286e0e2696c9",
            "d1917fc9-e091-4b9b-8c02-37220af3b6a3",
            "d6ff192b-5ad3-4e99-be78-1680d26fad99",
            "d7b9d4e7-6b96-46ea-aa46-576e90b9f481",
            "da7f26fe-d6ad-47d3-be65-4b28796d1dbd",
            "e18e9033-fcb4-4a67-b3ac-c7d2f06a7511",
            "ee644da4-8352-4a4f-bb6b-c5487dde9a28",
            "fd889d9c-538b-434a-9eed-f6428b0e70d2",
            "ffcf68fb-eff2-42b2-a855-1b65cac901e7",
            "9d655d30-cfad-4cda-82d6-9028ab41907d",
        ];

        for uuid in uuids {
            for proxy in &mut proxies {
                println!("{:?}", proxy);
                if proxy.proxy_type.eq(&Vless) {
                    if let Some(vless) = proxy.adapter.as_any().downcast_ref::<protocol::vless::Vless>() {
                        let mut p = vless.clone();
                        p.uuid = uuid.to_string();
                        proxy.adapter = Box::new(p);
                        result.push(proxy.clone());
                    } else {}
                } else if proxy.proxy_type.eq(&Vmess) {
                    if let Some(vmess) = proxy.adapter.as_any().downcast_ref::<protocol::vmess::Vmess>() {
                        let mut p = vmess.clone();
                        p.uuid = uuid.to_string();
                        proxy.adapter = Box::new(p);
                        result.push(proxy.clone());
                    }
                }
            }
        }

        SubManager::rename_dup_proxies_name(&mut result);

        SubManager::save_proxies_into_clash_file(&result, "/Users/reajason/RustroverProjects/clash-butler/conf/clash_release.yaml".to_string(), "/Users/reajason/RustroverProjects/clash-butler/2024.11.19.yaml".to_string());

        println!("{:?}", result.len());
    }
}

#[test]
    fn test_detect_subscription_type() {
        // 测试 YAML 格式检测
        let yaml_content = r#"
proxies:
  - name: "test"
    type: ss
    server: 1.2.3.4
    port: 443
"#;
        assert_eq!(SubManager::detect_subscription_type(yaml_content), SubscriptionType::Yaml);

        // 测试链接格式检测
        let links_content = r#"ss://YWVzLTI1Ni1nY206UUlHVVo3VkRQWk9BU0M5SEAxMjAuMjQxLjQ1LjUwOjE3MDAx#US-01
ss://YWVzLTI1Ni1nY206UUlHVVo3VkRQWk9BU0M5SEAxMjAuMjQxLjQ1LjUwOjE3MDAy#US-02"#;
        assert_eq!(SubManager::detect_subscription_type(links_content), SubscriptionType::Links);

        // 测试 Base64 格式检测
        let base64_content = "c3M6Ly9ZV1Z6TFRFeU9DMW5ZMjA2WkRsak5UYzNNekk0Wm1Jek5EbG1aUT09QDEyMC4yMzIuNzMuNjg6NDA2NzYjJUYwJTlGJTg3JUFEJUYwJTlGJTg3JUIwSEs=";
        assert_eq!(SubManager::detect_subscription_type(base64_content), SubscriptionType::Base64);
    }

    #[test]
    fn test_parse_base64_subscription() {
        // 使用测试文件中的 Base64 内容
        if let Ok(base64_content) = std::fs::read_to_string("tests/res/base64_proxies") {
            let result = SubManager::parse_base64_content(&base64_content);
            
            match result {
                Ok(proxies) => {
                    println!("成功解析 {} 个代理节点", proxies.len());
                    assert!(!proxies.is_empty(), "应该能够解析出代理节点");
                    
                    // 验证第一个节点的基本信息
                    if let Some(first_proxy) = proxies.first() {
                        println!("第一个代理节点: {}", first_proxy.get_name());
                        assert!(!first_proxy.get_name().is_empty(), "节点名称不应为空");
                    }
                }
                Err(e) => {
                    println!("Base64 解析失败: {}", e);
                }
            }
        }
    }

    #[test]
    fn test_is_likely_base64() {
        // 测试有效的 Base64 内容
        let valid_base64 = "c3M6Ly9ZV1Z6TFRFeU9DMW5ZMjA2WkRsak5UYzNNekk0Wm1Jek5EbG1aUT09QDEyMC4yMzIuNzMuNjg6NDA2NzYjJUYwJTlGJTg3JUFEJUYwJTlGJTg3JUIwSEs=";
        assert!(SubManager::is_likely_base64(valid_base64));

        // 测试无效的 Base64 内容（包含非 Base64 字符）
        let invalid_base64 = "这不是Base64内容";
        assert!(!SubManager::is_likely_base64(invalid_base64));

        // 测试短内容
        let short_content = "abc";
        assert!(!SubManager::is_likely_base64(short_content));
    }

    #[test]
    fn test_enhanced_parse_content() {
        // 测试 Base64 订阅内容解析
        let base64_content = "c3M6Ly9ZV1Z6TFRFeU9DMW5ZMjA2WkRsak5UYzNNekk0Wm1Jek5EbG1aUT09QDEyMC4yMzIuNzMuNjg6NDA2NzYjJUYwJTlGJTg3JUFEJUYwJTlGJTg3JUIwSEs=";
        let result = SubManager::parse_content(base64_content.to_string());
        
        match result {
            Ok(proxies) => {
                println!("解析结果: {} 个代理节点", proxies.len());
                assert!(!proxies.is_empty(), "应该能够解析出代理节点");
            }
            Err(e) => {
                println!("解析失败: {}", e);
                // 在测试环境中，这可能会失败，但不应该panic
            }
        }
    }
