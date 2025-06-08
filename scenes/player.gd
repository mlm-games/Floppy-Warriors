extends BaseWarrior

# Player-specific properties or overrides can go here
# For example, if player has a different draw speed:
# const DRAW_SPEED_PLAYER: float = 70.0 

func _ready() -> void:
	super() # Calls BaseWarrior._ready()
	# Any player-specific _ready() logic here

func _physics_process(delta: float) -> void:
	super(delta) # Calls BaseWarrior._physics_process()
	if is_dead: return

	# Player-specific aiming logic
	if is_instance_valid(torso) and is_instance_valid(bow_pivot):
		var mouse_pos = get_global_mouse_position()
		var target_angle = (mouse_pos - torso.global_position).angle()
		bow_pivot.global_rotation = target_angle
		# Bow visual flip is handled by super._physics_process()

func _input(event: InputEvent) -> void:
	# super(event) # BaseWarrior doesn't have _input, so no super call needed here
	if is_dead: return

	if event.is_action_pressed("fire_bow"):
		start_drawing_bow() # Call method from BaseWarrior
	elif event.is_action_released("fire_bow"):
		release_bow() # Call method from BaseWarrior

# func get_draw_speed() -> float:
#     return DRAW_SPEED_PLAYER # Example of overriding
