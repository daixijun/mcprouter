# Design: migrate-dynamic-token-management

## Architecture Overview

本变更引入一个完整的动态 Bearer Token 管理系统,包含以下核心组件:

````text
┌─────────────────────────────────────────────────────────────┐
│                       Frontend (React)                       │
│  ┌────────────────┐  ┌─────────────────┐  ┌──────────────┐  │
│  │ Token List     │  │ Create Dialog   │  │ Delete       │  │
│  │ Component      │  │                 │  │ Confirmation │  │
│  └────────┬───────┘  └────────┬────────┘  └──────┬───────┘  │
│           │                   │                   │          │
└───────────┼───────────────────┼───────────────────┼──────────┘
            │                   │                   │
            ▼                   ▼                   ▼
┌─────────────────────────────────────────────────────────────┐
│                    Tauri Commands (Rust)                     │
│  ┌────────────────┐  ┌─────────────────┐  ┌──────────────┐  │
│  │ list_tokens    │  │ create_token    │  │ delete_token │  │
│  └────────┬───────┘  └────────┬────────┘  └──────┬───────┘  │
│           │                   │                   │          │
└───────────┼───────────────────┼───────────────────┼──────────┘
            │                   │                   │
            ▼                   ▼                   ▼
┌─────────────────────────────────────────────────────────────┐
│                   Token Manager (Rust)                       │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  TokenManager                                        │   │
│  │  - tokens: Arc<RwLock<HashMap<String, Token>>>       │   │
│  │  - file_path: PathBuf (~/.mcprouter/tokens.json)     │   │
│  │                                                      │   │
│  │  Methods:                                            │   │
│  │  + create(name, desc, expires_in) -> Token           │   │
│  │  + list() -> Vec<Token>                              │   │
│  │  + delete(id) -> Result<()>                          │   │
│  │  + validate(token) -> Option<TokenId>                │   │
│  │  + record_usage(id) -> Result<()>                    │   │
│  │  + cleanup_expired() -> usize                        │   │
│  │  - save() -> Result<()>                              │   │
│  │  - load() -> Result<()>                              │   │
│  └──────────────────────────────────────────────────────┘   │
└───────────┬──────────────────────────────────────────────────┘
            │
            ▼
┌─────────────────────────────────────────────────────────────┐
│              Persistent Storage (JSON File)                  │
│                 ~/.mcprouter/tokens.json                     │
│  {                                                           │
│    "tokens": [                                               │
│      {                                                       │
│        "id": "tok-abc123...",                                │
│        "value": "mcp-XyZ789...",                             │
│        "name": "Production API",                             │
│        "description": "Token for production deployment",     │
│        "created_at": 1700000000,                             │
│        "expires_at": 1702000000,                             │
│        "last_used_at": 1700500000,                           │
│        "usage_count": 1234                                   │
│      }                                                       │
│    ]                                                         │
│  }                                                           │
└─────────────────────────────────────────────────────────────┘

            ▲                   │
            │                   │
            └───────┬───────────┘
                    │
            ┌───────▼────────┐
            │  Migration     │
            │  Logic         │
            │  (One-time)    │
            └────────────────┘
       server.bearer_token → tokens.json

## Core Data Structures

### Token Data Model

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub id: String,           // 唯一标识符: "tok-" + 32位随机字符
    pub value: String,        // Token 值: "mcp-" + 64位base64编码随机字符
    pub name: String,         // 用户友好的名称
    pub description: Option<String>, // 可选描述
    pub created_at: u64,      // 创建时间戳 (Unix timestamp)
    pub expires_at: Option<u64>, // 过期时间戳 (None = 永不过期)
    pub last_used_at: Option<u64>, // 最后使用时间戳
    pub usage_count: u64,     // 使用次数统计
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenStorage {
    pub tokens: HashMap<String, Token>,
    pub version: u32, // 存储格式版本,便于未来迁移
}
````

### Frontend Component Types

```typescript
export interface Token {
  id: string
  name: string
  description?: string
  created_at: number
  expires_at?: number
  last_used_at?: number
  usage_count: number
  // 注意: 不包含实际 token 值,仅用于列表显示
}

export interface CreateTokenRequest {
  name: string
  description?: string
  expires_in?: number // 秒数,如 3600 = 1小时, 86400 = 1天
}

export interface CreateTokenResponse {
  token: {
    id: string
    value: string // 仅在创建时返回
    name: string
    description?: string
    created_at: number
    expires_at?: number
  }
}
```

## Detailed Implementation

### 1. TokenManager (Backend Core)

#### Token Generation Strategy

```rust
impl TokenManager {
    // 生成安全的随机 token
    fn generate_secure_token() -> String {
        use rand::RngCore;
        let mut bytes = [0u8; 48]; // 48 bytes = 64 base64 chars
        let mut rng = rand::thread_rng();
        rng.fill_bytes(&mut bytes);
        format!("mcp-{}", base64::encode_config(&bytes, base64::URL_SAFE_NO_PAD))
    }

    // 生成 token ID
    fn generate_token_id() -> String {
        use rand::RngCore;
        let mut bytes = [0u8; 24]; // 24 bytes = 32 base64 chars
        let mut rng = rand::thread_rng();
        rng.fill_bytes(&mut bytes);
        format!("tok-{}", base64::encode_config(&bytes, base64::URL_SAFE_NO_PAD))
    }
}
```

### 2. Configuration Migration Logic

#### One-Time Migration Process

```rust
pub async fn migrate_from_static_config(
    config: &AppConfig,
    token_manager: &TokenManager,
) -> Result<bool, String> {
    // 检查是否已有 tokens (避免重复迁移)
    let existing_tokens = token_manager.list().await?;
    if !existing_tokens.is_empty() {
        return Ok(false); // 已有数据,跳过迁移
    }

    // 检查旧的配置
    if let (true, Some(ref static_token)) = (config.server.auth, &config.server.bearer_token) {
        tracing::info!("发现旧的静态 Bearer Token 配置,开始迁移...");

        let migrated_token = Token {
            id: TokenManager::generate_token_id(),
            value: static_token.clone(),
            name: "从配置迁移的 Token".to_string(),
            description: Some("从 server.bearer_token 自动迁移的访问凭证".to_string()),
            created_at: chrono::Utc::now().timestamp() as u64,
            expires_at: None, // 静态 token 永不过期
            last_used_at: None,
            usage_count: 0,
        };

        token_manager.add_token_direct(migrated_token).await?;

        tracing::warn!(
            "静态 token 已迁移到动态 Token 管理系统! \
            旧的 server.auth 和 server.bearer_token 配置字段可安全删除。"
        );

        return Ok(true);
    }

    Ok(false) // 无需迁移
}
```

## Security Considerations

### 1. Token Storage Security

- **File Permissions**: 确保 `~/.mcprouter/tokens.json` 仅用户可读 (chmod 600)
- **Memory Security**: 验证完成后不保留敏感信息在内存中
- **Secure Generation**: 使用密码学安全的随机数生成器

### 2. Token Generation Security

- **Cryptographically Secure**: 使用 `rand::thread_rng()` 生成密码学安全的随机数
- **Sufficient Length**: Token 值为 64 字符 base64 编码 (384 bits entropy)
- **Unique Prefix**: 使用 "mcp-" 前缀便于识别和过滤

### 3. Access Control

- **All Requests Authenticated**: 移除无认证模式,所有请求都需要有效 token
- **Audit Logging**: 记录所有认证成功/失败事件
