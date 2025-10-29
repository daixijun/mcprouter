use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, RwLock};
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::error::{McpError, Result};

/// Task priority for scheduling
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub enum TaskPriority {
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}

/// Task status tracking
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed(String),
    Cancelled,
}

/// Task metadata for tracking
#[derive(Debug, Clone, serde::Serialize)]
pub struct TaskMetadata {
    pub id: String,
    pub name: String,
    pub priority: TaskPriority,
    pub status: TaskStatus,
    pub created_at: String,           // ISO 8601 string
    pub started_at: Option<String>,   // ISO 8601 string
    pub completed_at: Option<String>, // ISO 8601 string
}

/// Async task manager for handling background operations
pub struct TaskManager {
    /// Task storage and tracking
    tasks: Arc<RwLock<HashMap<String, TaskMetadata>>>,

    /// Command channel for task operations
    command_tx: mpsc::UnboundedSender<TaskCommand>,

    /// Background task handle
    _manager_handle: JoinHandle<()>,
}

/// Commands for the task manager
enum TaskCommand {
    Submit {
        metadata: TaskMetadata,
        response_tx: oneshot::Sender<Result<()>>,
    },
    Cancel {
        task_id: String,
        response_tx: oneshot::Sender<bool>,
    },
    GetStatus {
        task_id: String,
        response_tx: oneshot::Sender<Option<TaskMetadata>>,
    },
    ListTasks {
        response_tx: oneshot::Sender<Vec<TaskMetadata>>,
    },
    Cleanup {
        response_tx: oneshot::Sender<usize>,
    },
}

impl TaskManager {
    /// Create new task manager
    pub fn new() -> Self {
        let tasks = Arc::new(RwLock::new(HashMap::new()));
        let (command_tx, command_rx) = mpsc::unbounded_channel();

        // Start background task manager
        let manager_handle = tokio::spawn(Self::run_task_manager(tasks.clone(), command_rx));

        Self {
            tasks,
            command_tx,
            _manager_handle: manager_handle,
        }
    }

    /// Submit a new task for execution (simplified version)
    pub async fn submit_task(&self, name: String, priority: TaskPriority) -> Result<String> {
        let task_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let metadata = TaskMetadata {
            id: task_id.clone(),
            name,
            priority,
            status: TaskStatus::Pending,
            created_at: now.clone(),
            started_at: None,
            completed_at: None,
        };

        let (response_tx, response_rx) = oneshot::channel();
        let command = TaskCommand::Submit {
            metadata,
            response_tx,
        };

        self.command_tx
            .send(command)
            .map_err(|_| McpError::ConfigError("Task manager channel closed".to_string()))?;

        response_rx
            .await
            .map_err(|_| McpError::ConfigError("Task submission response failed".to_string()))??;

        Ok(task_id)
    }

    /// Cancel a running task
    pub async fn cancel_task(&self, task_id: &str) -> Result<bool> {
        let (response_tx, response_rx) = oneshot::channel();
        let command = TaskCommand::Cancel {
            task_id: task_id.to_string(),
            response_tx,
        };

        self.command_tx
            .send(command)
            .map_err(|_| McpError::ConfigError("Task manager channel closed".to_string()))?;

        response_rx
            .await
            .map_err(|_| McpError::ConfigError("Task cancellation response failed".to_string()))
    }

    /// Get task status
    pub async fn get_task_status(&self, task_id: &str) -> Result<Option<TaskMetadata>> {
        let (response_tx, response_rx) = oneshot::channel();
        let command = TaskCommand::GetStatus {
            task_id: task_id.to_string(),
            response_tx,
        };

        self.command_tx
            .send(command)
            .map_err(|_| McpError::ConfigError("Task manager channel closed".to_string()))?;

        response_rx
            .await
            .map_err(|_| McpError::ConfigError("Task status response failed".to_string()))
    }

    /// List all tasks
    pub async fn list_tasks(&self) -> Result<Vec<TaskMetadata>> {
        let (response_tx, response_rx) = oneshot::channel();
        let command = TaskCommand::ListTasks { response_tx };

        self.command_tx
            .send(command)
            .map_err(|_| McpError::ConfigError("Task manager channel closed".to_string()))?;

        response_rx
            .await
            .map_err(|_| McpError::ConfigError("Task list response failed".to_string()))
    }

    /// Get task statistics
    pub async fn get_task_stats(&self) -> Result<TaskStats> {
        let tasks = self.tasks.read().await;
        let mut stats = TaskStats::default();

        let now = std::time::SystemTime::now();
        let recent_cutoff = now - std::time::Duration::from_secs(60); // 1 minute ago

        for task in tasks.values() {
            match task.status {
                TaskStatus::Pending => stats.pending += 1,
                TaskStatus::Running => stats.running += 1,
                TaskStatus::Completed => stats.completed += 1,
                TaskStatus::Failed(_) => stats.failed += 1,
                TaskStatus::Cancelled => stats.cancelled += 1,
            }

            // Check if task is recent (within last minute)
            if let Ok(datetime) = chrono::DateTime::parse_from_rfc3339(&task.created_at) {
                let task_time = datetime.with_timezone(&chrono::Utc);
                if task_time.timestamp()
                    > (recent_cutoff
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i64)
                {
                    stats.recent += 1;
                }
            }
        }

        Ok(stats)
    }

    /// Clean up old completed tasks
    pub async fn cleanup_old_tasks(&self) -> Result<usize> {
        let (response_tx, response_rx) = oneshot::channel::<usize>();
        let command = TaskCommand::Cleanup { response_tx };
        self.command_tx
            .send(command)
            .map_err(|_| McpError::ConfigError("Task manager channel closed".to_string()))?;

        response_rx
            .await
            .map_err(|_| McpError::ConfigError("Task cleanup response failed".to_string()))
    }

    /// Check for and handle task timeouts
    pub async fn check_timeouts(
        &self,
        timeout_duration: std::time::Duration,
    ) -> Result<Vec<String>> {
        let tasks = self.tasks.read().await;
        let mut timed_out_tasks = Vec::new();
        let now = std::time::SystemTime::now();

        for (task_id, task) in tasks.iter() {
            // Only check running tasks for timeouts
            if task.status == TaskStatus::Running {
                if let Some(started_at_str) = &task.started_at {
                    if let Ok(started_datetime) =
                        chrono::DateTime::parse_from_rfc3339(started_at_str)
                    {
                        let started_time = started_datetime.with_timezone(&chrono::Utc);
                        if let Ok(duration) = now.duration_since(
                            std::time::UNIX_EPOCH
                                + std::time::Duration::from_secs(started_time.timestamp() as u64),
                        ) {
                            if duration > timeout_duration {
                                timed_out_tasks.push(task_id.clone());
                            }
                        }
                    }
                }
            }
        }

        // Mark timed out tasks as failed
        if !timed_out_tasks.is_empty() {
            for task_id in &timed_out_tasks {
                self.set_task_failed(task_id, "Task timed out").await?;
                tracing::warn!("Task timed out: {}", task_id);
            }
        }

        Ok(timed_out_tasks)
    }

    /// Mark a task as failed
    async fn set_task_failed(&self, task_id: &str, error: &str) -> Result<()> {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(task_id) {
            task.status = TaskStatus::Failed(error.to_string());
            task.completed_at = Some(chrono::Utc::now().to_rfc3339());
            Ok(())
        } else {
            Err(McpError::ConfigError("Task not found".to_string()))
        }
    }

    /// Get tasks by status
    pub async fn get_tasks_by_status(&self, status: TaskStatus) -> Result<Vec<TaskMetadata>> {
        let tasks = self.tasks.read().await;
        let filtered_tasks = tasks
            .values()
            .filter(|task| match (&task.status, &status) {
                (TaskStatus::Failed(msg1), TaskStatus::Failed(msg2)) => msg1 == msg2,
                _ => task.status == status,
            })
            .cloned()
            .collect();
        Ok(filtered_tasks)
    }

    /// Get running tasks count
    pub async fn get_running_tasks_count(&self) -> usize {
        let tasks = self.tasks.read().await;
        tasks
            .values()
            .filter(|task| task.status == TaskStatus::Running)
            .count()
    }

    /// Background task manager loop
    async fn run_task_manager(
        tasks: Arc<RwLock<HashMap<String, TaskMetadata>>>,
        mut command_rx: mpsc::UnboundedReceiver<TaskCommand>,
    ) {
        tracing::info!("Task manager started");

        while let Some(command) = command_rx.recv().await {
            match command {
                TaskCommand::Submit {
                    metadata,
                    response_tx,
                } => {
                    let task_id = metadata.id.clone();
                    let tasks_clone = tasks.clone();

                    // Store task metadata
                    {
                        let mut tasks_guard = tasks_clone.write().await;
                        tasks_guard.insert(task_id.clone(), metadata.clone());
                    }

                    // Execute task with proper error handling
                    let task_priority = metadata.priority;
                    tokio::spawn(async move {
                        // Mark as running
                        {
                            let mut tasks_guard = tasks_clone.write().await;
                            if let Some(task_meta) = tasks_guard.get_mut(&task_id) {
                                task_meta.status = TaskStatus::Running;
                                task_meta.started_at = Some(chrono::Utc::now().to_rfc3339());
                            }
                        }

                        tracing::info!(
                            "Starting task execution: {} (priority: {:?})",
                            task_id,
                            task_priority
                        );

                        // Simulate actual work with configurable duration
                        let work_duration = match task_priority {
                            TaskPriority::Critical => std::time::Duration::from_millis(50),
                            TaskPriority::High => std::time::Duration::from_millis(100),
                            TaskPriority::Normal => std::time::Duration::from_millis(200),
                            TaskPriority::Low => std::time::Duration::from_millis(300),
                        };

                        // Do some simulated work
                        tokio::time::sleep(work_duration).await;

                        // Update task status as completed
                        {
                            let mut tasks_guard = tasks_clone.write().await;
                            if let Some(task_meta) = tasks_guard.get_mut(&task_id) {
                                task_meta.status = TaskStatus::Completed;
                                task_meta.completed_at = Some(chrono::Utc::now().to_rfc3339());
                            }
                        }

                        tracing::info!("Task completed successfully: {}", task_id);

                        // Send response
                        let _ = response_tx.send(Ok(()));
                    });
                }

                TaskCommand::Cancel {
                    task_id,
                    response_tx,
                } => {
                    let cancelled = {
                        let mut tasks_guard = tasks.write().await;
                        if let Some(task_meta) = tasks_guard.get_mut(&task_id) {
                            if matches!(task_meta.status, TaskStatus::Pending | TaskStatus::Running)
                            {
                                task_meta.status = TaskStatus::Cancelled;
                                task_meta.completed_at = Some(chrono::Utc::now().to_rfc3339());
                                true
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    };
                    let _ = response_tx.send(cancelled);
                }

                TaskCommand::GetStatus {
                    task_id,
                    response_tx,
                } => {
                    let task_meta = tasks.read().await.get(&task_id).cloned();
                    let _ = response_tx.send(task_meta);
                }

                TaskCommand::ListTasks { response_tx } => {
                    let task_list: Vec<TaskMetadata> =
                        tasks.read().await.values().cloned().collect();
                    let _ = response_tx.send(task_list);
                }

                TaskCommand::Cleanup { response_tx } => {
                    let mut tasks_guard = tasks.write().await;
                    let mut removed_count = 0;
                    let cutoff =
                        std::time::SystemTime::now() - std::time::Duration::from_secs(3600); // 1 hour

                    // Parse ISO 8601 timestamps and clean up old completed tasks
                    tasks_guard.retain(|_, meta| {
                        // Keep running tasks
                        if matches!(meta.status, TaskStatus::Running) {
                            return true;
                        }

                        // Parse timestamp from string
                        if let Ok(datetime) = chrono::DateTime::parse_from_rfc3339(&meta.created_at)
                        {
                            let task_time = datetime.with_timezone(&chrono::Utc);
                            if task_time.timestamp()
                                > (cutoff
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_secs() as i64)
                            {
                                return true; // Keep recent tasks
                            }
                        }

                        // Keep failed tasks for debugging
                        if matches!(meta.status, TaskStatus::Failed(_)) {
                            return true;
                        }

                        // Remove old completed tasks
                        removed_count += 1;
                        false
                    });

                    if removed_count > 0 {
                        tracing::info!("Cleaned up {} old completed tasks", removed_count);
                    }
                    tracing::debug!("Task cleanup completed");
                    let _ = response_tx.send(removed_count);
                }
            }
        }

        tracing::info!("Task manager stopped");
    }
}

/// Task statistics
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct TaskStats {
    pub pending: usize,
    pub running: usize,
    pub completed: usize,
    pub failed: usize,
    pub cancelled: usize,
    pub recent: usize, // Tasks created in the last minute
}

impl Drop for TaskManager {
    fn drop(&mut self) {
        // The background task will be automatically cancelled when the manager is dropped
        tracing::info!("Task manager dropped");
    }
}

/// Global task manager instance
static TASK_MANAGER: std::sync::LazyLock<Arc<TaskManager>> =
    std::sync::LazyLock::new(|| Arc::new(TaskManager::new()));

/// Get global task manager
pub fn get_task_manager() -> &'static Arc<TaskManager> {
    &TASK_MANAGER
}

/// Convenience function to submit a task
pub async fn submit_task(name: String, priority: TaskPriority) -> Result<String> {
    get_task_manager().submit_task(name, priority).await
}
