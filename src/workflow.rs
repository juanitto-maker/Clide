// ============================================
// workflow.rs - Workflow Automation (CORRECTED)
// ============================================

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{error, info, warn};

use crate::executor::{ExecutionResult, Executor};

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

#[derive(Debug)]
pub struct WorkflowResult {
    pub workflow_name: String,
    pub step_results: Vec<(String, ExecutionResult)>,
    pub success: bool,
}

pub struct WorkflowExecutor {
    executor: Executor,
}

impl WorkflowExecutor {
    pub fn new(executor: Executor) -> Self {
        Self { executor }
    }

    pub async fn execute_workflow(
        &self,
        workflow: &Workflow,
        vars: &HashMap<String, String>,
    ) -> Result<WorkflowResult> {
        info!("Starting workflow: {}", workflow.name);
        let mut results = Vec::new();
        let mut success = true;
        let mut executed_steps = Vec::new();

        // Merge default workflow variables with provided ones
        let mut active_vars = workflow.variables.clone();
        for (k, v) in vars {
            active_vars.insert(k.clone(), v.clone());
        }

        for step in &workflow.steps {
            info!("Executing step: {}", step.name);
            
            let mut command = step.command.clone();
            for (key, value) in &active_vars {
                command = command.replace(&format!("{{{{{}}}}}", key), value);
            }

            let res = self.executor.execute(&command).await?;
            results.push((step.name.clone(), res.clone()));
            executed_steps.push(step);

            if !res.success() {
                error!("Step {} failed", step.name);
                if !workflow.continue_on_error {
                    success = false;
                    if workflow.rollback_on_failure {
                        self.rollback(&executed_steps, &active_vars).await?;
                    }
                    break;
                }
            }
        }

        Ok(WorkflowResult {
            workflow_name: workflow.name.clone(),
            step_results: results,
            success,
        })
    }

    async fn rollback(&self, steps: &[&WorkflowStep], vars: &HashMap<String, String>) -> Result<()> {
        warn!("Starting rollback procedure...");
        for step in steps.iter().rev() {
            if let Some(ref rollback_cmd) = step.rollback_command {
                let mut cmd = rollback_cmd.clone();
                for (k, v) in vars {
                    cmd = cmd.replace(&format!("{{{{{}}}}}", k), v);
                }
                info!("Rolling back step: {}", step.name);
                let _ = self.executor.execute(&cmd).await;
            }
        }
        Ok(())
    }
}
