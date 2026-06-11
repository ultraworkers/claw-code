package models

// ResolutionStatus represents the resolution state of a task or halt event
type ResolutionStatus string

const (
	ResolutionPending    ResolutionStatus = "pending"
	ResolutionResolved   ResolutionStatus = "resolved"
	ResolutionEscalated  ResolutionStatus = "escalated"
	ResolutionDismissed  ResolutionStatus = "dismissed"
	ResolutionAbandoned  ResolutionStatus = "abandoned"
)

// ValidResolutionStatuses contains all valid resolution statuses
var ValidResolutionStatuses = []string{
	string(ResolutionPending),
	string(ResolutionResolved),
	string(ResolutionEscalated),
	string(ResolutionDismissed),
	string(ResolutionAbandoned),
}

// IsValidResolutionStatus checks if a resolution status is valid
func IsValidResolutionStatus(s string) bool {
	for _, rs := range ValidResolutionStatuses {
		if rs == s {
			return true
		}
	}
	return false
}
