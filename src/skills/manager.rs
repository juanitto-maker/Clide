// ============================================
// skills/manager.rs - Skills Manager (CORRECTED)
// ============================================

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

use crate::executor::{ExecutionResult, Executor};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub parameters: HashMap<String, ParameterDef>,
    pub commands: Vec<String>,
    #[serde(default)]
    pub require_confirmation: bool,
    #[serde(default)]
    pub retry_count: usize,
    #[serde(default)]
    pub timeout: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterDef {
    pub description: String,
    #[serde(rename = "type")]
    pub param_type: String,
    pub required: bool,
    pub default: Option<String>,
}

#[derive(Debug)]
pub struct SkillResult {
    pub skill_name: String,
    pub results: Vec<ExecutionResult>,
    pub success: bool,
}

pub struct SkillManager {
    pub skills: HashMap<String, Skill>,
    pub path: PathBuf,
}

impl SkillManager {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if !path.exists() {
            std::fs::create_dir_all(&path)?;
        }

        let mut manager = Self {
            skills: HashMap::new(),
            path,
        };
        manager.load_skills()?;
        Ok(manager)
    }

    pub fn load_skills(&mut self) -> Result<()> {
        self.skills.clear();
        let entries = std::fs::read_dir(&self.path)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                let content = std::fs::read_to_string(&path)?;
                match serde_yaml::from_str::<Skill>(&content) {
                    Ok(skill) => {
                        debug!("Loaded skill: {}", skill.name);
                        self.skills.insert(skill.name.clone(), skill);
                    }
                    Err(e) => warn!("Failed to load skill at {:?}: {}", path, e),
                }
            }
        }
        info!("Loaded {} skills", self.skills.len());
        Ok(())
    }

    pub async fn execute_skill(
        &self,
        name: &str,
        params: &HashMap<String, String>,
        executor: &Executor,
    ) -> Result<SkillResult> {
        let skill = self.skills.get(name)
            .context(format!("Skill '{}' not found", name))?;

        let mut results = Vec::new();
        let mut overall_success = true;

        for cmd_template in &skill.commands {
            let mut command = cmd_template.clone();
            for (key, value) in params {
                command = command.replace(&format!("{{{{{}}}}}", key), value);
            }

            let res = executor.execute(&command).await?;
            if !res.success() {
                overall_success = false;
            }
            results.push(res);
        }

        Ok(SkillResult {
            skill_name: name.to_string(),
            results,
            success: overall_success,
        })
    }

    pub fn list_skills(&self) -> Vec<&Skill> {
        self.skills.values().collect()
    }
}
