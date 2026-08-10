class_name EnemyAI
extends BaseWarrior

var player_node: BaseWarrior
var current_target_pos: Vector2
var current_aim_offset: Vector2

var aim_error_pixels: float = 25.0
var decision_delay_min: float = 1.5
var decision_delay_max: float = 3.5

@onready var ai_draw_complete_timer: Timer = %AiDrawCompleteTimer
@onready var ai_aim_timer: Timer = %AiAimTimer
@onready var decision_timer: Timer = %DecisionTimer


func _ready() -> void:
	super()

	ai_draw_complete_timer.one_shot = true
	ai_aim_timer.one_shot = false
	decision_timer.one_shot = true

	if not ai_aim_timer.timeout.is_connected(_update_aim):
		ai_aim_timer.timeout.connect(_update_aim)

	if not ai_draw_complete_timer.timeout.is_connected(_attempt_fire):
		ai_draw_complete_timer.timeout.connect(_attempt_fire)

	if not decision_timer.timeout.is_connected(_make_fire_decision):
		decision_timer.timeout.connect(_make_fire_decision)

	ai_aim_timer.wait_time = 0.1
	ai_aim_timer.start()

	_schedule_next_decision()


func set_target(target: Node2D) -> void:
	player_node = target as BaseWarrior
	_choose_aim_offset()
	_update_aim()


func configure_for_round(config: Dictionary) -> void:
	health = maxi(1, int(config.get("health", health)))

	arrow_damage_multiplier = float(
		config.get("damage_multiplier", 1.0)
	)
	draw_speed_multiplier = float(
		config.get("draw_speed_multiplier", 1.0)
	)

	aim_error_pixels = maxf(
		0.0,
		float(config.get("aim_error", aim_error_pixels))
	)

	decision_delay_min = maxf(
		0.2,
		float(config.get("decision_delay_min", decision_delay_min))
	)
	decision_delay_max = maxf(
		decision_delay_min,
		float(config.get("decision_delay_max", decision_delay_max))
	)

	set_initial_health_reference()

	if not is_dead:
		ai_aim_timer.start()
		_schedule_next_decision()


func _update_aim() -> void:
	if (
		is_dead
		or not is_instance_valid(player_node)
		or player_node.is_dead
	):
		return

	var target_torso := player_node.torso

	if is_instance_valid(target_torso):
		var distance := torso.global_position.distance_to(
			target_torso.global_position
		)

		# Light target leading. It remains intentionally inaccurate.
		var estimated_arrow_speed := (
			MIN_LAUNCH_FORCE
			+ MAX_LAUNCH_FORCE_ADDITION * 0.65
		)

		var lead_time := clampf(
			distance / maxf(estimated_arrow_speed, 1.0) * 0.45,
			0.0,
			0.45
		)

		current_target_pos = (
			target_torso.global_position
			+ target_torso.linear_velocity * lead_time
			+ current_aim_offset
		)
	else:
		current_target_pos = player_node.global_position + current_aim_offset

	bow_pivot.global_rotation = (
		current_target_pos - torso.global_position
	).angle()


func _choose_aim_offset() -> void:
	current_aim_offset = Vector2(
		randf_range(-aim_error_pixels, aim_error_pixels),
		randf_range(-aim_error_pixels, aim_error_pixels)
	)


func _make_fire_decision() -> void:
	if is_dead:
		return

	if (
		not is_instance_valid(player_node)
		or player_node.is_dead
	):
		decision_timer.start(0.25)
		return

	if is_drawing_bow:
		decision_timer.start(0.25)
		return

	_choose_aim_offset()
	_update_aim()
	start_drawing_bow()

	var full_draw_duration := MAX_DRAW_POWER / get_draw_speed()
	var maximum_duration := maxf(0.45, full_draw_duration * 0.9)
	var minimum_duration := minf(0.45, maximum_duration)

	ai_draw_complete_timer.start(
		randf_range(minimum_duration, maximum_duration)
	)


func _attempt_fire() -> void:
	if is_dead or not is_drawing_bow:
		return

	if (
		not is_instance_valid(player_node)
		or player_node.is_dead
	):
		cancel_bow_draw()
		return

	_update_aim()
	release_bow()
	_schedule_next_decision()


func _schedule_next_decision() -> void:
	if is_dead:
		return

	decision_timer.start(
		randf_range(decision_delay_min, decision_delay_max)
	)


func die() -> void:
	stop_ai_timers()
	super()


func stop_ai_timers() -> void:
	if is_instance_valid(ai_aim_timer):
		ai_aim_timer.stop()

	if is_instance_valid(decision_timer):
		decision_timer.stop()

	if is_instance_valid(ai_draw_complete_timer):
		ai_draw_complete_timer.stop()