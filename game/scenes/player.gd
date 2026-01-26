extends BaseWarrior
class_name Player

# Player-specific properties or overrides (later)
# const DRAW_SPEED_PLAYER: float = 70.0 

const AIM_SPEED: float = 2.0 # Radians per second for keyboard/controller aiming
var current_aim_angle: float = 0.0
var using_mouse: bool = true

func _ready() -> void:
	super ()
	current_aim_angle = bow_pivot.global_rotation

## Player-specific aiming logic
func _physics_process(delta: float) -> void:
	super (delta)
	
	# Get keyboard/controller aim input
	var aim_input := Vector2.ZERO
	aim_input.x = Input.get_axis("aim_left", "aim_right")
	aim_input.y = Input.get_axis("aim_up", "aim_down")
	
	# Check if using controller/keyboard or mouse
	if aim_input.length() > 0.1:
		using_mouse = false
		current_aim_angle = aim_input.angle()
		bow_pivot.global_rotation = current_aim_angle
	elif using_mouse:
		# Mouse aiming
		var mouse_pos = get_global_mouse_position()
		var target_angle = (mouse_pos - torso.global_position).angle()
		bow_pivot.global_rotation = target_angle
		current_aim_angle = target_angle
		# Bow visual flip is handled by super._physics_process()

func _input(event: InputEvent) -> void:
	if is_dead: return
	
	# Detect mouse movement to switch back to mouse mode
	if event is InputEventMouseMotion:
		using_mouse = true
	
	if event.is_action_pressed("fire_bow"):
		start_drawing_bow()
	elif event.is_action_released("fire_bow"):
		release_bow()
	
	if event.is_action_pressed("airdodge"):
		for child_node in get_children():
			torso.apply_central_impulse(Vector2.UP * 100)


# func get_draw_speed() -> float:
#     return DRAW_SPEED_PLAYER
