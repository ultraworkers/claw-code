package web

import (
	"errors"
	"net/http"

	"github.com/labstack/echo/v4"
)

// APIError represents a structured API error response
type APIError struct {
	Status  int    `json:"-"`
	Code    string `json:"code,omitempty"`
	Message string `json:"error"`
	Details string `json:"details,omitempty"`
}

// Error implements the error interface
func (e *APIError) Error() string {
	return e.Message
}

// Common error codes for consistent API responses
const (
	ErrCodeInvalidInput       = "INVALID_INPUT"
	ErrCodeNotFound           = "NOT_FOUND"
	ErrCodeInternalError      = "INTERNAL_ERROR"
	ErrCodeUnauthorized       = "UNAUTHORIZED"
	ErrCodeForbidden          = "FORBIDDEN"
	ErrCodeRateLimited        = "RATE_LIMITED"
	ErrCodeServiceUnavailable = "SERVICE_UNAVAILABLE"
	ErrCodeConflict           = "CONFLICT"
)

// Common API errors that can be reused
var (
	ErrInvalidID = &APIError{
		Status:  http.StatusBadRequest,
		Code:    ErrCodeInvalidInput,
		Message: "Invalid ID format",
		Details: "The provided ID must be a valid UUID",
	}

	ErrInvalidRequestBody = &APIError{
		Status:  http.StatusBadRequest,
		Code:    ErrCodeInvalidInput,
		Message: "Invalid request body",
		Details: "The request body could not be parsed or contains invalid data",
	}

	ErrQueryRequired = &APIError{
		Status:  http.StatusBadRequest,
		Code:    ErrCodeInvalidInput,
		Message: "Query parameter required",
		Details: "The 'q' query parameter is required for search operations",
	}

	ErrNotFound = &APIError{
		Status:  http.StatusNotFound,
		Code:    ErrCodeNotFound,
		Message: "Resource not found",
	}

	ErrInternalServer = &APIError{
		Status:  http.StatusInternalServerError,
		Code:    ErrCodeInternalError,
		Message: "Internal server error",
		Details: "An unexpected error occurred. Please try again later.",
	}

	ErrServiceUnavailable = &APIError{
		Status:  http.StatusServiceUnavailable,
		Code:    ErrCodeServiceUnavailable,
		Message: "Service temporarily unavailable",
		Details: "The service is currently unable to handle the request. Please try again later.",
	}

	ErrCircuitOpen = &APIError{
		Status:  http.StatusServiceUnavailable,
		Code:    ErrCodeServiceUnavailable,
		Message: "Service temporarily unavailable",
		Details: "The service is experiencing issues. Please try again later.",
	}
)

// RespondWithError sends a structured error response
func RespondWithError(c echo.Context, err *APIError) error {
	return c.JSON(err.Status, err)
}

// RespondWithErrorFromMap sends an error response from a map for backward compatibility
func RespondWithErrorFromMap(c echo.Context, status int, message string) error {
	return c.JSON(status, map[string]string{"error": message})
}

// IsNotFoundError checks if an error is a "not found" error
func IsNotFoundError(err error) bool {
	if err == nil {
		return false
	}
	var apiErr *APIError
	if errors.As(err, &apiErr) {
		return apiErr.Code == ErrCodeNotFound
	}
	return false
}

// NewNotFoundError creates a new not found error for a specific resource type
func NewNotFoundError(resourceType, identifier string) *APIError {
	return &APIError{
		Status:  http.StatusNotFound,
		Code:    ErrCodeNotFound,
		Message: resourceType + " not found",
		Details: "The requested " + resourceType + " with identifier '" + identifier + "' does not exist",
	}
}

// NewValidationError creates a validation error with specific field details
func NewValidationError(field, reason string) *APIError {
	return &APIError{
		Status:  http.StatusBadRequest,
		Code:    ErrCodeInvalidInput,
		Message: "Validation failed",
		Details: field + ": " + reason,
	}
}
