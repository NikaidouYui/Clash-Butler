#![allow(dead_code)]

use std::collections::HashMap;
use std::process::Stdio;
use tokio::process::{Child, Command};
use std::time::Duration;

use reqwest::Client;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use serde_json::Value;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::time::sleep;
use tracing::{error, info};

pub struct ClashMeta {
    pub external_port: u64,
    pub mixed_port: u64,
    pub proxy_url: String,
    pub external_url: String,
    core_path: String,
    test_path: String,
    log_path: String,
    process: Option<Child>,
}

impl ClashMeta {
    pub fn new(external_port: u64, mixed_port: u64) -> Self {
        ClashMeta {
            external_port,
            mixed_port,
            external_url: format!("http://127.0.0.1:{}", external_port),
            proxy_url: format!("http://127.0.0.1:{}", mixed_port),
            process: None,
            core_path: if cfg!(target_os = "windows") {
                "clash-meta/mihomo.exe".to_string()
            } else {
                "clash-meta/mihomo".to_string()
            },
            test_path: "subs/test".to_string(),
            log_path: "logs/clash.log".to_string(),
        }
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut log_file = File::create(&self.log_path).await?;

        let mut clash_process = Command::new(&self.core_path)
            .arg("-d")
            .arg(&self.test_path)
            .stdout(Stdio::piped())
            .spawn()?;
        let stdout = clash_process.stdout.take().unwrap();

        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            while reader.read_line(&mut line).await.unwrap() > 0 {
                if !line.contains("跳过非代理内容") {
                    log_file.write_all(line.as_bytes()).await.unwrap();
                }
                line.clear();
            }
        });

        sleep(Duration::from_secs(2)).await;

        let response = reqwest::get(format!("{}/version", &self.external_url)).await?;
        let res = response.json::<ClashVersion>().await?;
        info!("原神启动！ 版本号：{}", res.version);
        self.process = Some(clash_process);
        Ok(())
    }

    pub async fn restart(&self) -> Result<(), Box<dyn std::error::Error>> {
        let client = Client::builder().timeout(Duration::from_secs(5)).build()?;
        let response = client
            .post(format!("{}/restart", &self.external_url))
            .json(&json!({"path": self.test_path,"payload": ""}))
            .send()
            .await?;

        if response.status().is_success() {
            info!("内核重启成功");
            sleep(Duration::from_secs(2)).await;
        } else {
            info!("内核重启失败: {}", response.status());
        }
        Ok(())
    }

    pub async fn stop(mut self) -> std::io::Result<()> {
        if let Some(mut process) = self.process.take() {
            process.kill().await?;
            process.wait().await?;
        }
        Ok(())
    }

    pub async fn get_group(&self, group_name: &str) -> Result<Group, Box<dyn std::error::Error>> {
        let url = format!("{}/group/{}", &self.external_url, group_name);
        let client = Client::builder().timeout(Duration::from_secs(5)).build()?;
        let response = client.get(url).send().await?;
        let group = response.json::<Group>().await?;
        Ok(group)
    }

    pub async fn test_group(
        &self,
        group_name: &str,
        delay_test_config: &DelayTestConfig,
    ) -> Result<HashMap<String, i64>, Box<dyn std::error::Error>> {
        let url = format!("{}/group/{}/delay", &self.external_url, group_name);
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap();
        let response = client.get(&url).query(&delay_test_config).send().await?;
        if !response.status().is_success() {
            return Err(Box::from("获取分组延迟失败".to_string()));
        }
        let res: Value = response.json().await?;
        match res {
            Value::Object(map) => {
                let msg = map.get("message");
                if msg.is_some() {
                    let msg = msg.unwrap();
                    Err(Box::from(msg.to_string()))
                } else {
                    let mut result = HashMap::new();
                    for (name, value) in map {
                        if let Some(num) = value.as_i64() {
                            result.insert(name.clone(), num);
                        }
                    }
                    Ok(result)
                }
            }
            _ => Err(Box::from("所有节点无速度")),
        }
    }

    pub async fn test_proxy(
        &self,
        proxy_name: &str,
        delay_test_config: &DelayTestConfig,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let url = format!("{}/proxies/{}/delay", &self.external_url, proxy_name);
        let client = Client::builder().timeout(Duration::from_secs(60)).build()?;
        let response = client.get(&url).query(delay_test_config).send().await?;
        if !response.status().is_success() {
            return Err(Box::from("获取分组延迟失败".to_string()));
        }
        Ok(response.json::<ProxyDelay>().await?.delay)
    }

    pub async fn test_direct_delay(&self) -> Result<u64, Box<dyn std::error::Error>> {
        self.test_proxy(
            "DIRECT",
            &DelayTestConfig {
                url: "http://www.gstatic.com/generate_204".to_string(),
                expected: Some(204),
                timeout: 200,
            },
        )
        .await
    }

    pub async fn set_group_proxy(
        &self,
        group_name: &str,
        proxy_name: &str,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let url = format!("{}/proxies/{}", &self.external_url, group_name);
        let client = Client::builder().timeout(Duration::from_secs(10)).build()?;
        
        info!("尝试切换代理组 {} 到节点: {}", group_name, proxy_name);
        
        let response = client
            .put(&url)
            .json(&json!({"name": proxy_name}))
            .send()
            .await?;
            
        if response.status().is_success() {
            info!("代理切换请求成功: {} -> {}", group_name, proxy_name);
            
            // 等待一段时间让切换生效
            sleep(Duration::from_millis(1000)).await;
            
            // 验证切换是否成功
            match self.get_group(group_name).await {
                Ok(group) => {
                    if group.now == proxy_name {
                        info!("代理切换验证成功: 当前使用 {}", group.now);
                        Ok(true)
                    } else {
                        error!("代理切换验证失败: 期望 {}, 实际 {}", proxy_name, group.now);
                        Ok(false)
                    }
                }
                Err(e) => {
                    error!("无法验证代理切换状态: {}", e);
                    Ok(false)
                }
            }
        } else {
            error!("代理切换请求失败: HTTP {}", response.status());
            Ok(false)
        }
    }
}

#[derive(Deserialize, Debug)]
#[allow(unused)]
struct ClashVersion {
    meta: bool,
    version: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProxyDelay {
    pub delay: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(unused)]
pub struct DelayTestConfig {
    pub url: String,
    pub expected: Option<u16>,
    pub timeout: u16,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(unused)]
pub struct Group {
    pub all: Vec<String>,
    pub now: String,
    pub name: String,
}

#[cfg(test)]
mod tests {
    use crate::clash::ClashMeta;
    use crate::clash::DelayTestConfig;

    #[tokio::test]
    async fn test_proxy_delay() {
        let clash_meta = ClashMeta::new(9091, 7891);
        let delay = clash_meta
            .test_proxy(
                "DIRECT",
                &DelayTestConfig {
                    url: "http://www.gstatic.com/generate_204".to_string(),
                    expected: Some(204),
                    timeout: 500,
                },
            )
            .await
            .unwrap();
        println!("{}", delay);
    }

    #[tokio::test]
    async fn test_group_proxies() {
        let clash_meta = ClashMeta::new(9091, 7999);
        let result = clash_meta.get_group("PROXY").await;
        println!("{:?}", result);
    }

    #[tokio::test]
    async fn test_set_group_node() {
        let clash_meta = ClashMeta::new(9091, 7999);
        let result = clash_meta
            .set_group_proxy("PROXY", "None_None_vmess_044")
            .await;
        if result.is_ok() {
            println!("success")
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_group_delay() {
        let clash_meta = ClashMeta::new(9091, 7890);
        let result = clash_meta
            .test_group(
                "PROXY",
                &DelayTestConfig {
                    url: "http://www.google.com/generate_204".to_string(),
                    expected: Some(204),
                    timeout: 1000,
                },
            )
            .await;

        println!("{:?}", result);
    }
}
