// ============================================
// workflow.rs - Workflow Automation
// ============================================
// Multi-step automated workflows with rollback

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{error, info, warn};

use crate::executor::{ExecutionResult, Executor};

/// Workflow definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub name: String,
    pub description: String,
    pub steps: Vec<WorkflowStep>,
    
    #[serde(default)]
    pub rollback_on_failure: bool,
    
    #[serde(default)]
    pub continue_on_error: bool,
    
    #[serde(default)]
    pub variables: HashMap<String, String>,
}

/// Single workflow step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub name: String,
    pub command: String,
    
    #[serde(default)]
    pub rollback_command: Option<String>,
    
    #[serde(default)]
    pub condition: Option<String>,
    
    #[serde(default)]
    pub retry_count: usize,
    
    #[serde(default)]
    pub timeout: Option<u64>,
    
    #[serde(default)]
    pub critical: bool,
}

/// Workflow execution result
#[derive(Debug)]
pub struct WorkflowResult {
    pub success: bool,
    pub steps_completed: usize,
    pub step_results: Vec<StepResult>,
    pub duration_ms: u64,
    pub rolled_back: bool,
}

/// Step execution result
#[derive(Debug, Clone)]
pub struct StepResult {
    pub step_name: String,
    pub success: bool,
    pub output: ExecutionResult,
    pub rolled_back: bool,
}

/// Workflow executor
pub struct WorkflowExecutor {
    executor: Executor,
}

impl WorkflowExecutor {
    /// Create new workflow executor
    pub fn new(executor: Executor) -> Self {
        Self { executor }
    }

    /// Execute a workflow
    pub async fn execute(&self, workflow: &Workflow) -> Result<WorkflowResult> {
        info!("Starting workflow: {}", workflow.name);

        let start = std::time::Instant::now();
        let mut step_results = Vec::new();
        let mut steps_completed = 0;
        let mut rolled_back = false;

        // Execute each step
        for (i, step) in workflow.steps.iter().enumerate() {
            info!("Executing step {}/{}: {}", i + 1, workflow.steps.len(), step.name);

            // Check condition if present
            if let Some(ref condition) = step.condition {
                if !self.check_condition(condition, &workflow.variables).await? {
                    info!("Step {} skipped due to condition", step.name);
                    continue;
                }
            }

            // Replace variables in command
            let command = self.replace_variables(&step.command, &workflow.variables)?;

            // Execute step with retries
            let result = self.execute_step(&command, step.retry_count).await?;
            let success = result.success();

            step_results.push(StepResult {
                step_name: step.name.clone(),
                success,
                output: result,
                rolled_back: false,
            });

            if success {
                steps_completed += 1;
            } else {
                error!("Step {} failed: {}", step.name, step_results.last().unwrap().output.stderr);

                // Handle failure
                if step.critical || !workflow.continue_on_error {
                    warn!("Critical step failed, stopping workflow");

                    // Rollback if configured
                    if workflow.rollback_on_failure {
                        rolled_back = self.rollback_steps(&workflow.steps[..i], &step_results).await?;
                    }

                    break;
                } else {
                    warn!("Step failed but continuing due to continue_on_error");
                }
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;
        let success = steps_completed == workflow.steps.len() && !rolled_back;

        info!(
            "Workflow completed: {} (success: {}, steps: {}/{}, duration: {}ms, rolled_back: {})",
            workflow.name,
            success,
            steps_completed,
            workflow.steps.len(),
            duration_ms,
            rolled_back
        );

        Ok(WorkflowResult {
            success,
            steps_completed,
            step_results,
            duration_ms,
            rolled_back,
        })
    }

    /// Execute single step with retries
    async fn execute_step(&self, command: &str, retry_count: usize) -> Result<ExecutionResult> {
        let mut attempts = 0;
        let max_attempts = retry_count + 1;

        loop {
            attempts += 1;

            match self.executor.execute(command).await {
                Ok(result) => {
                    if result.success() || attempts >= max_attempts {
                        return Ok(result);
                    }

                    warn!(
                        "Command failed (attempt {}/{}), retrying...",
                        attempts, max_attempts
                    );

                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                }
                Err(e) => {
                    if attempts >= max_attempts {
                        return Err(e);
                    }

                    warn!("Command error (attempt {}/{}): {}", attempts, max_attempts, e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                }
            }
        }
    }

    /// Rollback executed steps
    async fn rollback_steps(
        &self,
        steps: &[WorkflowStep],
        results: &[StepResult],
    ) -> Result<bool> {
        info!("Rolling back {} steps...", results.len());

        let mut rolled_back = false;

        // Rollback in reverse order
        for (step, result) in steps.iter().zip(results.iter()).rev() {
            if !result.success {
                continue; // Skip failed steps
            }

            if let Some(ref rollback_cmd) = step.rollback_command {
                info!("Rolling back step: {}", step.name);

                match self.executor.execute(rollback_cmd).await {
                    Ok(rollback_result) => {
                        if rollback_result.success() {
                            info!("Successfully rolled back: {}", step.name);
                            rolled_back = true;
                        } else {
                            error!("Rollback failed for {}: {}", step.name, rollback_result.stderr);
                        }
                    }
                    Err(e) => {
                        error!("Rollback error for {}: {}", step.name, e);
                    }
                }
            }
        }

        Ok(rolled_back)
    }

    /// Check condition
    async fn check_condition(
        &self,
        condition: &str,
        variables: &HashMap<String, String>,
    ) -> Result<bool> {
        // Replace variables
        let condition = self.replace_variables(condition, variables)?;

        // Execute condition as command
        match self.executor.execute(&condition).await {
            Ok(result) => Ok(result.success()),
            Err(_) => Ok(false),
        }
    }

    /// Replace variables in string
    fn replace_variables(&self, text: &str, variables: &HashMap<String, String>) -> Result<String> {
        let mut result = text.to_string();

        for (key, value) in variables {
            let placeholder = format!("${{{}}}", key);
            result = result.replace(&placeholder, value);
        }

        Ok(result)
    }

    /// Create workflow from YAML
    pub fn from_yaml(yaml: &str) -> Result<Workflow> {
        let workflow: Workflow = serde_yaml::from_str(yaml)
            .context("Failed to parse workflow YAML")?;

        Ok(workflow)
    }

    /// Format workflow result
    pub fn format_result(&self, result: &WorkflowResult) -> String {
        let mut output = String::new();

        if result.success {
            output.push_str("✅ Workflow completed successfully\n");
        } else {
            output.push_str("❌ Workflow failed\n");
        }

        output.push_str(&format!(
            "Steps completed: {}\nDuration: {}ms\n",
            result.steps_completed, result.duration_ms
        ));

        if result.rolled_back {
            output.push_str("⚠️  Workflow was rolled back\n");
        }

        output.push_str("\nStep Results:\n");

        for (i, step) in result.step_results.iter().enumerate() {
            let status = if step.success { "✅" } else { "❌" };
            let rollback = if step.rolled_back { " (rolled back)" } else { "" };

            output.push_str(&format!(
                "  {}. {} {} ({}ms){}\n",
                i + 1,
                status,
                step.step_name,
                step.output.duration_ms,
                rollback
            ));

            if !step.success && !step.output.stderr.is_empty() {
                output.push_str(&format!("     Error: {}\n", step.output.stderr.lines().next().unwrap_or("")));
            }
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn test_config() -> Config {
        Config {
            gemini_api_key: "test".to_string(),
            signal_number: "+1234567890".to_string(),
            authorized_numbers: vec![],
            require_confirmation: false,
            confirmation_timeout: 60,
            allow_commands: true,
            deny_by_default: false,
            allowed_commands: vec![],
            blocked_commands: vec![],
            dry_run: true,
            ssh_key_path: None,
            ssh_verify_host_keys: true,
            allowed_ssh_hosts: vec![],
            ssh_timeout: 30,
            logging: Default::default(),
            execution: Default::default(),
            gemini: Default::default(),
            rate_limit: Default::default(),
        }
    }

    #[tokio::test]
    async fn test_workflow_creation() {
        let yaml = r#"
name: test_workflow
description: Test workflow
steps:
  - name: step1
    command: echo "hello"
  - name: step2
    command: echo "world"
rollback_on_failure: true
"#;

        let workflow = WorkflowExecutor::from_yaml(yaml).unwrap();
        assert_eq!(workflow.name, "test_workflow");
        assert_eq!(workflow.steps.len(), 2);
        assert!(workflow.rollback_on_failure);
    }

    #[test]
    fn test_variable_replacement() {
        let config = test_config();
        let executor = Executor::new(config);
        let workflow_executor = WorkflowExecutor::new(executor);

        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "world".to_string());

        let result = workflow_executor
            .replace_variables("echo ${name}", &vars)
            .unwrap();

        assert_eq!(result, "echo world");
    }
}