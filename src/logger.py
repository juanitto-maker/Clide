"""
clide - Logging Module
Handles structured logging with file rotation and console output
"""

import logging
import sys
from pathlib import Path
from logging.handlers import RotatingFileHandler
from typing import Optional
from datetime import datetime


class ClideLogger:
    """Custom logger for clide with flight-themed messages"""
    
    def __init__(self, name: str = "clide"):
        self.name = name
        self.logger = logging.getLogger(name)
        self._initialized = False
    
    def setup(
        self,
        log_file: str,
        level: str = "INFO",
        max_size: str = "10MB",
        backup_count: int = 5,
        log_format: str = "%(asctime)s - %(name)s - %(levelname)s - %(message)s",
        console: bool = True
    ) -> None:
        """
        Setup logger with file and console handlers
        
        Args:
            log_file: Path to log file
            level: Logging level (DEBUG, INFO, WARNING, ERROR, CRITICAL)
            max_size: Max size before rotation (e.g., "10MB")
            backup_count: Number of backup files to keep
            log_format: Log message format
            console: Whether to also log to console
        """
        if self._initialized:
            return
        
        # Parse log level
        numeric_level = getattr(logging, level.upper(), logging.INFO)
        self.logger.setLevel(numeric_level)
        
        # Create log directory if it doesn't exist
        log_path = Path(log_file).expanduser()
        log_path.parent.mkdir(parents=True, exist_ok=True)
        
        # Parse max size
        size_bytes = self._parse_size(max_size)
        
        # File handler with rotation
        file_handler = RotatingFileHandler(
            log_path,
            maxBytes=size_bytes,
            backupCount=backup_count,
            encoding='utf-8'
        )
        file_handler.setLevel(numeric_level)
        file_formatter = logging.Formatter(log_format)
        file_handler.setFormatter(file_formatter)
        self.logger.addHandler(file_handler)
        
        # Console handler (if enabled)
        if console:
            console_handler = logging.StreamHandler(sys.stdout)
            console_handler.setLevel(numeric_level)
            console_formatter = ColoredFormatter(log_format)
            console_handler.setFormatter(console_formatter)
            self.logger.addHandler(console_handler)
        
        self._initialized = True
        self.logger.info("🛫 clide logger initialized")
    
    def _parse_size(self, size_str: str) -> int:
        """Parse size string (e.g., '10MB') to bytes"""
        size_str = size_str.upper().strip()
        
        if size_str.endswith('KB'):
            return int(size_str[:-2]) * 1024
        elif size_str.endswith('MB'):
            return int(size_str[:-2]) * 1024 * 1024
        elif size_str.endswith('GB'):
            return int(size_str[:-2]) * 1024 * 1024 * 1024
        else:
            return int(size_str)
    
    # Flight-themed logging methods
    
    def takeoff(self, message: str) -> None:
        """Log start of operation"""
        self.logger.info(f"🛫 {message}")
    
    def landing(self, message: str) -> None:
        """Log successful completion"""
        self.logger.info(f"🛬 {message}")
    
    def turbulence(self, message: str) -> None:
        """Log warnings"""
        self.logger.warning(f"⚠️  {message}")
    
    def crash(self, message: str, exc_info: bool = False) -> None:
        """Log errors"""
        self.logger.error(f"❌ {message}", exc_info=exc_info)
    
    def altitude(self, message: str) -> None:
        """Log debug information"""
        self.logger.debug(f"📊 {message}")
    
    # Standard logging methods
    
    def debug(self, message: str) -> None:
        """Debug level log"""
        self.logger.debug(message)
    
    def info(self, message: str) -> None:
        """Info level log"""
        self.logger.info(message)
    
    def warning(self, message: str) -> None:
        """Warning level log"""
        self.logger.warning(message)
    
    def error(self, message: str, exc_info: bool = False) -> None:
        """Error level log"""
        self.logger.error(message, exc_info=exc_info)
    
    def critical(self, message: str, exc_info: bool = False) -> None:
        """Critical level log"""
        self.logger.critical(message, exc_info=exc_info)
    
    # Command logging (audit trail)
    
    def log_command(
        self,
        command: str,
        source: str = "signal",
        user: str = "unknown",
        vps: Optional[str] = None
    ) -> None:
        """
        Log command execution for audit trail
        
        Args:
            command: The command being executed
            source: Source of command (signal, telegram, etc.)
            user: User who issued command
            vps: Target VPS (if any)
        """
        vps_info = f" on {vps}" if vps else ""
        self.logger.info(
            f"COMMAND [{source}] {user}{vps_info}: {command}"
        )
    
    def log_result(
        self,
        command: str,
        success: bool,
        output: str = "",
        error: str = "",
        duration: float = 0.0
    ) -> None:
        """
        Log command execution result
        
        Args:
            command: The command that was executed
            success: Whether execution succeeded
            output: Command output
            error: Error message (if failed)
            duration: Execution time in seconds
        """
        status = "✓ SUCCESS" if success else "✗ FAILED"
        self.logger.info(
            f"RESULT [{status}] {command} ({duration:.2f}s)"
        )
        
        if output:
            self.logger.debug(f"OUTPUT: {output[:500]}")  # Truncate long output
        
        if error:
            self.logger.error(f"ERROR: {error}")
    
    def log_safety_check(
        self,
        command: str,
        is_safe: bool,
        reason: str = ""
    ) -> None:
        """
        Log safety check results
        
        Args:
            command: Command being checked
            is_safe: Whether command passed safety checks
            reason: Reason for blocking (if unsafe)
        """
        if is_safe:
            self.logger.debug(f"SAFETY [✓ PASS] {command}")
        else:
            self.logger.warning(f"SAFETY [✗ BLOCKED] {command} - Reason: {reason}")


class ColoredFormatter(logging.Formatter):
    """Formatter that adds colors to console output"""
    
    # ANSI color codes
    COLORS = {
        'DEBUG': '\033[0;36m',      # Cyan
        'INFO': '\033[0;32m',       # Green
        'WARNING': '\033[1;33m',    # Yellow
        'ERROR': '\033[0;31m',      # Red
        'CRITICAL': '\033[1;35m',   # Magenta
    }
    RESET = '\033[0m'
    
    def format(self, record):
        # Add color to level name
        levelname = record.levelname
        if levelname in self.COLORS:
            record.levelname = f"{self.COLORS[levelname]}{levelname}{self.RESET}"
        
        # Format the message
        result = super().format(record)
        
        # Reset color
        record.levelname = levelname
        
        return result


class AuditLogger:
    """Separate audit logger for compliance and security"""
    
    def __init__(self, audit_file: str = "~/.clide/logs/audit.log"):
        self.audit_file = Path(audit_file).expanduser()
        self.audit_file.parent.mkdir(parents=True, exist_ok=True)
        
        self.logger = logging.getLogger("clide.audit")
        self.logger.setLevel(logging.INFO)
        
        # Audit logs should never rotate (keep everything)
        handler = logging.FileHandler(self.audit_file, encoding='utf-8')
        formatter = logging.Formatter(
            '%(asctime)s | %(message)s',
            datefmt='%Y-%m-%d %H:%M:%S'
        )
        handler.setFormatter(formatter)
        self.logger.addHandler(handler)
    
    def log(
        self,
        event_type: str,
        user: str,
        action: str,
        details: str = "",
        success: bool = True
    ) -> None:
        """
        Log audit event
        
        Args:
            event_type: Type of event (COMMAND, CONFIG, AUTH, etc.)
            user: User performing action
            action: Action performed
            details: Additional details
            success: Whether action succeeded
        """
        status = "SUCCESS" if success else "FAILED"
        message = f"{event_type} | {user} | {action} | {status}"
        
        if details:
            message += f" | {details}"
        
        self.logger.info(message)


# Global logger instances
_main_logger: Optional[ClideLogger] = None
_audit_logger: Optional[AuditLogger] = None


def setup_logger(
    log_file: str,
    level: str = "INFO",
    max_size: str = "10MB",
    backup_count: int = 5,
    log_format: str = "%(asctime)s - %(name)s - %(levelname)s - %(message)s",
    console: bool = True
) -> ClideLogger:
    """Setup and return main logger"""
    global _main_logger
    if _main_logger is None:
        _main_logger = ClideLogger()
    _main_logger.setup(log_file, level, max_size, backup_count, log_format, console)
    return _main_logger


def get_logger() -> ClideLogger:
    """Get main logger instance"""
    global _main_logger
    if _main_logger is None:
        _main_logger = ClideLogger()
    return _main_logger


def get_audit_logger() -> AuditLogger:
    """Get audit logger instance"""
    global _audit_logger
    if _audit_logger is None:
        _audit_logger = AuditLogger()
    return _audit_logger
