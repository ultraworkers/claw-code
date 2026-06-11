## Accessibility Manager for Godot Games
##
## Enforces WCAG 3.0+ accessibility in game UIs per ACCESSIBILITY_GUIDE.md.
## AI agents: initialize this autoload in every generated game project.
class_name AccessibilityManager
extends Node

## Minimum contrast ratio (WCAG AAA)
const MIN_CONTRAST_AAA: float = 7.0
const MIN_CONTRAST_AA: float = 4.5

## Minimum touch/click target size (pixels)
const MIN_TARGET_SIZE: float = 44.0

## Focus indicator width (pixels)
const FOCUS_INDICATOR_WIDTH: float = 3.0

## Track if reduced motion is preferred
var prefers_reduced_motion: bool = false

## Track if high contrast is enabled
var high_contrast_mode: bool = false

## Track if screen reader mode is active
var screen_reader_mode: bool = false


func _ready() -> void:
	# Check OS accessibility settings where available
	_detect_accessibility_preferences()


## Calculate contrast ratio between two colors
static func contrast_ratio(fg: Color, bg: Color) -> float:
	var fg_lum := _relative_luminance(fg)
	var bg_lum := _relative_luminance(bg)
	var lighter := maxf(fg_lum, bg_lum)
	var darker := minf(fg_lum, bg_lum)
	return (lighter + 0.05) / (darker + 0.05)


## Check if color pair meets WCAG AAA
static func meets_aaa(fg: Color, bg: Color) -> bool:
	return contrast_ratio(fg, bg) >= MIN_CONTRAST_AAA


## Check if color pair meets WCAG AA
static func meets_aa(fg: Color, bg: Color) -> bool:
	return contrast_ratio(fg, bg) >= MIN_CONTRAST_AA


## Validate that a Control node meets minimum target size
static func validate_target_size(control: Control) -> bool:
	var size := control.size
	if size.x < MIN_TARGET_SIZE or size.y < MIN_TARGET_SIZE:
		push_warning("AccessibilityManager: Control '%s' size %s below minimum %dpx" % [
			control.name, size, MIN_TARGET_SIZE
		])
		return false
	return true


## Enforce minimum target size on a Control
static func enforce_target_size(control: Control) -> void:
	if control.custom_minimum_size.x < MIN_TARGET_SIZE:
		control.custom_minimum_size.x = MIN_TARGET_SIZE
	if control.custom_minimum_size.y < MIN_TARGET_SIZE:
		control.custom_minimum_size.y = MIN_TARGET_SIZE


## Add visible focus indicator to a Control
static func add_focus_indicator(control: Control, color: Color = Color.DODGER_BLUE) -> void:
	var style := StyleBoxFlat.new()
	style.border_color = color
	style.border_width_left = int(FOCUS_INDICATOR_WIDTH)
	style.border_width_right = int(FOCUS_INDICATOR_WIDTH)
	style.border_width_top = int(FOCUS_INDICATOR_WIDTH)
	style.border_width_bottom = int(FOCUS_INDICATOR_WIDTH)
	control.add_theme_stylebox_override("focus", style)


## Announce text for screen readers (Godot 4.x TTS)
func announce(text: String, interrupt: bool = false) -> void:
	if screen_reader_mode or OS.has_feature("accessibility"):
		if interrupt:
			DisplayServer.tts_stop()
		DisplayServer.tts_speak(text)


## Calculate relative luminance per WCAG formula
static func _relative_luminance(color: Color) -> float:
	var r := _linearize(color.r)
	var g := _linearize(color.g)
	var b := _linearize(color.b)
	return 0.2126 * r + 0.7152 * g + 0.0722 * b


static func _linearize(value: float) -> float:
	if value <= 0.03928:
		return value / 12.92
	return pow((value + 0.055) / 1.055, 2.4)


func _detect_accessibility_preferences() -> void:
	# Platform-specific detection
	if OS.has_feature("accessibility"):
		screen_reader_mode = true
	# Reduced motion preference (check OS where available)
	prefers_reduced_motion = false # Default; override per platform
