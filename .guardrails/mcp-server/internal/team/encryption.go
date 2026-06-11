// Package team provides team management functionality with optional encryption.
package team

import (
	"crypto/aes"
	"crypto/cipher"
	"crypto/rand"
	"crypto/sha256"
	"encoding/base64"
	"fmt"
	"io"
	"os"
)

// EncryptionManager provides optional encryption at rest for sensitive data.
// Uses AES-GCM symmetric encryption when TEAM_ENCRYPTION_KEY env var is set.
// Encrypts sensitive fields while keeping structure readable.
type EncryptionManager struct {
	enabled bool
	key     []byte
	gcm     cipher.AEAD
}

// NewEncryptionManager initializes encryption from the TEAM_ENCRYPTION_KEY environment variable.
// If the key is 44 characters, it's treated as a base64-encoded Fernet key.
// Otherwise, the key is derived using SHA256.
func NewEncryptionManager() *EncryptionManager {
	em := &EncryptionManager{
		enabled: false,
		key:     nil,
		gcm:     nil,
	}
	em.initEncryption()
	return em
}

// initEncryption sets up the encryption key and GCM cipher from environment.
func (em *EncryptionManager) initEncryption() {
	keyStr := os.Getenv("TEAM_ENCRYPTION_KEY")
	if keyStr == "" {
		return
	}

	var key []byte

	// Check if key is already base64 encoded Fernet key (44 chars)
	if len(keyStr) == 44 {
		// Decode base64 key
		decoded, err := base64.URLEncoding.DecodeString(keyStr)
		if err != nil {
			// Try standard base64 encoding
			decoded, err = base64.StdEncoding.DecodeString(keyStr)
			if err != nil {
				fmt.Fprintf(os.Stderr, "Failed to decode encryption key: %v\n", err)
				return
			}
		}
		key = decoded
	} else {
		// Derive key from provided string using SHA256
		hash := sha256.Sum256([]byte(keyStr))
		key = hash[:]
	}

	// Ensure key is 32 bytes for AES-256
	if len(key) != 32 {
		// Hash the key to get exactly 32 bytes
		hash := sha256.Sum256(key)
		key = hash[:]
	}

	// Create AES cipher
	block, err := aes.NewCipher(key)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Failed to create AES cipher: %v\n", err)
		return
	}

	// Create GCM mode
	gcm, err := cipher.NewGCM(block)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Failed to create GCM: %v\n", err)
		return
	}

	em.key = key
	em.gcm = gcm
	em.enabled = true
}

// Enabled returns true if encryption is enabled and initialized.
func (em *EncryptionManager) Enabled() bool {
	return em.enabled
}

// Encrypt encrypts a string value using AES-GCM.
// Returns the original data if encryption is disabled or on error.
// The output is base64 encoded and contains: nonce + ciphertext + tag.
func (em *EncryptionManager) Encrypt(data string) string {
	if !em.enabled || data == "" {
		return data
	}

	// Generate random nonce
	nonce := make([]byte, em.gcm.NonceSize())
	if _, err := io.ReadFull(rand.Reader, nonce); err != nil {
		return data
	}

	// Encrypt and append tag to ciphertext
	ciphertext := em.gcm.Seal(nonce, nonce, []byte(data), nil)

	// Return base64 encoded result
	return base64.StdEncoding.EncodeToString(ciphertext)
}

// Decrypt decrypts an AES-GCM encrypted value.
// Returns the original data if decryption fails.
func (em *EncryptionManager) Decrypt(data string) string {
	if !em.enabled || data == "" {
		return data
	}

	// Decode base64
	ciphertext, err := base64.StdEncoding.DecodeString(data)
	if err != nil {
		return data
	}

	// Ensure ciphertext is long enough
	if len(ciphertext) < em.gcm.NonceSize() {
		return data
	}

	// Extract nonce and ciphertext
	nonce, ciphertext := ciphertext[:em.gcm.NonceSize()], ciphertext[em.gcm.NonceSize():]

	// Decrypt
	plaintext, err := em.gcm.Open(nil, nonce, ciphertext, nil)
	if err != nil {
		return data
	}

	return string(plaintext)
}

// EncryptDict encrypts sensitive fields in a map.
// Only encrypts string values for specified fields.
func (em *EncryptionManager) EncryptDict(data map[string]interface{}, sensitiveFields []string) map[string]interface{} {
	if !em.enabled {
		return data
	}

	result := make(map[string]interface{}, len(data))

	// Copy all values
	for k, v := range data {
		result[k] = v
	}

	// Encrypt sensitive string fields
	for _, field := range sensitiveFields {
		if val, ok := result[field]; ok {
			if strVal, ok := val.(string); ok {
				result[field] = em.Encrypt(strVal)
			}
		}
	}

	return result
}

// DecryptDict decrypts sensitive fields in a map.
// Only decrypts string values for specified fields.
func (em *EncryptionManager) DecryptDict(data map[string]interface{}, sensitiveFields []string) map[string]interface{} {
	if !em.enabled {
		return data
	}

	result := make(map[string]interface{}, len(data))

	// Copy all values
	for k, v := range data {
		result[k] = v
	}

	// Decrypt sensitive string fields
	for _, field := range sensitiveFields {
		if val, ok := result[field]; ok {
			if strVal, ok := val.(string); ok {
				result[field] = em.Decrypt(strVal)
			}
		}
	}

	return result
}
