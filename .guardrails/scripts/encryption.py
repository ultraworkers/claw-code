#!/usr/bin/env python3
"""Encryption module for team data at rest (SEC-007).

Uses Fernet symmetric encryption from the cryptography library.
Key is stored in TEAM_ENCRYPTION_KEY environment variable.
"""

import os
from typing import Any, Dict, Optional


class EncryptionManager:
    """Manages optional encryption at rest for sensitive data.

    Encrypts: role assignments, person names, audit logs
    Keeps structure unencrypted: teams, phases
    """

    ENCRYPTED_MARKER = "__encrypted__"
    ENCRYPTED_PREFIX = "ENC:"

    def __init__(self, key: str = None):
        """Initialize encryption manager.

        Args:
            key: Fernet key (base64-encoded). If None, reads from TEAM_ENCRYPTION_KEY env var.
        """
        self._fernet = None
        self._enabled = False

        key = key or os.environ.get("TEAM_ENCRYPTION_KEY")
        if key:
            try:
                from cryptography.fernet import Fernet
                self._fernet = Fernet(key)
                self._enabled = True
            except Exception as e:
                print(f"⚠️  Failed to initialize encryption: {e}", file=os.sys.stderr)

    @property
    def enabled(self) -> bool:
        """Check if encryption is enabled and configured."""
        return self._enabled

    def encrypt_value(self, value: str) -> str:
        """Encrypt a single string value."""
        if not self._enabled or value is None:
            return value
        if value.startswith(self.ENCRYPTED_PREFIX):
            return value
        encrypted = self._fernet.encrypt(value.encode()).decode()
        return f"{self.ENCRYPTED_PREFIX}{encrypted}"

    def decrypt_value(self, value: str) -> str:
        """Decrypt a single string value."""
        if not self._enabled or value is None:
            return value
        if not value.startswith(self.ENCRYPTED_PREFIX):
            return value
        encrypted = value[len(self.ENCRYPTED_PREFIX):]
        try:
            return self._fernet.decrypt(encrypted.encode()).decode()
        except Exception:
            return value

    def encrypt_data(self, data: Dict[str, Any]) -> Dict[str, Any]:
        """Encrypt sensitive fields in team data."""
        if not self._enabled:
            return data
        encrypted = {}
        for key, value in data.items():
            if key in ("assigned_to", "person", "user", "assignee") and isinstance(value, str):
                encrypted[key] = self.encrypt_value(value)
            elif key == "roles" and isinstance(value, list):
                encrypted[key] = [self._encrypt_role(role) for role in value]
            elif key == "details" and isinstance(value, dict):
                encrypted[key] = self._encrypt_details(value)
            elif isinstance(value, dict):
                encrypted[key] = self.encrypt_data(value)
            elif isinstance(value, list):
                encrypted[key] = [self.encrypt_data(item) if isinstance(item, dict) else item for item in value]
            else:
                encrypted[key] = value
        encrypted[self.ENCRYPTED_MARKER] = True
        return encrypted

    def decrypt_data(self, data: Dict[str, Any]) -> Dict[str, Any]:
        """Decrypt sensitive fields in team data."""
        if not self._enabled:
            return data
        decrypted = {}
        for key, value in data.items():
            if key == self.ENCRYPTED_MARKER:
                continue
            elif key in ("assigned_to", "person", "user", "assignee") and isinstance(value, str):
                decrypted[key] = self.decrypt_value(value)
            elif key == "roles" and isinstance(value, list):
                decrypted[key] = [self._decrypt_role(role) for role in value]
            elif key == "details" and isinstance(value, dict):
                decrypted[key] = self._decrypt_details(value)
            elif isinstance(value, dict):
                decrypted[key] = self.decrypt_data(value)
            elif isinstance(value, list):
                decrypted[key] = [self.decrypt_data(item) if isinstance(item, dict) else item for item in value]
            else:
                decrypted[key] = value
        return decrypted

    def _encrypt_role(self, role: Dict[str, Any]) -> Dict[str, Any]:
        encrypted = dict(role)
        if "assigned_to" in encrypted and encrypted["assigned_to"]:
            encrypted["assigned_to"] = self.encrypt_value(encrypted["assigned_to"])
        return encrypted

    def _decrypt_role(self, role: Dict[str, Any]) -> Dict[str, Any]:
        decrypted = dict(role)
        if "assigned_to" in decrypted and decrypted["assigned_to"]:
            decrypted["assigned_to"] = self.decrypt_value(decrypted["assigned_to"])
        return decrypted

    def _encrypt_details(self, details: Dict[str, Any]) -> Dict[str, Any]:
        encrypted = {}
        for key, value in details.items():
            if key in ("assignee", "before", "after", "user") and isinstance(value, str):
                encrypted[key] = self.encrypt_value(value)
            elif isinstance(value, dict):
                encrypted[key] = self._encrypt_details(value)
            else:
                encrypted[key] = value
        return encrypted

    def _decrypt_details(self, details: Dict[str, Any]) -> Dict[str, Any]:
        decrypted = {}
        for key, value in details.items():
            if key in ("assignee", "before", "after", "user") and isinstance(value, str):
                decrypted[key] = self.decrypt_value(value)
            elif isinstance(value, dict):
                decrypted[key] = self._decrypt_details(value)
            else:
                decrypted[key] = value
        return decrypted

    def is_encrypted(self, data: Dict[str, Any]) -> bool:
        """Check if data is marked as encrypted."""
        return data.get(self.ENCRYPTED_MARKER, False)


def generate_encryption_key() -> str:
    """Generate a new Fernet encryption key.

    Returns:
        Base64-encoded Fernet key suitable for TEAM_ENCRYPTION_KEY env var.
    """
    from cryptography.fernet import Fernet
    return Fernet.generate_key().decode()


if __name__ == "__main__":
    key = generate_encryption_key()
    print(f"Generated encryption key: {key}")
    print("Set this as TEAM_ENCRYPTION_KEY environment variable to enable encryption")
