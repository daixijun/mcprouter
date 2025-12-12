use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// MCP 提示词实体
///
/// 对应数据库表 mcp_server_prompts，用于缓存 MCP 服务器提供的提示词信息
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "mcp_server_prompts")]
pub struct Model {
    /// 主键，UUID v7 格式
    #[sea_orm(primary_key)]
    pub id: String,

    /// 关联的服务器ID
    pub server_id: String,

    /// 提示词名称
    pub name: String,

    /// 提示词标题
    pub title: Option<String>,

    /// 提示词描述
    pub description: Option<String>,

    /// 是否启用
    pub enabled: bool,

    /// 参数定义（JSON 数组）
    pub arguments: Option<String>,

    /// 元数据（JSON object）
    pub meta: Option<String>,

    /// 创建时间
    #[sea_orm(column_type = "TimestampWithTimeZone")]
    pub created_at: ChronoDateTimeWithTimeZone,

    /// 更新时间
    #[sea_orm(column_type = "TimestampWithTimeZone")]
    pub updated_at: ChronoDateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 与服务器表的多对一关系
    #[sea_orm(
        belongs_to = "super::mcp_server::Entity",
        from = "Column::ServerId",
        to = "super::mcp_server::Column::Id"
    )]
    Server,
}

impl Related<super::mcp_server::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Server.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

/// 提示词参数定义
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptArgument {
    /// 参数名称
    pub name: String,
    /// 参数描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 是否必需
    #[serde(default)]
    pub required: bool,
    /// 参数类型
    pub argument_type: PromptArgumentType,
}

/// 提示词参数类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PromptArgumentType {
    /// 字符串类型
    String,
    /// 数字类型
    Number,
    /// 布尔类型
    Boolean,
    /// 数组类型
    Array,
    /// 对象类型
    Object,
}

impl Model {
    /// 解析参数定义
    pub fn parse_arguments(&self) -> Result<Vec<PromptArgument>, serde_json::Error> {
        match &self.arguments {
            Some(args_str) => serde_json::from_str(args_str),
            None => Ok(Vec::new()),
        }
    }

    /// 序列化参数定义
    pub fn serialize_arguments(args: &[PromptArgument]) -> String {
        serde_json::to_string(args).unwrap_or_default()
    }

    /// 检查提示词是否启用
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// 获取提示词显示名称（优先使用 title，其次是 name）
    pub fn get_display_name(&self) -> &str {
        self.title.as_deref().unwrap_or(&self.name)
    }

    /// 获取完整的提示词路径（server__prompt 格式）
    pub fn get_full_path(&self) -> String {
        // 注意：这里我们需要 server 的名称，但这需要 join 查询
        // 在实际使用中，应该通过 join 查询来获取
        format!("__{}", self.name)
    }

    /// 检查是否有参数定义
    pub fn has_arguments(&self) -> bool {
        self.arguments.is_some()
    }

    /// 获取必需参数列表
    pub fn get_required_arguments(&self) -> Result<Vec<String>, serde_json::Error> {
        let args = self.parse_arguments()?;
        Ok(args
            .into_iter()
            .filter(|arg| arg.required)
            .map(|arg| arg.name)
            .collect())
    }

    /// 获取可选参数列表
    pub fn get_optional_arguments(&self) -> Result<Vec<String>, serde_json::Error> {
        let args = self.parse_arguments()?;
        Ok(args
            .into_iter()
            .filter(|arg| !arg.required)
            .map(|arg| arg.name)
            .collect())
    }

    /// 获取所有参数名称
    pub fn get_all_argument_names(&self) -> Result<Vec<String>, serde_json::Error> {
        let args = self.parse_arguments()?;
        Ok(args.into_iter().map(|arg| arg.name).collect())
    }

    /// 检查是否包含特定参数
    pub fn has_argument(&self, arg_name: &str) -> Result<bool, serde_json::Error> {
        let args = self.parse_arguments()?;
        Ok(args.iter().any(|arg| arg.name == arg_name))
    }

    /// 获取参数定义
    pub fn get_argument(
        &self,
        arg_name: &str,
    ) -> Result<Option<PromptArgument>, serde_json::Error> {
        let args = self.parse_arguments()?;
        Ok(args.into_iter().find(|arg| arg.name == arg_name))
    }

    /// 解析元数据
    pub fn parse_meta(&self) -> Result<serde_json::Value, serde_json::Error> {
        match &self.meta {
            Some(meta_str) => serde_json::from_str(meta_str),
            None => Ok(serde_json::Value::Object(serde_json::Map::new())),
        }
    }

    /// 序列化元数据
    pub fn serialize_meta(meta: &serde_json::Value) -> String {
        serde_json::to_string(meta).unwrap_or_default()
    }

    /// 检查是否有元数据
    pub fn has_meta(&self) -> bool {
        self.meta.is_some()
    }

    /// 验证提供的参数是否符合要求
    pub fn validate_arguments(
        &self,
        provided_args: &std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<(), String> {
        let args = self
            .parse_arguments()
            .map_err(|e| format!("Failed to parse arguments: {}", e))?;

        // 检查必需参数
        for arg in &args {
            if arg.required && !provided_args.contains_key(&arg.name) {
                return Err(format!("Missing required argument: {}", arg.name));
            }
        }

        // 检查未知参数
        for arg_name in provided_args.keys() {
            if !args.iter().any(|arg| &arg.name == arg_name) {
                return Err(format!("Unknown argument: {}", arg_name));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_mcp_prompt_model() {
        let arguments = vec![
            PromptArgument {
                name: "message".to_string(),
                description: Some("The message to process".to_string()),
                required: true,
                argument_type: PromptArgumentType::String,
            },
            PromptArgument {
                name: "count".to_string(),
                description: Some("Number of repetitions".to_string()),
                required: false,
                argument_type: PromptArgumentType::Number,
            },
        ];

        let meta = serde_json::json!({
            "category": "processing",
            "tags": ["message", "process"],
            "author": "MCP Team",
            "version": "1.0.0"
        });

        let model = Model {
            id: "test-id".to_string(),
            server_id: "server-id".to_string(),
            name: "process_message".to_string(),
            title: Some("Process Message".to_string()),
            description: Some("Processes a message with optional repetitions".to_string()),
            enabled: true,
            arguments: Some(serde_json::to_string(&arguments).unwrap()),
            meta: Some(serde_json::to_string(&meta).unwrap()),
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
        };

        assert_eq!(model.name, "process_message");
        assert_eq!(model.get_display_name(), "Process Message");
        assert!(model.is_enabled());
        assert!(model.has_arguments());

        let parsed_args = model.parse_arguments().unwrap();
        assert_eq!(parsed_args.len(), 2);
        assert_eq!(parsed_args[0].name, "message");
        assert!(parsed_args[0].required);
        assert_eq!(parsed_args[1].name, "count");
        assert!(!parsed_args[1].required);

        let required_args = model.get_required_arguments().unwrap();
        assert_eq!(required_args, vec!["message"]);

        let optional_args = model.get_optional_arguments().unwrap();
        assert_eq!(optional_args, vec!["count"]);

        assert!(model.has_meta());
        let parsed_meta = model.parse_meta().unwrap();
        assert_eq!(parsed_meta["category"], "processing");
        assert_eq!(parsed_meta["author"], "MCP Team");
        assert_eq!(parsed_meta["version"], "1.0.0");
        assert!(parsed_meta["tags"].is_array());
    }

    #[test]
    fn test_prompt_without_title() {
        let model = Model {
            id: "test-id".to_string(),
            server_id: "server-id".to_string(),
            name: "simple_prompt".to_string(),
            title: None,
            description: None,
            enabled: true,
            arguments: None,
            meta: None,
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
        };

        assert_eq!(model.get_display_name(), "simple_prompt");
        assert!(!model.has_arguments());
        assert_eq!(model.get_required_arguments().unwrap().len(), 0);
        assert_eq!(model.get_optional_arguments().unwrap().len(), 0);
    }

    #[test]
    fn test_argument_validation() {
        let arguments = vec![
            PromptArgument {
                name: "required_arg".to_string(),
                description: None,
                required: true,
                argument_type: PromptArgumentType::String,
            },
            PromptArgument {
                name: "optional_arg".to_string(),
                description: None,
                required: false,
                argument_type: PromptArgumentType::Number,
            },
        ];

        let model = Model {
            id: "test-id".to_string(),
            server_id: "server-id".to_string(),
            name: "test_prompt".to_string(),
            arguments: Some(serde_json::to_string(&arguments).unwrap()),
            ..Default::default()
        };

        // 测试必需参数缺失
        let mut args = HashMap::new();
        let result = model.validate_arguments(&args);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Missing required argument: required_arg"));

        // 测试未知参数
        args.insert(
            "required_arg".to_string(),
            serde_json::Value::String("test".to_string()),
        );
        args.insert(
            "unknown_arg".to_string(),
            serde_json::Value::String("test".to_string()),
        );
        let result = model.validate_arguments(&args);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Unknown argument: unknown_arg"));

        // 测试有效参数
        args.remove("unknown_arg");
        let result = model.validate_arguments(&args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_serialization_methods() {
        let arguments = vec![PromptArgument {
            name: "test_arg".to_string(),
            description: Some("Test argument".to_string()),
            required: true,
            argument_type: PromptArgumentType::Boolean,
        }];

        let serialized = Model::serialize_arguments(&arguments);
        let parsed: Vec<PromptArgument> = serde_json::from_str(&serialized).unwrap();
        assert_eq!(arguments, parsed);

        // 测试空参数列表
        let empty_args: Vec<PromptArgument> = vec![];
        let serialized_empty = Model::serialize_arguments(&empty_args);
        let parsed_empty: Vec<PromptArgument> = serde_json::from_str(&serialized_empty).unwrap();
        assert_eq!(parsed_empty.len(), 0);
    }

    #[test]
    fn test_argument_operations() {
        let arguments = vec![
            PromptArgument {
                name: "arg1".to_string(),
                description: None,
                required: true,
                argument_type: PromptArgumentType::String,
            },
            PromptArgument {
                name: "arg2".to_string(),
                description: None,
                required: false,
                argument_type: PromptArgumentType::Array,
            },
        ];

        let model = Model {
            id: "test-id".to_string(),
            server_id: "server-id".to_string(),
            name: "test_prompt".to_string(),
            arguments: Some(serde_json::to_string(&arguments).unwrap()),
            ..Default::default()
        };

        assert!(model.has_argument("arg1").unwrap());
        assert!(model.has_argument("arg2").unwrap());
        assert!(!model.has_argument("arg3").unwrap());

        let arg1 = model.get_argument("arg1").unwrap();
        assert!(arg1.is_some());
        assert!(arg1.unwrap().required);

        let arg3 = model.get_argument("arg3").unwrap();
        assert!(arg3.is_none());

        let all_names = model.get_all_argument_names().unwrap();
        assert_eq!(all_names, vec!["arg1", "arg2"]);
    }

    #[test]
    fn test_prompt_argument_type() {
        let types = vec![
            PromptArgumentType::String,
            PromptArgumentType::Number,
            PromptArgumentType::Boolean,
            PromptArgumentType::Array,
            PromptArgumentType::Object,
        ];

        for arg_type in &types {
            let serialized = serde_json::to_string(arg_type).unwrap();
            let parsed: PromptArgumentType = serde_json::from_str(&serialized).unwrap();
            assert_eq!(arg_type, &parsed);
        }
    }
}

impl Default for Model {
    fn default() -> Self {
        Self {
            id: String::new(),
            server_id: String::new(),
            name: String::new(),
            title: None,
            description: None,
            enabled: true,
            arguments: None,
            meta: None,
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
        }
    }
}
