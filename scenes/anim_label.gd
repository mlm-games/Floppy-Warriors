@tool
extends RichTextLabel
class_name AnimLabel

@export var pulse_scale: Vector2 = Vector2(0.98, 1.1)
@export var pulse_speed: float = 4.0
@export var color_gradient: Gradient

var _time: float = 0.0

func _ready() -> void:
	pivot_offset = size/2
	if Engine.is_editor_hint(): resized.connect(func(): pivot_offset = size/2)

func _process(delta: float) -> void:
	_time += delta
	
	# Smooth breathing? effect (just use tweens)
	var t = (sin(_time * pulse_speed) + 1.0) * 0.5
	
	# Scale animation
	scale = Vector2.ONE.lerp(pulse_scale, t)
	
	# Color animation
	if color_gradient:
		modulate = color_gradient.sample(t)
