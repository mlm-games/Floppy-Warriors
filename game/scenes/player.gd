class_name Player
extends BaseWarrior

const AIM_SPEED: float = 2.0

@export var airdodge_impulse: float = 100.0
@export var airdodge_cooldown: float = 1.0

var current_aim_angle: float = 0.0
var using_mouse: bool = true
var input_enabled: bool = true
var airdodge_cooldown_remaining: float = 0.0


func _ready() -> void:
	super()
	current_aim_angle = bow_pivot.global_rotation


func _physics_process(delta: float) -> void:
	airdodge_cooldown_remaining = maxf(
		0.0,
		airdodge_cooldown_remaining - delta
	)

	if not is_dead and input_enabled:
		_update_aim()

	super(delta)


func _update_aim() -> void:
	var aim_input := Vector2(
		Input.get_axis("aim_left", "aim_right"),
		Input.get_axis("aim_up", "aim_down")
	)

	if aim_input.length() > 0.1:
		using_mouse = false
		current_aim_angle = aim_input.angle()
		bow_pivot.global_rotation = current_aim_angle
	elif using_mouse:
		var mouse_position := get_global_mouse_position()
		current_aim_angle = (
			mouse_position - torso.global_position
		).angle()
		bow_pivot.global_rotation = current_aim_angle


func _input(event: InputEvent) -> void:
	if is_dead or not input_enabled:
		return

	if event is InputEventMouseMotion:
		using_mouse = true

	if event.is_action_pressed("fire_bow"):
		start_drawing_bow()
	elif event.is_action_released("fire_bow"):
		release_bow()

	if (
		event.is_action_pressed("airdodge")
		and airdodge_cooldown_remaining <= 0.0
	):
		perform_airdodge()


func perform_airdodge() -> void:
	if is_dead or not is_instance_valid(torso):
		return

	airdodge_cooldown_remaining = maxf(0.05, airdodge_cooldown)
	torso.sleeping = false
	torso.apply_central_impulse(Vector2.UP * airdodge_impulse)


func set_input_enabled(enabled: bool) -> void:
	input_enabled = enabled

	if not input_enabled:
		cancel_bow_draw()