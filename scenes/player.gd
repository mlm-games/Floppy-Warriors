extends BaseWarrior
class_name Player

# Player-specific properties or overrides can go here
# For example, if player has a different draw speed:
# const DRAW_SPEED_PLAYER: float = 70.0 

func _ready() -> void:
	super()
	

func _physics_process(delta: float) -> void:
	super(delta)
	if is_dead: return

	# Player-specific aiming logic
	if is_instance_valid(torso) and is_instance_valid(bow_pivot):
		var mouse_pos = get_global_mouse_position()
		var target_angle = (mouse_pos - torso.global_position).angle()
		bow_pivot.global_rotation = target_angle
		# Bow visual flip is handled by super._physics_process()

func _input(event: InputEvent) -> void:
	if is_dead: return
	if event.is_action_pressed("fire_bow"):
		start_drawing_bow()
	elif event.is_action_released("fire_bow"):
		release_bow()
	
	if event.is_action_pressed("airdodge"):
		for child_node in get_children():
			torso.apply_central_impulse(Vector2.UP*100)


# func get_draw_speed() -> float:
#     return DRAW_SPEED_PLAYER # Example of overriding
