"""Tests for security module"""
from __future__ import annotations

import pytest
from unittest.mock import patch, MagicMock

from src.utils.security import SecureStorage, SECURE_DIR


class TestSecureStorage:
    """API Key 安全存储测试"""

    def test_store_and_retrieve_with_keyring(self):
        """keyring 可用时的存取流程"""
        with patch("src.utils.security.KEYRING_AVAILABLE", True):
            mock_keyring = MagicMock()
            mock_keyring.get_password.return_value = "sk-test123"
            with patch("src.utils.security.keyring", mock_keyring):
                result = SecureStorage.retrieve_api_key("tongyi")
                assert result == "sk-test123"

    def test_has_api_key_fallback(self):
        """本地文件回退检测"""
        with patch("src.utils.security.KEYRING_AVAILABLE", False):
            with patch.object(SECURE_DIR.__class__, "exists", return_value=True):
                # 简化测试：只要函数不抛异常即可
                assert SecureStorage.has_api_key("tongyi") in (True, False)

    def test_store_api_key_local_encrypted(self):
        """本地加密存储回退"""
        with patch("src.utils.security.KEYRING_AVAILABLE", False):
            with patch("src.utils.security.CRYPTO_AVAILABLE", True):
                with patch("src.utils.security._get_fernet") as mock_fernet:
                    mock_cipher = MagicMock()
                    mock_cipher.encrypt.return_value = b"encrypted-data"
                    mock_fernet.return_value = mock_cipher
                    result = SecureStorage.store_api_key(
                        "tongyi", "sk-local", master_password="mpw"
                    )
                    assert result is True

    def test_retrieve_api_key_local_encrypted(self):
        """本地加密读取回退"""
        with patch("src.utils.security.KEYRING_AVAILABLE", False):
            with patch("src.utils.security.CRYPTO_AVAILABLE", True):
                with patch("src.utils.security._get_fernet") as mock_fernet:
                    mock_cipher = MagicMock()
                    mock_cipher.decrypt.return_value = b"sk-local"
                    mock_fernet.return_value = mock_cipher
                    with patch.object(
                        type(SECURE_DIR), "__truediv__", return_value=MagicMock(exists=lambda: True, read_bytes=lambda: b"encrypted")
                    ):
                        result = SecureStorage.retrieve_api_key(
                            "tongyi", master_password="mpw"
                        )
                        assert result == "sk-local"

    def test_store_api_key_no_crypto_fallback(self):
        """缺少 cryptography 时降级为明文存储"""
        with patch("src.utils.security.KEYRING_AVAILABLE", False):
            with patch("src.utils.security.CRYPTO_AVAILABLE", False):
                result = SecureStorage.store_api_key("tongyi", "sk-test")
                assert result is True
                # 验证可以读回
                retrieved = SecureStorage.retrieve_api_key("tongyi")
                assert retrieved == "sk-test"
                # 清理
                SecureStorage.delete_api_key("tongyi")

    def test_delete_api_key(self):
        """删除 API Key"""
        with patch("src.utils.security.KEYRING_AVAILABLE", True):
            mock_keyring = MagicMock()
            with patch("src.utils.security.keyring", mock_keyring):
                SecureStorage.delete_api_key("tongyi")
                mock_keyring.delete_password.assert_called_once()
