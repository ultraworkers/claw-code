## Comfort Zone Enforcement for XR/VR
##
## Enforces spatial safety rules from SPATIAL_COMPUTING_UI.md.
## AI agents: use this when generating any XR/VR UI placement.
class_name ComfortZone
extends Node

## Maximum angle from center for UI placement (degrees)
const COMFORT_CONE_ANGLE: float = 30.0

## Maximum motion-to-photon latency (milliseconds)
const MAX_LATENCY_MS: float = 20.0

## Maximum virtual acceleration (g-force)
const MAX_ACCELERATION_G: float = 1.5

## Maximum continuous session duration (minutes)
const MAX_SESSION_MINUTES: int = 60

## Minimum framerate for VR comfort
const MIN_FPS: int = 72
const TARGET_FPS: int = 90

var session_start_time: float = 0.0
var session_warned: bool = false


func _ready() -> void:
	session_start_time = Time.get_ticks_msec() / 1000.0


func _process(_delta: float) -> void:
	_check_session_duration()
	_check_framerate()


## Validate that a UI element position is within the comfort cone
func is_in_comfort_zone(world_position: Vector3, head_position: Vector3, head_forward: Vector3) -> bool:
	var to_element := (world_position - head_position).normalized()
	var angle := rad_to_deg(acos(to_element.dot(head_forward)))
	return angle <= COMFORT_CONE_ANGLE


## Clamp a position to the comfort zone boundary
func clamp_to_comfort_zone(world_position: Vector3, head_position: Vector3, head_forward: Vector3, distance: float = 2.0) -> Vector3:
	if is_in_comfort_zone(world_position, head_position, head_forward):
		return world_position
	# Project onto comfort cone boundary
	var to_element := (world_position - head_position).normalized()
	var clamped := head_forward.slerp(to_element, COMFORT_CONE_ANGLE / rad_to_deg(acos(to_element.dot(head_forward))))
	return head_position + clamped * distance


## Check if acceleration is within safe bounds
func is_safe_acceleration(acceleration: Vector3) -> bool:
	return acceleration.length() / 9.81 <= MAX_ACCELERATION_G


## Monitor session duration and warn at limit
func _check_session_duration() -> void:
	var elapsed := (Time.get_ticks_msec() / 1000.0 - session_start_time) / 60.0
	if elapsed >= MAX_SESSION_MINUTES and not session_warned:
		session_warned = true
		_show_break_reminder()


## Monitor framerate for VR comfort
func _check_framerate() -> void:
	var fps := Engine.get_frames_per_second()
	if fps < MIN_FPS:
		push_warning("ComfortZone: FPS %d below minimum %d — motion sickness risk" % [fps, MIN_FPS])


## Show a non-intrusive break reminder
func _show_break_reminder() -> void:
	# Emit signal for UI layer to display reminder
	# Do NOT force-pause — that's a dark pattern
	push_warning("ComfortZone: Session duration %d minutes — suggest break" % MAX_SESSION_MINUTES)
