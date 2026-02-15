"""
clide - Memory Module
Handles persistent storage using SQLite for conversation history,
user preferences, and command context
"""

import sqlite3
import json
import hashlib
from datetime import datetime, timedelta
from typing import List, Dict, Any, Optional, Tuple
from pathlib import Path
from cryptography.fernet import Fernet
from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.kdf.pbkdf2 import PBKDF2
import base64


class Memory:
    """Persistent memory and context management"""
    
    def __init__(self, db_path: str, encryption_passphrase: str = ""):
        self.db_path = Path(db_path).expanduser()
        self.db_path.parent.mkdir(parents=True, exist_ok=True)
        
        self.conn: Optional[sqlite3.Connection] = None
        self.cursor: Optional[sqlite3.Cursor] = None
        
        # Encryption
        self.cipher: Optional[Fernet] = None
        if encryption_passphrase:
            self._setup_encryption(encryption_passphrase)
        
        # Initialize database
        self._connect()
        self._create_tables()
    
    def _setup_encryption(self, passphrase: str) -> None:
        """Setup encryption for sensitive data"""
        # Derive key from passphrase
        salt = b'clide_salt_v1'  # In production, use random salt per user
        kdf = PBKDF2(
            algorithm=hashes.SHA256(),
            length=32,
            salt=salt,
            iterations=100000,
        )
        key = base64.urlsafe_b64encode(kdf.derive(passphrase.encode()))
        self.cipher = Fernet(key)
    
    def _encrypt(self, data: str) -> str:
        """Encrypt sensitive data"""
        if self.cipher:
            return self.cipher.encrypt(data.encode()).decode()
        return data
    
    def _decrypt(self, data: str) -> str:
        """Decrypt sensitive data"""
        if self.cipher:
            try:
                return self.cipher.decrypt(data.encode()).decode()
            except Exception:
                return data  # Return as-is if decryption fails
        return data
    
    def _connect(self) -> None:
        """Connect to SQLite database"""
        self.conn = sqlite3.connect(self.db_path)
        self.conn.row_factory = sqlite3.Row
        self.cursor = self.conn.cursor()
    
    def _create_tables(self) -> None:
        """Create database tables"""
        
        # Conversation history
        self.cursor.execute('''
            CREATE TABLE IF NOT EXISTS conversations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                role TEXT NOT NULL,
                message TEXT NOT NULL,
                context TEXT
            )
        ''')
        
        # Command execution history
        self.cursor.execute('''
            CREATE TABLE IF NOT EXISTS commands (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                command TEXT NOT NULL,
                vps TEXT,
                success BOOLEAN,
                output TEXT,
                error TEXT,
                duration REAL
            )
        ''')
        
        # User preferences
        self.cursor.execute('''
            CREATE TABLE IF NOT EXISTS preferences (
                user_id TEXT PRIMARY KEY,
                preferences TEXT NOT NULL,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        ''')
        
        # VPS configurations (cached)
        self.cursor.execute('''
            CREATE TABLE IF NOT EXISTS vps_cache (
                vps_name TEXT PRIMARY KEY,
                last_ip TEXT,
                last_connected DATETIME,
                metadata TEXT
            )
        ''')
        
        # Credentials (encrypted)
        self.cursor.execute('''
            CREATE TABLE IF NOT EXISTS credentials (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        ''')
        
        # Context snapshots
        self.cursor.execute('''
            CREATE TABLE IF NOT EXISTS context (
                user_id TEXT,
                context_key TEXT,
                context_value TEXT,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (user_id, context_key)
            )
        ''')
        
        # Create indexes
        self.cursor.execute(
            'CREATE INDEX IF NOT EXISTS idx_conversations_user_time '
            'ON conversations(user_id, timestamp DESC)'
        )
        self.cursor.execute(
            'CREATE INDEX IF NOT EXISTS idx_commands_user_time '
            'ON commands(user_id, timestamp DESC)'
        )
        
        self.conn.commit()
    
    # Conversation History
    
    def add_conversation(
        self,
        user_id: str,
        role: str,
        message: str,
        context: Optional[Dict[str, Any]] = None
    ) -> None:
        """
        Add conversation message to history
        
        Args:
            user_id: User identifier (phone number)
            role: 'user' or 'assistant'
            message: Message content
            context: Optional context data
        """
        context_json = json.dumps(context) if context else None
        
        self.cursor.execute(
            'INSERT INTO conversations (user_id, role, message, context) '
            'VALUES (?, ?, ?, ?)',
            (user_id, role, message, context_json)
        )
        self.conn.commit()
    
    def get_conversation_history(
        self,
        user_id: str,
        limit: int = 50
    ) -> List[Dict[str, Any]]:
        """
        Get conversation history for user
        
        Args:
            user_id: User identifier
            limit: Maximum number of messages to retrieve
            
        Returns:
            List of conversation messages
        """
        self.cursor.execute(
            'SELECT role, message, context, timestamp '
            'FROM conversations '
            'WHERE user_id = ? '
            'ORDER BY timestamp DESC '
            'LIMIT ?',
            (user_id, limit)
        )
        
        rows = self.cursor.fetchall()
        history = []
        
        for row in reversed(rows):  # Reverse to get chronological order
            history.append({
                'role': row['role'],
                'message': row['message'],
                'context': json.loads(row['context']) if row['context'] else None,
                'timestamp': row['timestamp']
            })
        
        return history
    
    # Command History
    
    def add_command(
        self,
        user_id: str,
        command: str,
        vps: Optional[str] = None,
        success: bool = False,
        output: str = "",
        error: str = "",
        duration: float = 0.0
    ) -> None:
        """
        Log command execution
        
        Args:
            user_id: User who executed command
            command: Command that was executed
            vps: Target VPS (if any)
            success: Whether execution succeeded
            output: Command output
            error: Error message (if failed)
            duration: Execution time in seconds
        """
        self.cursor.execute(
            'INSERT INTO commands '
            '(user_id, command, vps, success, output, error, duration) '
            'VALUES (?, ?, ?, ?, ?, ?, ?)',
            (user_id, command, vps, success, output, error, duration)
        )
        self.conn.commit()
    
    def get_command_history(
        self,
        user_id: str,
        limit: int = 100,
        vps: Optional[str] = None
    ) -> List[Dict[str, Any]]:
        """
        Get command execution history
        
        Args:
            user_id: User identifier
            limit: Maximum number of commands
            vps: Filter by VPS (optional)
            
        Returns:
            List of command records
        """
        if vps:
            self.cursor.execute(
                'SELECT * FROM commands '
                'WHERE user_id = ? AND vps = ? '
                'ORDER BY timestamp DESC '
                'LIMIT ?',
                (user_id, vps, limit)
            )
        else:
            self.cursor.execute(
                'SELECT * FROM commands '
                'WHERE user_id = ? '
                'ORDER BY timestamp DESC '
                'LIMIT ?',
                (user_id, limit)
            )
        
        rows = self.cursor.fetchall()
        return [dict(row) for row in rows]
    
    def get_last_command(self, user_id: str) -> Optional[str]:
        """Get the last command executed by user"""
        self.cursor.execute(
            'SELECT command FROM commands '
            'WHERE user_id = ? '
            'ORDER BY timestamp DESC '
            'LIMIT 1',
            (user_id,)
        )
        row = self.cursor.fetchone()
        return row['command'] if row else None
    
    # User Preferences
    
    def set_preferences(self, user_id: str, preferences: Dict[str, Any]) -> None:
        """Save user preferences"""
        prefs_json = json.dumps(preferences)
        
        self.cursor.execute(
            'INSERT OR REPLACE INTO preferences (user_id, preferences, updated_at) '
            'VALUES (?, ?, CURRENT_TIMESTAMP)',
            (user_id, prefs_json)
        )
        self.conn.commit()
    
    def get_preferences(self, user_id: str) -> Dict[str, Any]:
        """Get user preferences"""
        self.cursor.execute(
            'SELECT preferences FROM preferences WHERE user_id = ?',
            (user_id,)
        )
        row = self.cursor.fetchone()
        
        if row:
            return json.loads(row['preferences'])
        return {}
    
    def update_preference(self, user_id: str, key: str, value: Any) -> None:
        """Update a single preference"""
        prefs = self.get_preferences(user_id)
        prefs[key] = value
        self.set_preferences(user_id, prefs)
    
    # Context Management
    
    def set_context(self, user_id: str, key: str, value: Any) -> None:
        """Set context value for user"""
        value_json = json.dumps(value)
        
        self.cursor.execute(
            'INSERT OR REPLACE INTO context (user_id, context_key, context_value, timestamp) '
            'VALUES (?, ?, ?, CURRENT_TIMESTAMP)',
            (user_id, key, value_json)
        )
        self.conn.commit()
    
    def get_context(self, user_id: str, key: str) -> Optional[Any]:
        """Get context value for user"""
        self.cursor.execute(
            'SELECT context_value FROM context '
            'WHERE user_id = ? AND context_key = ?',
            (user_id, key)
        )
        row = self.cursor.fetchone()
        
        if row:
            return json.loads(row['context_value'])
        return None
    
    def get_all_context(self, user_id: str) -> Dict[str, Any]:
        """Get all context for user"""
        self.cursor.execute(
            'SELECT context_key, context_value FROM context WHERE user_id = ?',
            (user_id,)
        )
        rows = self.cursor.fetchall()
        
        context = {}
        for row in rows:
            context[row['context_key']] = json.loads(row['context_value'])
        
        return context
    
    def clear_context(self, user_id: str, key: Optional[str] = None) -> None:
        """Clear context for user"""
        if key:
            self.cursor.execute(
                'DELETE FROM context WHERE user_id = ? AND context_key = ?',
                (user_id, key)
            )
        else:
            self.cursor.execute(
                'DELETE FROM context WHERE user_id = ?',
                (user_id,)
            )
        self.conn.commit()
    
    # Credentials (Encrypted)
    
    def store_credential(self, key: str, value: str) -> None:
        """Store encrypted credential"""
        encrypted_value = self._encrypt(value)
        
        self.cursor.execute(
            'INSERT OR REPLACE INTO credentials (key, value, created_at) '
            'VALUES (?, ?, CURRENT_TIMESTAMP)',
            (key, encrypted_value)
        )
        self.conn.commit()
    
    def get_credential(self, key: str) -> Optional[str]:
        """Get decrypted credential"""
        self.cursor.execute(
            'SELECT value FROM credentials WHERE key = ?',
            (key,)
        )
        row = self.cursor.fetchone()
        
        if row:
            return self._decrypt(row['value'])
        return None
    
    def delete_credential(self, key: str) -> None:
        """Delete credential"""
        self.cursor.execute('DELETE FROM credentials WHERE key = ?', (key,))
        self.conn.commit()
    
    # VPS Cache
    
    def update_vps_cache(
        self,
        vps_name: str,
        ip: str,
        metadata: Optional[Dict[str, Any]] = None
    ) -> None:
        """Update VPS connection cache"""
        metadata_json = json.dumps(metadata) if metadata else None
        
        self.cursor.execute(
            'INSERT OR REPLACE INTO vps_cache '
            '(vps_name, last_ip, last_connected, metadata) '
            'VALUES (?, ?, CURRENT_TIMESTAMP, ?)',
            (vps_name, ip, metadata_json)
        )
        self.conn.commit()
    
    def get_vps_cache(self, vps_name: str) -> Optional[Dict[str, Any]]:
        """Get VPS cache info"""
        self.cursor.execute(
            'SELECT * FROM vps_cache WHERE vps_name = ?',
            (vps_name,)
        )
        row = self.cursor.fetchone()
        
        if row:
            return {
                'vps_name': row['vps_name'],
                'last_ip': row['last_ip'],
                'last_connected': row['last_connected'],
                'metadata': json.loads(row['metadata']) if row['metadata'] else None
            }
        return None
    
    # Cleanup
    
    def cleanup_old_data(self, days: int = 90) -> Tuple[int, int]:
        """
        Delete data older than specified days
        
        Args:
            days: Delete data older than this many days
            
        Returns:
            Tuple of (conversations_deleted, commands_deleted)
        """
        cutoff_date = datetime.now() - timedelta(days=days)
        cutoff_str = cutoff_date.strftime('%Y-%m-%d %H:%M:%S')
        
        # Delete old conversations
        self.cursor.execute(
            'DELETE FROM conversations WHERE timestamp < ?',
            (cutoff_str,)
        )
        conversations_deleted = self.cursor.rowcount
        
        # Delete old commands
        self.cursor.execute(
            'DELETE FROM commands WHERE timestamp < ?',
            (cutoff_str,)
        )
        commands_deleted = self.cursor.rowcount
        
        self.conn.commit()
        
        return conversations_deleted, commands_deleted
    
    def vacuum(self) -> None:
        """Optimize database (reclaim space)"""
        self.cursor.execute('VACUUM')
        self.conn.commit()
    
    # Statistics
    
    def get_stats(self, user_id: str) -> Dict[str, Any]:
        """Get usage statistics for user"""
        # Total conversations
        self.cursor.execute(
            'SELECT COUNT(*) as count FROM conversations WHERE user_id = ?',
            (user_id,)
        )
        total_conversations = self.cursor.fetchone()['count']
        
        # Total commands
        self.cursor.execute(
            'SELECT COUNT(*) as count FROM commands WHERE user_id = ?',
            (user_id,)
        )
        total_commands = self.cursor.fetchone()['count']
        
        # Successful commands
        self.cursor.execute(
            'SELECT COUNT(*) as count FROM commands WHERE user_id = ? AND success = 1',
            (user_id,)
        )
        successful_commands = self.cursor.fetchone()['count']
        
        # First interaction
        self.cursor.execute(
            'SELECT MIN(timestamp) as first FROM conversations WHERE user_id = ?',
            (user_id,)
        )
        first_interaction = self.cursor.fetchone()['first']
        
        success_rate = 0
        if total_commands > 0:
            success_rate = (successful_commands / total_commands) * 100
        
        return {
            'total_conversations': total_conversations,
            'total_commands': total_commands,
            'successful_commands': successful_commands,
            'success_rate': round(success_rate, 2),
            'first_interaction': first_interaction
        }
    
    def close(self) -> None:
        """Close database connection"""
        if self.conn:
            self.conn.close()
    
    def __enter__(self):
        return self
    
    def __exit__(self, exc_type, exc_val, exc_tb):
        self.close()
