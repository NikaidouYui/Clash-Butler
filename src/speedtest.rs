use std::time::Duration;
use std::time::Instant;

use futures_util::StreamExt;
use reqwest::Proxy;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Serialize, Deserialize)]
#[allow(unused)]
pub struct SpeedTestConfig {
    pub enabled: bool,
    pub url: String,
    pub timeout: u16,
    pub min_speed_mbps: f64,
}

/// 对指定代理进行带宽测速
pub async fn test_proxy_speed(
    url: &str,
    timeout: Duration,
    proxy_url: &str,
) -> Result<f64, Box<dyn std::error::Error>> {
    // 备用测速URL列表
    let backup_urls = vec![
        url,  // 使用配置中的URL
        "https://speed.cloudflare.com/__down?bytes=1048576",  // 1MB
        "https://speed.cloudflare.com/__down?bytes=524288",   // 512KB
        "http://speedtest.ftp.otenet.gr/files/test1Mb.db",    // 备用服务器
    ];
    
    let mut last_error = None;
    
    // 尝试不同的URL
    for test_url in backup_urls {
        // 每个URL尝试2次
        for attempt in 1..=2 {
            match test_download(test_url, timeout, Some(proxy_url)).await {
                Ok((_, bandwidth, _)) => {
                    // 如果速度合理（大于0且小于1GB/s），直接返回
                    if bandwidth > 0.0 && bandwidth < 1024.0 * 1024.0 {
                        return Ok(bandwidth);
                    }
                }
                Err(e) => {
                    last_error = Some(format!("URL: {}, Attempt: {}, Error: {}", test_url, attempt, e));
                    // 如果不是最后一次尝试，等待一下再重试
                    if attempt < 2 {
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                }
            }
        }
    }
    
    Err(format!("All speed test attempts failed. Last error: {:?}", last_error).into())
}

/// 格式化速度显示
pub fn format_speed(bandwidth_kbps: f64) -> String {
    let mbps = bandwidth_kbps / 1024.0;
    if mbps >= 1000.0 {
        format!("_{:.1}GB", mbps / 1024.0)
    } else if mbps >= 1.0 {
        format!("_{:.1}MB", mbps)
    } else {
        format!("_{:.0}KB", bandwidth_kbps)
    }
}

#[allow(dead_code)]
async fn test_download(
    url: &str,
    timeout: Duration,
    proxy_url: Option<&str>,
) -> Result<(Duration, f64, Duration), Box<dyn std::error::Error>> {
    let client_builder = reqwest::Client::builder()
        .timeout(timeout)
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36");

    let client = if let Some(proxy) = proxy_url {
        client_builder.proxy(Proxy::all(proxy).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?).build().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
    } else {
        client_builder.build().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
    };

    let start = Instant::now();
    let response = client.get(url).send().await.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    
    // 检查响应状态
    if !response.status().is_success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("HTTP error: {}", response.status())
        ).into());
    }

    // Stream the response body with better error handling
    let mut stream = response.bytes_stream();
    let mut total_bytes = 0;
    let mut first_byte_time = Duration::from_secs(0);
    let mut first_chunk = true;

    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                if first_chunk {
                    first_byte_time = start.elapsed();
                    first_chunk = false;
                }
                total_bytes += chunk.len();
            }
            Err(e) => {
                // 如果已经下载了一些数据，可以继续计算速度
                if total_bytes > 0 {
                    break;
                } else {
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, e).into());
                }
            }
        }
    }
    
    let total_duration = start.elapsed();
    
    // 确保有足够的数据和时间来计算带宽
    if total_bytes == 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "No data received"
        ).into());
    }
    
    if total_duration.as_secs_f64() < 0.001 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Download too fast to measure accurately"
        ).into());
    }
    
    let bandwidth = (total_bytes as f64 / 1024.0) / total_duration.as_secs_f64(); // KB per second
    Ok((total_duration, bandwidth, first_byte_time))
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_download() {
        let url = "https://speed.cloudflare.com/__down?bytes=1024"; // 1KB download for testing
        match crate::speedtest::test_download(
            url,
            Duration::from_secs(10),
            Some("http://127.0.0.1:7890"),
        )
        .await
        {
            Ok(result) => println!("{:?}", result),
            Err(e) => eprintln!("{:?}", e),
        }
    }

    #[test]
    fn test_format_speed() {
        // 测试不同速度的格式化
        assert_eq!(format_speed(500.0), "_500KB");
        assert_eq!(format_speed(1024.0), "_1.0MB");
        assert_eq!(format_speed(2048.0), "_2.0MB");
        assert_eq!(format_speed(1024.0 * 1024.0), "_1.0GB");
        assert_eq!(format_speed(1536.0), "_1.5MB");
    }
}

// #[tokio::main]
// async fn main() {
//     let url = "https://speed.cloudflare.com/__down?bytes=104857600";  // 100MB download
//     let proxy_url = "http://127.0.0.1:7890";
//     let result = test_download(url, proxy_url).await.unwrap();
//     println!("{:?}", result);
//
//
//     // let handles = (0..5).map(|_| {
//     //     tokio::spawn(async move {
//     //         test_download(url, proxy_url).await.unwrap()
//     //     })
//     // });
//     //
//     // let mut total_ttfb: Duration = Duration::new(0, 0);
//     // let mut total_bandwidth = 0.0;
//     // let mut results = Vec::new();
//     // for handle in handles {
//     //     let result = handle.await.unwrap();
//     //     results.push(result);
//     //     total_ttfb += result.2;
//     //     total_bandwidth += result.1;
//     // }
//     //
//     // let avg_ttfb = total_ttfb / results.len() as u32;
//     // let avg_bandwidth = total_bandwidth / results.len() as f64;
//
//     // println!("Average TTFB: {:.2?} seconds, Average Bandwidth: {:.2} bytes/sec", avg_ttfb,
// avg_bandwidth); }
