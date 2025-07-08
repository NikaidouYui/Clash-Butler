use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::time::Duration;

use clap::Parser;
use proxrs::protocol::Proxy;
use proxrs::sub::SubManager;
use tracing::error;
use tracing::info;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

use crate::clash::ClashMeta;
use crate::clash::DelayTestConfig;
use crate::settings::Settings;

mod cgi_trace;
mod clash;
mod ip;
mod risk;
mod routes;
mod server;
mod settings;
mod speedtest;
mod website;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    // Starts the Axum server
    #[arg(long)]
    server: bool,
}

const TEST_PROXY_GROUP_NAME: &str = "PROXY";

#[tokio::main]
async fn main() {
    tracing::subscriber::set_global_default(
        FmtSubscriber::builder()
            .with_max_level(Level::INFO)
            .finish(),
    )
    .expect("setting default subscriber failed");
    let args = Cli::parse();
    let config = Settings::new();
    match config {
        Ok(config) => {
            // 创建订阅测试所用的目录结构
            create_folder();
            if args.server {
                // 服务端
                // server::start_server(config).await
            } else {
                // 本地生成
                run(config).await
            }
        }
        Err(e) => {
            error!("配置文件读取失败: {}", e)
        }
    }
}

async fn run(config: Settings) {
    let test_yaml_path = "subs/test/config.yaml";
    let test_all_yaml_path = "subs/test/all.yaml";
    let release_yaml_path = env::current_dir().unwrap().join("clash.yaml");
    let test_clash_template_path = "conf/clash_test.yaml";
    let release_clash_template_path = "conf/clash_release.yaml";
    let mut urls = config.subs;
    if config.need_add_pool {
        urls.extend(config.pools)
    }
    let test_proxies = SubManager::get_proxies_from_urls(&urls).await;
    info!("待测速节点个数：{}", &test_proxies.len());
    if test_proxies.is_empty() {
        error!("当前无可用的待测试订阅连接，请修改配置文件添加订阅链接或确保当前网络通顺");
        return;
    }

    // 全部保存一下节点信息
    SubManager::save_proxies_into_clash_file(
        &test_proxies,
        test_clash_template_path.to_string(),
        test_all_yaml_path.to_string(),
    );

    let chunk_size = config.test_group_size;
    let proxies_group: Vec<_> = test_proxies
        .chunks(chunk_size)
        .map(|p| p.to_vec())
        .collect();
    let group_size = proxies_group.len();
    if group_size > 1 {
        info!(
            "为加速测试速度，以 {} 为限制分为 {} 组测试",
            chunk_size,
            proxies_group.len()
        );
    }

    // 启动 Clash 内核
    let external_port = 9091;
    let mixed_port = 7999;
    let mut useful_proxies = Vec::new();
    for (index, proxies) in proxies_group.iter().enumerate() {
        if group_size > 1 {
            info!("正在测试第 {} 组", index + 1)
        }

        SubManager::save_proxies_into_clash_file(
            proxies,
            test_clash_template_path.to_string(),
            test_yaml_path.to_string(),
        );

        let mut clash_meta = ClashMeta::new(external_port, mixed_port);
        if let Err(e) = clash_meta.start().await {
            error!("原神启动失败，第一次启动可能会下载 geo 相关的文件，重新启动即可，打开 logs/clash.log，查看具体错误原因，{}", e);
            clash_meta.stop().unwrap();
            continue;
        }

        match clash_meta.get_group(TEST_PROXY_GROUP_NAME).await {
            Ok(nodes) => {
                info!(
                    "开始测试 subs/test/config.yaml 中节点的延迟速度，节点总数：{}",
                    nodes.all.len()
                )
            }
            Err(e) => {
                error!("获取节点数失败，请检查 clash 日志文件和 subs/test/config.yaml 生成的节点是否正确, {}", e);
                clash_meta.stop().unwrap();
                continue;
            }
        }

        info!("开始测试连通性");
        
        // 获取当前批次所有节点名称，用于对比测试结果
        let all_nodes_in_batch: Vec<String> = proxies.iter().map(|p| p.get_name().to_string()).collect();
        info!("当前批次节点总数：{}，节点列表（前10个）：{:?}",
              all_nodes_in_batch.len(),
              all_nodes_in_batch.iter().take(10).collect::<Vec<_>>());
        
        let delay_results = test_node_with_delay_config(&clash_meta, &config.connect_test, &all_nodes_in_batch).await;
        let nodes = get_all_tested_nodes(&delay_results);
        info!("连通性测试结果：{} 个节点可用", nodes.len());
        
        // 添加详细的调试信息
        if !nodes.is_empty() {
            info!("通过连通性测试的节点名称（前10个）:");
            for (i, node) in nodes.iter().take(10).enumerate() {
                info!("  {}. 「{}」", i + 1, node);
            }
            
            info!("当前批次原始节点名称（前10个）:");
            for (i, proxy) in proxies.iter().take(10).enumerate() {
                info!("  {}. 「{}」", i + 1, proxy.get_name());
            }
            
            let cur_useful_proxies = proxies
                .iter()
                .filter(|&proxy| {
                    let proxy_name = proxy.get_name().to_string();
                    let found = nodes.contains(&proxy_name);
                    if found {
                        info!("✅ 节点匹配成功: 「{}」", proxy_name);
                    }
                    found
                })
                .cloned()
                .collect::<Vec<Proxy>>();
            info!("cur_useful_proxies len: {}", &cur_useful_proxies.len());
            
            // 如果没有匹配的节点，显示更多调试信息
            if cur_useful_proxies.is_empty() && !nodes.is_empty() {
                error!("⚠️ 节点名称匹配失败！");
                error!("测试通过的节点名称:");
                for node in &nodes {
                    error!("  测试节点: 「{}」", node);
                }
                error!("原始代理节点名称:");
                for proxy in proxies {
                    error!("  原始节点: 「{}」", proxy.get_name());
                }
            }
            
            useful_proxies.extend(cur_useful_proxies);
            info!("useful_proxies len: {}", useful_proxies.len());
        }
        clash_meta.stop().unwrap();
    }

    if useful_proxies.is_empty() {
        error!("当前无可用节点，请尝试更换订阅节点或重试");
        return;
    } else {
        info!("当前总可用节点个数：{}", &useful_proxies.len());
    }

    // 先生成快速模式的配置文件（只测试连通性，保留更多节点）
    let fast_yaml_path = env::current_dir().unwrap().join("clash-fast.yaml");
    SubManager::save_proxies_into_clash_file(
        &useful_proxies,
        release_clash_template_path.to_string(),
        fast_yaml_path.to_string_lossy().to_string(),
    );
    info!("快速模式配置文件地址：{}", fast_yaml_path.to_string_lossy());
    info!("快速模式节点数量：{}", useful_proxies.len());

    // 如果启用了快速模式，同时也生成完整处理的配置文件
    if !config.fast_mode {
        let mut clash_meta = ClashMeta::new(external_port, mixed_port);
        SubManager::save_proxies_into_clash_file(
            &useful_proxies,
            test_clash_template_path.to_string(),
            test_yaml_path.to_string(),
        );

        if let Err(e) = clash_meta.start().await {
            error!("原神启动失败，第一次启动可能会下载 geo 相关的文件，重新启动即可，打开 logs/clash.log，查看具体错误原因，{}", e);
            clash_meta.stop().unwrap();
            return;
        }
        info!("当前节点个数为：{}", useful_proxies.len());

        let nodes = &mut useful_proxies
            .iter()
            .map(|p| p.get_name().to_string())
            .collect::<Vec<String>>();
        let mut node_rename_map: HashMap<String, String> = HashMap::new();
        if config.rename_node {
            if nodes.is_empty() {
                error!("当前无可用节点，请尝试更换订阅节点或重试");
                clash_meta.stop().unwrap();
                return;
            }
            
            // 使用 retain 方法安全地过滤节点，避免索引问题
            let nodes_to_process = nodes.clone();
            let mut processed_nodes = Vec::new();
            
            for node in &nodes_to_process {
                info!("正在处理节点: {}", node);
                
                // 设置代理节点（已包含验证逻辑）
                match clash_meta.set_group_proxy(TEST_PROXY_GROUP_NAME, node).await {
                    Ok(true) => {
                        info!("「{}」代理切换成功", node);
                        
                        // 额外等待确保代理完全生效
                        tokio::time::sleep(Duration::from_millis(2000)).await;
                        
                        // 获取IP地址
                        let ip_result = cgi_trace::get_ip(&clash_meta.proxy_url, config.debug_mode).await;
                        if ip_result.is_ok() {
                            let (proxy_ip, from) = ip_result.unwrap();
                            info!("「{}」ip: {} from: {}", node, proxy_ip, from);
                            
                            let mut openai_is_ok = false;
                        match website::openai_is_ok(&clash_meta.proxy_url, config.debug_mode).await {
                            Ok(_) => {
                                info!("「{}」 openai is ok", node);
                                openai_is_ok = true;
                            }
                            Err(err) => {
                                error!("「{}」 openai is not ok, {:#}", node, err)
                            }
                        }

                        let mut claude_is_ok = false;
                        match website::claude_is_ok(&clash_meta.proxy_url, config.debug_mode).await {
                            Ok(_) => {
                                info!("「{}」 claude is ok", node);
                                claude_is_ok = true;
                            }
                            Err(err) => {
                                error!("「{}」 claude is not ok, {:#}", node, err)
                            }
                        }

                        // 只要有一个服务可用就保留节点（放宽过滤条件）
                        if openai_is_ok || claude_is_ok {
                            processed_nodes.push(node.clone());
                            
                            let ip_detail_result =
                                ip::get_ip_detail(&proxy_ip, &clash_meta.proxy_url).await;
                            match ip_detail_result {
                                Ok(ip_detail) => {
                                    info!("{:?}", ip_detail);
                                    if config.rename_node {
                                        let mut new_name = config
                                            .rename_pattern
                                            .replace("${IP}", &proxy_ip.to_string())
                                            .replace("${COUNTRYCODE}", &ip_detail.country_code)
                                            .replace("${ISP}", &ip_detail.isp)
                                            .replace("${CITY}", &ip_detail.city);
                                        if openai_is_ok {
                                            new_name += "_OpenAI";
                                        }
                                        if claude_is_ok {
                                            new_name += "_Claude";
                                        }
                                        node_rename_map.insert(node.clone(), new_name);
                                    }
                                }
                                Err(e) => {
                                    error!("获取节点 {} 的 IP 信息失败, {}", node, e);
                                    // 即使获取IP信息失败，只要有服务可用就保留节点
                                    let mut new_name = proxy_ip.to_string();
                                    if openai_is_ok {
                                        new_name += "_OpenAI";
                                    }
                                    if claude_is_ok {
                                        new_name += "_Claude";
                                    }
                                    node_rename_map.insert(node.clone(), new_name);
                                }
                            }
                        } else {
                            error!("节点 {} OpenAI 和 Claude 都不可用，已过滤", node);
                        }
                    } else {
                        error!("获取节点 {} 的 IP 失败, 获取不到 IP 地址，可能节点已失效，已过滤", node);
                    }
                }
                Ok(false) => {
                    error!("「{}」代理切换失败，跳过该节点", node);
                }
                Err(e) => {
                    error!("「{}」设置代理时发生错误: {}", node, e);
                }
            }
            }
            
            // 更新节点列表为处理后的节点
            *nodes = processed_nodes;
            info!("节点处理完成，剩余可用节点数量: {}", nodes.len());
        }

        let mut release_proxies = useful_proxies
            .into_iter()
            .filter(|proxy: &Proxy| nodes.contains(&proxy.get_name().to_string()))
            .collect::<Vec<Proxy>>();

        if !node_rename_map.is_empty() {
            for proxy in &mut release_proxies {
                let name = if let Some(new_name) = node_rename_map.get(proxy.get_name()) {
                    new_name.clone()
                } else {
                    proxy.get_name().to_string()
                };
                proxy.set_name(&name);
            }
        }

        SubManager::rename_dup_proxies_name(&mut release_proxies);
        SubManager::save_proxies_into_clash_file(
            &release_proxies,
            release_clash_template_path.to_string(),
            release_yaml_path.to_string_lossy().to_string(),
        );
        info!("完整处理配置文件地址：{}", release_yaml_path.to_string_lossy());
        info!("完整处理节点数量：{}", release_proxies.len());
        clash_meta.stop().unwrap();
    } else {
        info!("快速模式已启用，跳过完整处理流程");
    }
}

#[allow(dead_code)]
fn get_top_node(test_results: &Vec<HashMap<String, i64>>) -> (String, i64) {
    let mut combined_data: HashMap<String, Vec<i64>> = HashMap::new();
    for test in test_results {
        for (node, latency) in test {
            combined_data
                .entry(node.clone())
                .or_default()
                .push(*latency);
        }
    }
    let node_stats: Vec<(String, i64)> = combined_data
        .clone()
        .into_iter()
        .map(|(node, latencies)| {
            let sum: i64 = latencies.iter().sum();
            let count = latencies.len() as i64;
            let mean = sum / count;
            (node, mean)
        })
        .collect();
    node_stats
        .into_iter()
        .min_by_key(|(_, mean)| *mean)
        .unwrap()
}

async fn test_node_with_delay_config(
    clash_meta: &ClashMeta,
    delay_test_config: &DelayTestConfig,
    all_nodes_in_batch: &Vec<String>,
) -> Vec<HashMap<String, i64>> {
    const ROUND: i32 = 5;
    info!("测试配置：{:?}", delay_test_config);
    let mut delay_results = vec![];

    // 预热 2 轮，DNS lookup
    info!("开始预热测试...");
    for i in 0..2 {
        info!("预热第 {} 轮", i + 1);
        let _ = clash_meta
            .test_group(TEST_PROXY_GROUP_NAME, delay_test_config)
            .await;
    }

    for n in 0..ROUND {
        info!("测试第 {} 轮", n + 1);
        let result = clash_meta
            .test_group(TEST_PROXY_GROUP_NAME, delay_test_config)
            .await;

        match result {
            Ok(delay) => {
                info!("第 {} 轮测试成功，有速度节点个数为：{}", n + 1, delay.len());
                if !delay.is_empty() {
                    // 显示前几个通过测试的节点
                    let sample_nodes: Vec<_> = delay.keys().take(5).collect();
                    info!("第 {} 轮通过测试的节点示例: {:?}", n + 1, sample_nodes);
                }
                
                // 找出本轮失败的节点
                let passed_nodes: std::collections::HashSet<String> = delay.keys().cloned().collect();
                let failed_nodes: Vec<String> = all_nodes_in_batch
                    .iter()
                    .filter(|node| !passed_nodes.contains(*node))
                    .cloned()
                    .collect();
                
                if !failed_nodes.is_empty() {
                    error!("第 {} 轮失败的节点数量：{}，失败节点（前10个）：{:?}",
                           n + 1, failed_nodes.len(),
                           failed_nodes.iter().take(10).collect::<Vec<_>>());
                }
                
                delay_results.push(delay.clone());
            }
            Err(e) => {
                error!("第 {} 轮测试失败: {}", n + 1, e);
            }
        }
    }
    
    info!("连通性测试完成，共 {} 轮有效结果", delay_results.len());
    delay_results
}

/*
获取所有已测速有过一次速度的节点
修改逻辑：要求节点至少在60%的测试轮次中通过测试，提高稳定性
 */
fn get_all_tested_nodes(test_results: &Vec<HashMap<String, i64>>) -> Vec<String> {
    if test_results.is_empty() {
        return Vec::new();
    }
    
    let total_rounds = test_results.len();
    let min_success_rounds = (total_rounds as f64 * 0.6).ceil() as usize; // 至少60%的轮次通过
    
    let mut node_success_count: HashMap<String, usize> = HashMap::new();
    
    // 统计每个节点在多少轮测试中通过
    for result in test_results {
        for key in result.keys() {
            *node_success_count.entry(key.clone()).or_insert(0) += 1;
        }
    }
    
    // 分离通过和失败的节点
    let mut stable_nodes = Vec::new();
    let mut failed_nodes = Vec::new();
    
    for (node, count) in node_success_count {
        if count >= min_success_rounds {
            info!("✅ 节点 「{}」 在 {}/{} 轮测试中通过，符合稳定性要求", node, count, total_rounds);
            stable_nodes.push(node);
        } else {
            error!("❌ 节点 「{}」 在 {}/{} 轮测试中通过，不符合稳定性要求（需要至少 {} 轮），已丢弃",
                   node, count, total_rounds, min_success_rounds);
            failed_nodes.push(node);
        }
    }
    
    info!("稳定性筛选结果：{} 轮测试中，要求至少 {} 轮通过", total_rounds, min_success_rounds);
    info!("✅ 通过筛选的节点数量: {}", stable_nodes.len());
    error!("❌ 被丢弃的节点数量: {}", failed_nodes.len());
    
    if !failed_nodes.is_empty() {
        error!("被丢弃的节点列表:");
        for (i, node) in failed_nodes.iter().enumerate() {
            error!("  {}. 「{}」", i + 1, node);
        }
    }
    
    stable_nodes
}

/*
获取测速稳定的节点
 */
#[allow(dead_code)]
fn get_stable_tested_nodes(test_results: &Vec<HashMap<String, i64>>) -> Vec<String> {
    // 合并所有测试数据
    let mut combined_data: HashMap<String, Vec<i64>> = HashMap::new();
    for test in test_results {
        for (node, latency) in test {
            combined_data
                .entry(node.clone())
                .or_default()
                .push(*latency);
        }
    }

    // 计算每个节点的平均延迟和标准差
    let mut node_stats: Vec<(String, f64)> = combined_data
        .clone()
        .into_iter()
        .filter_map(|(node, latencies)| {
            let sum: i64 = latencies.iter().sum();
            let count = latencies.len();
            if count <= combined_data.len() / 2 {
                None
            } else {
                let mean = sum as f64 / count as f64;
                Some((node, mean))
            }
        })
        .collect();

    // 根据平均延迟对稳定的节点进行排序
    node_stats.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    node_stats.into_iter().map(|(node, _)| node).collect()
}

// 创建目录
fn create_folder() {
    let logs_path = "logs";
    if !Path::new(logs_path).exists() {
        fs::create_dir(logs_path).unwrap()
    }

    let subs_path = "subs";
    if !Path::new(subs_path).exists() {
        fs::create_dir(subs_path).unwrap();
    }

    let test_path = "subs/test";
    if !Path::new(test_path).exists() {
        fs::create_dir(test_path).unwrap();
    }

    let release_path = "subs/release";
    if !Path::new(release_path).exists() {
        fs::create_dir(release_path).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_stable_nodes() {
        // [
        //     { "免费节点2": 829 },
        //     { "免费节点3": 815, "免费节点2": 945, "免费节点1": 838 },
        //     { "免费节点4": 835, "免费节点1": 850, "免费节点3": 819 },
        //     { "免费节点1": 844, "免费节点3": 830, "免费节点2": 856 },
        //     { "免费节点3": 857, "免费节点4": 796, "2": 911, "免费节点4": 816 },
        //     { "免费节点1": 895, "免费节点3": 863, "免费节点4": 829 },
        //     { "免费节点3": 837, "免费节点1": 809, "免费节点4": 849 },
        //     { "免费节点3": 849, "免费节点2": 904, "免费节点4": 892 }
        // ];

        // 假设这是从十组测试中收集的数据
        let test_data = vec![
            HashMap::from([
                ("node1".to_string(), 100),
                ("node2".to_string(), 200),
                ("node3".to_string(), 150),
            ]),
            HashMap::from([
                ("node1".to_string(), 110),
                ("node2".to_string(), 190),
                ("node3".to_string(), 160),
            ]),
            HashMap::from([("node1".to_string(), 120), ("node3".to_string(), 10000)]),
        ];

        println!("{:?}", get_top_node(&test_data));
    }

    #[test]
    fn test_rename_pattern() {
        let count = "${COUNTRYCODE}_${CITY}_${ISP}".matches('_').count();
        println!("{count}");
        let count = "HongKong_Jordan_VertexConnectivityLLC62"
            .matches('_')
            .count();
        println!("{count}")
    }
}
