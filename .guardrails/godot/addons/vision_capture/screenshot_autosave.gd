@tool
extends Node

## Auto-saves screenshots to user://screenshots/ for the guardrail vision pipeline.

@export var enabled: bool = true
@export var capture_interval_seconds: float = 30.0
@export var output_directory: String = "user://screenshots"

var _timer: Timer
var _capture_count: int = 0

func _ready() -> void:
	if not enabled:
		return
	var dir = DirAccess.open("user://")
	if dir:
		dir.make_dir_recursive("screenshots")

	_timer = Timer.new()
	_timer.wait_time = capture_interval_seconds
	_timer.autostart = true
	_timer.timeout.connect(_on_timer_timeout)
	add_child(_timer)
	print("[VisionCapture] Auto-capture enabled every ", capture_interval_seconds, "s")

func _on_timer_timeout() -> void:
	_capture_screenshot()

func capture_now() -> String:
	return _capture_screenshot()

func _capture_screenshot() -> String:
	var vp = get_viewport()
	var tex = vp.get_texture()
	var img = tex.get_image()
	if not img:
		push_error("[VisionCapture] Failed to get viewport image")
		return ""

	var timestamp = Time.get_datetime_string_from_system().replace(":", "-")
	_capture_count += 1
	var filename = "screenshot_%s_%04d.png" % [timestamp, _capture_count]
	var path = output_directory.path_join(filename)

	var err = img.save_png(path)
	if err == OK:
		print("[VisionCapture] Saved: ", path)
		return path
	else:
		push_error("[VisionCapture] Failed to save screenshot: " + str(err))
		return ""
