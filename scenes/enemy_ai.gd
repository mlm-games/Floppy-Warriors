class_name EnemyAI extends BaseWarrior

var player_node: BaseWarrior = null 

var current_target_pos: Vector2

# const AI_DRAW_SPEED_MULTIPLIER = 0.8 
@onready var ai_draw_complete_timer: Timer = %AiDrawCompleteTimer
@onready var ai_aim_timer: Timer = %AiAimTimer
@onready var decision_timer: Timer = %DecisionTimer

func _ready() -> void:
	super()
	
	ai_aim_timer.timeout.connect(_update_aim)
	ai_draw_complete_timer.timeout.connect(_attempt_fire)
	
	decision_timer.wait_time = randf_range(2.0, 4.0) # Time between firing decisions
	decision_timer.timeout.connect(_make_fire_decision)

func set_target(target: Node2D) -> void:
	player_node = target

func _update_aim() -> void:
	if is_dead:
		return

	var target_torso : RigidBody2D = player_node.torso
	if target_torso:
		current_target_pos = target_torso.global_position + Vector2(randf_range(-25, 25), randf_range(-25, 25)) # Increased inaccuracy slightly
	else:
		current_target_pos = player_node.global_position

	var target_angle = (current_target_pos - torso.global_position).angle()
	bow_pivot.global_rotation = target_angle #lerp(bow_pivot.global_rotation, target_angle, 0.3) # Optional smoothing
	# Bow visual flip is handled by super._physics_process()

func _make_fire_decision() -> void:
	if is_dead or is_drawing_bow: return
	
	start_drawing_bow()
	
	var draw_duration = randf_range(0.5, MAX_DRAW_POWER / get_draw_speed() * 0.85) 

	decision_timer.stop()

	ai_draw_complete_timer.wait_time = draw_duration
	ai_draw_complete_timer.start()
		
func _attempt_fire() -> void:
	if is_dead or not is_drawing_bow: return
	
	release_bow()

	decision_timer.wait_time = randf_range(1.5, 3.5)
	if not is_dead:
		decision_timer.start()

# Override die to also stop AI timers
func die() -> void:
	stop_ai_timers()
	super()

func stop_ai_timers() -> void:
	ai_aim_timer.stop()
	decision_timer.stop()
	ai_draw_complete_timer.stop()

# func get_draw_speed() -> float:
#     return DRAW_SPEED * AI_DRAW_SPEED_MULTIPLIER # TODO:Progression of AI having different draw speed
