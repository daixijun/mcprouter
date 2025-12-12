#[cfg(test)]
mod token_permissions_test {
    use super::*;
    use crate::token_manager::TokenManager;

    #[tokio::test]
    async fn test_token_permissions_integration() {
        // This test demonstrates that token permissions are correctly loaded
        // from the database when calling convert_to_token_info

        // Create test environment
        let db_url = "sqlite::memory:";
        let db = sea_orm::Database::connect(db_url).await.unwrap();
        crate::migration::Migrator::up(&db, None).await.unwrap();

        let orm_storage = crate::storage::orm_storage::Storage::new(db_url).await.unwrap();
        let token_manager = TokenManager::new(std::sync::Arc::new(orm_storage)).await.unwrap();

        // Create a token
        let token_info = token_manager.create("test-token".to_string(), Some("Test token".to_string())).await.unwrap();

        // Add some permissions
        let _ = token_manager.orm_storage
            .add_permission(&token_info.id, "tool", "test-server__test-tool")
            .await;
        let _ = token_manager.orm_storage
            .add_permission(&token_info.id, "resource", "test-server__test-resource")
            .await;
        let _ = token_manager.orm_storage
            .add_permission(&token_info.id, "prompt", "test-server__test-prompt")
            .await;

        // Get the raw token from database
        let token = token_manager.orm_storage.get_token_by_id(&token_info.id).await.unwrap().unwrap();

        // Test the convert_to_token_info method (this is our main fix)
        let result = token_manager.convert_to_token_info(&token).await.unwrap();

        // Verify permissions are correctly loaded
        assert_eq!(result.id, token_info.id);
        assert_eq!(result.name, "test-token");
        assert_eq!(result.allowed_tools, vec!["test-server__test-tool"]);
        assert_eq!(result.allowed_resources, vec!["test-server__test-resource"]);
        assert_eq!(result.allowed_prompts, vec!["test-server__test-prompt"]);

        // Verify that the fix works - permissions are loaded from database, not empty vectors
        assert!(!result.allowed_tools.is_empty(), "Tools should not be empty");
        assert!(!result.allowed_resources.is_empty(), "Resources should not be empty");
        assert!(!result.allowed_prompts.is_empty(), "Prompts should not be empty");
    }
}