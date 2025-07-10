# Clash-Butler 配置指南

本文档详细说明了 `Clash-Butler` 的 `config.toml` 配置文件中的所有可用选项。

## 全局配置

这些是位于配置文件顶层的全局设置。

---

### `fast_mode`

-   **描述:** 是否开启快速模式。在快速模式下，程序仅对代理节点进行基础的连通性测试，而不会进行带宽测速。这可以显著加快筛选过程。
-   **类型:** `Boolean`
-   **默认值:** `false` (在 `config.example.toml` 中指定)
-   **示例:**
    ```toml
    fast_mode = true
    ```

---

### `debug_mode`

-   **描述:** 是否开启调试模式。开启后，程序会输出更详细的日志信息，便于排查问题。
-   **类型:** `Boolean`
-   **默认值:** `true` (在 `config.example.toml` 中指定)
-   **示例:**
    ```toml
    debug_mode = false
    ```

---

### `subs`

-   **描述:** 需要进行测试和筛选的订阅链接列表。
-   **类型:** `Array of Strings`
-   **支持格式:**
    -   网络地址 (e.g., `https://...`)
    -   本地文件绝对路径 (e.g., `/path/to/your/sub.yaml`)
    -   单个代理节点链接 (e.g., `ss://...`)
-   **默认值:** 空列表 `[]`
-   **示例:**
    ```toml
    subs = [
        "https://your-subscription-url-here.com/sub",
        "/Users/me/subs/local_config.yaml"
    ]
    ```

---

### `rename_node`

-   **描述:** 是否对筛选出的代理节点进行重命名。开启后，程序会尝试通过查询节点的真实IP和地理位置信息来生成新的名称。
-   **类型:** `Boolean`
-   **默认值:** `true` (在 `config.example.toml` 中指定)
-   **示例:**
    ```toml
    rename_node = true
    ```

---

### `rename_pattern`

-   **描述:** 定义节点重命名的格式模板。仅在 `rename_node` 为 `true` 时生效。
-   **类型:** `String`
-   **可用变量:**
    -   `${COUNTRYCODE}`: 国家/地区代码 (e.g., US, JP)
    -   `${CITY}`: 城市名 (e.g., Los Angeles)
    -   `${ISP}`: 互联网服务提供商
-   **默认值:** `"${COUNTRYCODE}_${CITY}_${ISP}"`
-   **示例:**
    ```toml
    # 生成类似 "US-LosAngeles-Gcore" 的名称
    rename_pattern = "${COUNTRYCODE}-${CITY}-${ISP}"
    ```

---

### `need_add_pool`

-   **描述:** 是否将公共的代理节点池与您的订阅链接合并，一同进行筛选。
-   **类型:** `Boolean`
-   **默认值:** `true` (在 `config.example.toml` 中指定)
-   **示例:**
    ```toml
    need_add_pool = false
    ```

---

### `pools`

-   **描述:** 公共代理节点池的订阅链接列表。仅在 `need_add_pool` 为 `true` 时生效。
-   **类型:** `Array of Strings`
-   **默认值:** 包含两个预设链接 (参考 `config.example.toml`)
-   **示例:**
    ```toml
    pools = [
        "https://raw.githubusercontent.com/Ruk1ng001/freeSub/main/clash.yaml"
    ]
    ```

---

### `test_group_size`

-   **描述:** 在进行并发测试时，每一组包含的代理节点数量。
-   **类型:** `Integer`
-   **默认值:** `50` (在 `config.example.toml` 中指定)
-   **示例:**
    ```toml
    test_group_size = 100
    ```

---

## `[connect_test]`

此部分配置用于连通性测试的参数。

### `url`

-   **描述:** 用于测试节点是否可用的目标URL。程序会请求此URL并检查响应。
-   **类型:** `String`
-   **默认值:** `"http://www.google-analytics.com/generate_204"`
-   **示例:**
    ```toml
    [connect_test]
    url = "http://www.gstatic.com/generate_204"
    ```

### `expected`

-   **描述:** 预期的HTTP响应状态码。如果实际响应码与此值匹配，则认为连通性测试通过。
-   **类型:** `Integer`
-   **默认值:** `204`
-   **示例:**
    ```toml
    [connect_test]
    expected = 200
    ```

### `timeout`

-   **描述:** 连通性测试的超时时间。
-   **类型:** `Integer` (毫秒)
-   **默认值:** `2000`
-   **示例:**
    ```toml
    [connect_test]
    timeout = 5000 # 5秒
    ```

---

## `[speed_test]`

此部分配置用于带宽测速的参数。

### `enabled`

-   **描述:** 是否启用带宽测速功能。如果禁用，即使 `fast_mode` 为 `false`，也只进行连通性测试。
-   **类型:** `Boolean`
-   **默认值:** `true`
-   **示例:**
    ```toml
    [speed_test]
    enabled = false
    ```

### `url`

-   **描述:** 用于下载以测试带宽的文件的URL。建议使用一个稳定的大文件链接。
-   **类型:** `String`
-   **默认值:** `"https://speed.cloudflare.com/__down?bytes=10485760"` (10MB)
-   **示例:**
    ```toml
    [speed_test]
    url = "http://cachefly.cachefly.net/100mb.test"
    ```

### `timeout`

-   **描述:** 带宽测速的超时时间。
-   **类型:** `Integer` (毫秒)
-   **默认值:** `10000`
-   **示例:**
    ```toml
    [speed_test]
    timeout = 20000 # 20秒
    ```

### `min_speed_mbps`

-   **描述:** (此项未在示例配置中，但在代码中存在) 节点被认为是有效节点所需的最低下载速度（以Mbps为单位）。
-   **类型:** `Float`
-   **默认值:** `0.0` (代码中的默认值，意味着任何速度都可接受)
-   **示例:**
    ```toml
    [speed_test]
    min_speed_mbps = 5.5 # 节点速度必须高于 5.5 Mbps