class_name BaseWarrior
extends Node2D

signal died
signal health_changed(current_health: int, max_health: int)
signal hit_confirmed(limb_name: StringName, damage: int, killed: bool)

@export var health: int = 100
@export var arrow_scene: PackedScene
@export var knockback_intensity_multiplier: float = 7.0

@onready var torso: RigidBody2D = %Torso
@onready var bow_pivot: Node2D = %BowPivot
@onready var arrow_spawn_point: Marker2D = %ArrowSpawnPoint
@onready var health_bar: ProgressBar = %HealthBar
@onready var bow_visual: Sprite2D = %BowVisual
@onready var bow_light: PointLight2D = %BowPointLight2D

const MAX_DRAW_POWER: float = 100.0
const DRAW_SPEED: float = 160.0
const MIN_LAUNCH_FORCE: float = 100.0
const MAX_LAUNCH_FORCE_ADDITION: float = 1000.0
const HEALTH_BAR_POS_OFFSET := Vector2(0, -60)

var is_drawing_bow: bool = false
var current_draw_power: float = 0.0
var is_dead: bool = false

var max_health: int
var initial_health: int

# Per-run combat modifiers. RoundManager changes these on the player.
var arrow_damage_multiplier: float = 1.0
var arrow_velocity_multiplier: float = 1.0
var arrow_knockback_multiplier: float = 1.0
var headshot_damage_multiplier: float = 1.0
var draw_speed_multiplier: float = 1.0

var arrow_count: int = 1
var arrow_spread_degrees: float = 10.0

var queue_free_tween: Tween
var is_fading_out: bool = false


func _ready() -> void:
	initial_health = maxi(1, health)
	max_health = initial_health
	_sync_health_bar()

	%VisibleOnScreenNotifier2D.screen_exited.connect(_on_screen_exited)


func _physics_process(delta: float) -> void:
	if is_dead:
		return

	update_health_bar_position()

	if is_drawing_bow:
		current_draw_power = minf(
			current_draw_power + get_draw_speed() * delta,
			MAX_DRAW_POWER
		)

		var draw_ratio := current_draw_power / MAX_DRAW_POWER
		bow_visual.modulate = Color(1.0, 1.0 - draw_ratio, 1.0 - draw_ratio)
		bow_light.energy = draw_ratio
	else:
		bow_light.energy = lerpf(bow_light.energy, 0.0, delta * 5.0)

	var aim_angle_degrees := bow_pivot.global_rotation_degrees
	if aim_angle_degrees > 90.0 or aim_angle_degrees < -90.0:
		bow_pivot.scale.y = -1.0
	else:
		bow_pivot.scale.y = 1.0


func update_health_bar_position() -> void:
	if not is_instance_valid(health_bar) or not is_instance_valid(torso):
		return

	health_bar.global_position = torso.global_position + HEALTH_BAR_POS_OFFSET
	health_bar.rotation = 0.0


func get_draw_speed() -> float:
	return DRAW_SPEED * maxf(0.1, draw_speed_multiplier)


func start_drawing_bow() -> void:
	if is_dead:
		return

	is_drawing_bow = true
	current_draw_power = 0.0


func release_bow() -> void:
	if is_dead or not is_drawing_bow:
		return

	is_drawing_bow = false
	fire_arrow()

	bow_visual.modulate = Color.WHITE
	bow_light.energy = 0.0


func cancel_bow_draw() -> void:
	is_drawing_bow = false
	current_draw_power = 0.0

	if is_instance_valid(bow_visual):
		bow_visual.modulate = Color.WHITE

	if is_instance_valid(bow_light):
		bow_light.energy = 0.0


func fire_arrow() -> void:
	if not arrow_scene:
		printerr(name + ": Arrow scene not set!")
		return

	if not is_instance_valid(arrow_spawn_point):
		printerr(name + ": ArrowSpawnPoint not found or invalid!")
		return

	var launch_force := (
		MIN_LAUNCH_FORCE
		+ (current_draw_power / MAX_DRAW_POWER) * MAX_LAUNCH_FORCE_ADDITION
	)

	var shot_count := maxi(1, arrow_count)

	for shot_index in range(shot_count):
		var arrow_instance := arrow_scene.instantiate() as RigidBody2D

		if arrow_instance == null:
			printerr(name + ": Arrow scene root must be a RigidBody2D.")
			return

		get_tree().current_scene.add_child(arrow_instance)
		arrow_instance.global_transform = arrow_spawn_point.global_transform

		var angle_offset := 0.0

		if shot_count > 1:
			var spread_step := arrow_spread_degrees / float(shot_count - 1)
			var offset_degrees := (
				-arrow_spread_degrees * 0.5
				+ spread_step * float(shot_index)
			)
			angle_offset = deg_to_rad(offset_degrees)

		arrow_instance.global_rotation += angle_offset

		if arrow_instance.has_method("set_shooter"):
			arrow_instance.set_shooter(self)

		var direction := arrow_instance.global_transform.x.normalized()
		arrow_instance.linear_velocity = (
			direction
			* launch_force
			* arrow_velocity_multiplier
		)


func take_damage(
	amount: float,
	knockback_direction: Vector2 = Vector2.ZERO,
	hit_body: RigidBody2D = null,
	knockback_multiplier: float = 1.0
) -> bool:
	if is_dead:
		return false

	var final_damage := maxi(1, roundi(amount))
	health = maxi(0, health - final_damage)

	var impulse_target: RigidBody2D = torso
	if is_instance_valid(hit_body):
		impulse_target = hit_body

	if (
		is_instance_valid(impulse_target)
		and knockback_direction.length_squared() > 0.0
	):
		impulse_target.sleeping = false
		impulse_target.apply_central_impulse(
			knockback_direction.normalized()
			* float(final_damage)
			* knockback_intensity_multiplier
			* knockback_multiplier
		)

	_sync_health_bar()
	health_changed.emit(health, max_health)

	if health <= 0:
		die()
		return true

	return false


func die() -> void:
	if is_dead:
		return

	is_dead = true
	health = 0
	cancel_bow_draw()

	if is_instance_valid(health_bar):
		health_bar.value = 0
		health_bar.visible = false

	health_changed.emit(health, max_health)

	for child_node in get_children():
		if child_node is RigidBody2D:
			var body := child_node as RigidBody2D

			body.set_collision_layer_value(
				C.CollisionLayers.PlayersActive,
				false
			)
			body.set_collision_layer_value(
				C.CollisionLayers.PlayersDead,
				true
			)

			body.set_collision_mask_value(
				C.CollisionLayers.PlayersActive,
				false
			)
			body.set_collision_mask_value(
				C.CollisionLayers.PlayersDead,
				true
			)
			body.set_collision_mask_value(
				C.CollisionLayers.Arrows,
				false
			)

			body.sleeping = false
			body.apply_central_impulse(
				Vector2(
					randf_range(-30.0, 30.0),
					randf_range(-80.0, -10.0)
				)
			)

	died.emit()


func set_initial_health_reference() -> void:
	max_health = maxi(1, health)
	initial_health = max_health
	_sync_health_bar()
	health_changed.emit(health, max_health)


func set_max_health(value: int, heal_added_health: bool = true) -> void:
	var old_maximum := max_health
	max_health = maxi(1, value)
	initial_health = max_health

	if heal_added_health:
		health += maxi(0, max_health - old_maximum)

	health = clampi(health, 0, max_health)

	_sync_health_bar()
	health_changed.emit(health, max_health)


func add_max_health(amount: int, heal_added_health: bool = true) -> void:
	set_max_health(max_health + amount, heal_added_health)


func heal(amount: int) -> int:
	if is_dead or amount <= 0:
		return 0

	var previous_health := health
	health = mini(max_health, health + amount)

	_sync_health_bar()
	health_changed.emit(health, max_health)

	return health - previous_health


func register_hit(
	limb_name: StringName,
	damage: int,
	killed: bool
) -> void:
	hit_confirmed.emit(limb_name, damage, killed)


func get_current_health() -> int:
	return health


func get_max_health() -> int:
	return max_health


func _sync_health_bar() -> void:
	if not is_instance_valid(health_bar):
		return

	health_bar.max_value = max_health
	health_bar.value = health


func _on_screen_exited() -> void:
	die()


func call_queue_free() -> void:
	if is_fading_out or is_queued_for_deletion():
		return

	is_fading_out = true

	if queue_free_tween:
		queue_free_tween.kill()

	await get_tree().create_timer(2.0).timeout

	if not is_inside_tree():
		return

	queue_free_tween = get_tree().create_tween()
	queue_free_tween.set_ease(Tween.EASE_IN_OUT)
	queue_free_tween.set_trans(Tween.TRANS_CUBIC)
	queue_free_tween.tween_property(self, "modulate", Color.TRANSPARENT, 0.75)
	queue_free_tween.tween_callback(queue_free)