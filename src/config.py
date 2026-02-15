"""
clide - Configuration Module
Handles loading and validating configuration from config.yaml
"""

import os
import yaml
from pathlib import Path
from typing import Dict, Any, List, Optional
from dataclasses import dataclass


@dataclass
class SignalConfig:
    """Signal messenger configuration"""
    phone_number: str
    receive_groups: bool = False
    admin_only: bool = True
    allowed_numbers: List[str] = None
    signal_cli_path: Optional[str] = None


@dataclass
class GeminiConfig:
    """Gemini AI configuration"""
    api_key: str
    model: str = "gemini-2.0-flash-exp"
    temperature: float = 0.7
    max_tokens: int = 2048
    system_prompt: str = ""


@dataclass
class ClineConfig:
    """Cline CLI configuration"""
    enabled: bool = True
    max_retries: int = 3
    timeout: int = 300
    safety_level: str = "medium"
    cline_path: Optional[str] = None


@dataclass
class MemoryConfig:
    """Memory and database configuration"""
    database_path: str
    max_history: int = 1000
    auto_cleanup: bool = True
    cleanup_days: int = 90
    encryption_passphrase: str = ""


@dataclass
class SafetyConfig:
    """Safety and security configuration"""
    dry_run_default: bool = False
    confirm_destructive: bool = True
    confirm_all: bool = False
    auto_backup: bool = True
    blocked_patterns: List[str] = None
    requires_confirmation: List[str] = None


@dataclass
class VPSConfig:
    """VPS server configuration"""
    name: str
    host: str
    user: str
    port: int = 22
    ssh_key: str = ""
    description: str = ""


@dataclass
class LoggingConfig:
    """Logging configuration"""
    level: str = "INFO"
    file: str = "~/.clide/logs/clide.log"
    max_size: str = "10MB"
    backup_count: int = 5
    format: str = "%(asctime)s - %(name)s - %(levelname)s - %(message)s"
    console: bool = True


@dataclass
class MonitoringConfig:
    """Monitoring and alerts configuration"""
    enabled: bool = True
    check_interval: int = 300
    alerts: Dict[str, int] = None
    notifications: Dict[str, bool] = None


class Config:
    """Main configuration class"""
    
    def __init__(self, config_path: str = "config.yaml"):
        self.config_path = config_path
        self.raw_config: Dict[str, Any] = {}
        
        # Configuration objects
        self.signal: Optional[SignalConfig] = None
        self.gemini: Optional[GeminiConfig] = None
        self.cline: Optional[ClineConfig] = None
        self.memory: Optional[MemoryConfig] = None
        self.safety: Optional[SafetyConfig] = None
        self.logging: Optional[LoggingConfig] = None
        self.monitoring: Optional[MonitoringConfig] = None
        self.vps_list: List[VPSConfig] = []
        self.workflows: Dict[str, Any] = {}
        
        # Load configuration
        self.load()
    
    def load(self) -> None:
        """Load configuration from YAML file"""
        if not os.path.exists(self.config_path):
            raise FileNotFoundError(
                f"Configuration file not found: {self.config_path}\n"
                f"Please copy config.example.yaml to config.yaml and customize it."
            )
        
        with open(self.config_path, 'r') as f:
            self.raw_config = yaml.safe_load(f)
        
        # Parse configuration sections
        self._parse_signal()
        self._parse_gemini()
        self._parse_cline()
        self._parse_memory()
        self._parse_safety()
        self._parse_logging()
        self._parse_monitoring()
        self._parse_vps()
        self._parse_workflows()
        
        # Validate configuration
        self.validate()
    
    def _parse_signal(self) -> None:
        """Parse Signal configuration"""
        signal_data = self.raw_config.get('signal', {})
        self.signal = SignalConfig(
            phone_number=signal_data.get('phone_number', ''),
            receive_groups=signal_data.get('receive_groups', False),
            admin_only=signal_data.get('admin_only', True),
            allowed_numbers=signal_data.get('allowed_numbers', []),
            signal_cli_path=signal_data.get('signal_cli_path')
        )
    
    def _parse_gemini(self) -> None:
        """Parse Gemini configuration"""
        gemini_data = self.raw_config.get('gemini', {})
        
        # Handle environment variables in API key
        api_key = gemini_data.get('api_key', '')
        if api_key.startswith('${') and api_key.endswith('}'):
            env_var = api_key[2:-1]
            api_key = os.getenv(env_var, '')
        
        self.gemini = GeminiConfig(
            api_key=api_key,
            model=gemini_data.get('model', 'gemini-2.0-flash-exp'),
            temperature=gemini_data.get('temperature', 0.7),
            max_tokens=gemini_data.get('max_tokens', 2048),
            system_prompt=gemini_data.get('system_prompt', '')
        )
    
    def _parse_cline(self) -> None:
        """Parse Cline configuration"""
        cline_data = self.raw_config.get('cline', {})
        self.cline = ClineConfig(
            enabled=cline_data.get('enabled', True),
            max_retries=cline_data.get('max_retries', 3),
            timeout=cline_data.get('timeout', 300),
            safety_level=cline_data.get('safety_level', 'medium'),
            cline_path=cline_data.get('cline_path')
        )
    
    def _parse_memory(self) -> None:
        """Parse memory configuration"""
        memory_data = self.raw_config.get('memory', {})
        db_path = memory_data.get('database_path', '~/.clide/memory.db')
        
        self.memory = MemoryConfig(
            database_path=os.path.expanduser(db_path),
            max_history=memory_data.get('max_history', 1000),
            auto_cleanup=memory_data.get('auto_cleanup', True),
            cleanup_days=memory_data.get('cleanup_days', 90),
            encryption_passphrase=memory_data.get('encryption_passphrase', '')
        )
    
    def _parse_safety(self) -> None:
        """Parse safety configuration"""
        safety_data = self.raw_config.get('safety', {})
        self.safety = SafetyConfig(
            dry_run_default=safety_data.get('dry_run_default', False),
            confirm_destructive=safety_data.get('confirm_destructive', True),
            confirm_all=safety_data.get('confirm_all', False),
            auto_backup=safety_data.get('auto_backup', True),
            blocked_patterns=safety_data.get('blocked_patterns', []),
            requires_confirmation=safety_data.get('requires_confirmation', [])
        )
    
    def _parse_logging(self) -> None:
        """Parse logging configuration"""
        logging_data = self.raw_config.get('logging', {})
        log_file = logging_data.get('file', '~/.clide/logs/clide.log')
        
        self.logging = LoggingConfig(
            level=logging_data.get('level', 'INFO'),
            file=os.path.expanduser(log_file),
            max_size=logging_data.get('max_size', '10MB'),
            backup_count=logging_data.get('backup_count', 5),
            format=logging_data.get('format', '%(asctime)s - %(name)s - %(levelname)s - %(message)s'),
            console=logging_data.get('console', True)
        )
    
    def _parse_monitoring(self) -> None:
        """Parse monitoring configuration"""
        monitoring_data = self.raw_config.get('monitoring', {})
        self.monitoring = MonitoringConfig(
            enabled=monitoring_data.get('enabled', True),
            check_interval=monitoring_data.get('check_interval', 300),
            alerts=monitoring_data.get('alerts', {}),
            notifications=monitoring_data.get('notifications', {})
        )
    
    def _parse_vps(self) -> None:
        """Parse VPS server configurations"""
        vps_data = self.raw_config.get('vps', [])
        self.vps_list = []
        
        for vps in vps_data:
            ssh_key = vps.get('ssh_key', '')
            if ssh_key:
                ssh_key = os.path.expanduser(ssh_key)
            
            self.vps_list.append(VPSConfig(
                name=vps.get('name', ''),
                host=vps.get('host', ''),
                user=vps.get('user', ''),
                port=vps.get('port', 22),
                ssh_key=ssh_key,
                description=vps.get('description', '')
            ))
    
    def _parse_workflows(self) -> None:
        """Parse workflow templates"""
        self.workflows = self.raw_config.get('workflows', {})
    
    def validate(self) -> None:
        """Validate configuration"""
        errors = []
        
        # Validate Signal config
        if not self.signal.phone_number or self.signal.phone_number == "+1234567890":
            errors.append("Signal phone number not configured (still using example value)")
        
        # Validate Gemini config
        if not self.gemini.api_key or self.gemini.api_key == "YOUR_GEMINI_API_KEY_HERE":
            errors.append("Gemini API key not configured (still using placeholder)")
        
        # Validate memory database path
        db_dir = os.path.dirname(self.memory.database_path)
        if not os.path.exists(db_dir):
            try:
                os.makedirs(db_dir, exist_ok=True)
            except Exception as e:
                errors.append(f"Cannot create database directory: {e}")
        
        # Validate logging directory
        log_dir = os.path.dirname(self.logging.file)
        if not os.path.exists(log_dir):
            try:
                os.makedirs(log_dir, exist_ok=True)
            except Exception as e:
                errors.append(f"Cannot create log directory: {e}")
        
        # Validate safety level
        if self.cline.safety_level not in ['low', 'medium', 'high']:
            errors.append(f"Invalid safety level: {self.cline.safety_level}. Must be: low, medium, or high")
        
        # Show validation warnings
        if errors:
            print("⚠️  Configuration Warnings:")
            for error in errors:
                print(f"   - {error}")
            print("\nSome features may not work until configuration is completed.")
            print("Edit config.yaml and restart clide.\n")
    
    def get_vps(self, name: str) -> Optional[VPSConfig]:
        """Get VPS configuration by name"""
        for vps in self.vps_list:
            if vps.name == name:
                return vps
        return None
    
    def reload(self) -> None:
        """Reload configuration from file"""
        self.load()


# Global configuration instance
_config_instance: Optional[Config] = None


def load_config(config_path: str = "config.yaml") -> Config:
    """Load and return global configuration instance"""
    global _config_instance
    if _config_instance is None:
        _config_instance = Config(config_path)
    return _config_instance


def get_config() -> Config:
    """Get global configuration instance"""
    global _config_instance
    if _config_instance is None:
        raise RuntimeError("Configuration not loaded. Call load_config() first.")
    return _config_instance
