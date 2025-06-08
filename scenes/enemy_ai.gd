extends BaseWarrior

var player_node: Node2D = null # Target player

var aim_timer: Timer
var decision_timer: Timer

var current_target_pos: Vector2

# AI specific draw speed or other parameters can be defined here
# const AI_DRAW_SPEED_MULTIPLIER = 0.8 

func _ready() -> void:
	super() # Calls BaseWarrior._ready()

	aim_timer = Timer.new()
	aim_timer.name = "AIAimTimer" # Good practice for debugging
	aim_timer.wait_time = 0.1 # How often to update aim
	aim_timer.timeout.connect(_update_aim)
	add_child(aim_timer)
	aim_timer.start()

	decision_timer = Timer.new()
	decision_timer.name = "AIDecisionTimer"
	decision_timer.wait_time = randf_range(2.0, 4.0) # Time between firing decisions
	decision_timer.timeout.connect(_make_fire_decision)
	add_child(decision_timer)
	decision_timer.start()

func _physics_process(delta: float) -> void:
	super(delta) # Calls BaseWarrior._physics_process() for draw power, health bar, etc.
	if is_dead: return
	# AI specific logic is mainly in timers, not much needed here directly

func set_target(target: Node2D) -> void:
	player_node = target

func _update_aim() -> void:
	if is_dead or not is_instance_valid(player_node) or not is_instance_valid(torso) or not is_instance_valid(bow_pivot):
		return

	var target_torso = player_node.get_node_or_null("Torso") as RigidBody2D
	if target_torso:
		current_target_pos = target_torso.global_position + Vector2(randf_range(-25, 25), randf_range(-25, 25)) # Increased inaccuracy slightly
	else:
		current_target_pos = player_node.global_position

	var target_angle = (current_target_pos - torso.global_position).angle()
	bow_pivot.global_rotation = target_angle #lerp(bow_pivot.global_rotation, target_angle, 0.3) # Optional smoothing
	# Bow visual flip is handled by super._physics_process()

func _make_fire_decision() -> void:
	if is_dead or is_drawing_bow or not is_instance_valid(player_node): return
	
	start_drawing_bow() # Call method from BaseWarrior
	
	var draw_duration = randf_range(0.5, MAX_DRAW_POWER / get_draw_speed() * 0.85) 

	decision_timer.stop()

	var draw_complete_timer = Timer.new()
	draw_complete_timer.name = "AIDrawCompleteTimer"
	draw_complete_timer.wait_time = draw_duration
	draw_complete_timer.one_shot = true
	draw_complete_timer.timeout.connect(func():
		if is_instance_valid(self) and not is_dead:
			_attempt_fire()
		draw_complete_timer.queue_free()
	)
	add_child(draw_complete_timer)
	draw_complete_timer.start()
		
func _attempt_fire() -> void:
	if is_dead or not is_drawing_bow: return
	
	release_bow() # Call method from BaseWarrior

	decision_timer.wait_time = randf_range(1.5, 3.5)
	if not is_dead: # Only restart if not dead during the draw
		decision_timer.start()

# Override die to also stop AI timers
func die() -> void:
	if is_dead: return # Prevent calling super.die() multiple times if already dead
	stop_ai_timers()
	super() # Calls BaseWarrior.die() for ragdoll and signal

func stop_ai_timers() -> void:
	if is_instance_valid(aim_timer): aim_timer.stop()
	if is_instance_valid(decision_timer): decision_timer.stop()
	# Stop any other AI-specific timers
	for child in get_children():
		if child is Timer and child.name == "AIDrawCompleteTimer":
			child.stop() # Stop any pending fire commands

# func get_draw_speed() -> float:
#     return DRAW_SPEED * AI_DRAW_SPEED_MULTIPLIER # Example of AI having different draw speed
