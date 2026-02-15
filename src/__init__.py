"""
clide - Glide through your CLI
Autonomous terminal operations from your pocket

This package provides:
- Signal bot integration for natural language commands
- Gemini Flash AI for command interpretation
- Autonomous execution with safety guardrails
- Persistent memory and context management
"""

__version__ = "0.1.0-alpha"
__author__ = "clide contributors"
__license__ = "MIT"

# Export main components for easier imports
from .config import Config, load_config, get_config
from .logger import get_logger, setup_logger
from .memory import Memory
from .brain import Brain
from .executor import Executor
from .safety import Safety, SafetyResult
from .bot import ClideBot

__all__ = [
    # Configuration
    'Config',
    'load_config',
    'get_config',
    
    # Logging
    'get_logger',
    'setup_logger',
    
    # Core components
    'Memory',
    'Brain',
    'Executor',
    'Safety',
    'SafetyResult',
    'ClideBot',
    
    # Metadata
    '__version__',
    '__author__',
    '__license__',
]
