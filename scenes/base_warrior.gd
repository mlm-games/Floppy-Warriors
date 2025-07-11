class_name BaseWarrior
extends Node2D

signal died

@export var health: int = 100
@export var arrow_scene: PackedScene
@export var knockback_intensity_multiplier := 7

@onready var torso: RigidBody2D = %Torso
@onready var bow_pivot: Node2D = %BowPivot
@onready var arrow_spawn_point: Marker2D = %ArrowSpawnPoint
@onready var health_bar: ProgressBar = %HealthBar
@onready var bow_visual: Polygon2D = %BowVisual

var is_drawing_bow: bool = false
var current_draw_power: float = 0.0
const MAX_DRAW_POWER: float = 100.0
const DRAW_SPEED: float = 160.0
const MIN_LAUNCH_FORCE: float = 100.0
const MAX_LAUNCH_FORCE_ADDITION: float = 1000.0
const HEALTH_BAR_POS_OFFSET = Vector2(0, -60) # Adjust offset as needed

var is_dead: bool = false
var initial_health: int
var queue_free_tween: Tween

func _ready() -> void:
	initial_health = health
	health_bar.max_value = initial_health
	health_bar.value = health
	
	%VisibleOnScreenNotifier2D.screen_exited.connect(_on_screen_exited)

func _physics_process(delta: float) -> void:
	if is_dead: return

	update_health_bar_position()
	
	if is_drawing_bow:
		current_draw_power = min(current_draw_power + get_draw_speed() * delta, MAX_DRAW_POWER)
		bow_visual.modulate = Color(1, 1 - current_draw_power / MAX_DRAW_POWER, 1 - current_draw_power / MAX_DRAW_POWER)
		%BowPointLight2D.energy =  current_draw_power / MAX_DRAW_POWER
	else:
		%BowPointLight2D.energy = lerpf(%BowPointLight2D.energy, 0, delta*5)
	
	# Aiming logic will be specific to Player (mouse) and AI (target), so it's not here.
	# But bow visual flip can be common
	var aim_direction_angle_deg = bow_pivot.global_rotation_degrees
	if aim_direction_angle_deg > 90 or aim_direction_angle_deg < -90:
		bow_pivot.scale.y = -1
	else:
		bow_pivot.scale.y = 1

func update_health_bar_position() -> void:
	health_bar.global_position = torso.global_position + HEALTH_BAR_POS_OFFSET
	health_bar.rotation = 0 # Keep it upright relative to global space

func get_draw_speed() -> float:
	# Allows subclasses to override if needed
	return DRAW_SPEED

func start_drawing_bow() -> void:
	if is_dead: return
	is_drawing_bow = true
	current_draw_power = 0.0

func release_bow() -> void:
	if is_dead or not is_drawing_bow: return
	is_drawing_bow = false
	fire_arrow()
	bow_visual.modulate = Color.WHITE # Reset bow color

func fire_arrow() -> void:
	if not arrow_scene:
		printerr(name + ": Arrow scene not set!")
		return
	if not is_instance_valid(arrow_spawn_point):
		printerr(name + ": ArrowSpawnPoint not found or invalid!")
		return

	var arrow_instance : RigidBody2D = arrow_scene.instantiate()
	get_tree().current_scene.add_child(arrow_instance)

	arrow_instance.global_transform = arrow_spawn_point.global_transform
	
	var launch_force = MIN_LAUNCH_FORCE + (current_draw_power / MAX_DRAW_POWER) * MAX_LAUNCH_FORCE_ADDITION
	arrow_instance.linear_velocity = arrow_instance.transform.x * launch_force
	arrow_instance.set_shooter(self) # 'self' will be the Player or AIPlayer instance

func take_damage(amount: int, knockback_direction: Vector2 = Vector2.ZERO) -> void:
	if is_dead: return
	health = max(0, health - amount)
	health_bar.value = health
	print(name + " health: ", health)

	if health <= 0:
		die()
	else:
		torso.apply_central_impulse(knockback_direction * amount * knockback_intensity_multiplier)

func die() -> void:
	if is_dead: return
	is_dead = true
	health_bar.value = 0
	health_bar.visible = false
	print(name + " has died!")
	
	# Make ragdoll limp, adjust collision
	for child_node in get_children():
		if child_node is RigidBody2D:
			child_node.set_collision_layer_value(C.CollisionLayers.PlayersActive, false)
			child_node.set_collision_layer_value(C.CollisionLayers.PlayersDead, true)
			child_node.set_collision_mask_value(C.CollisionLayers.PlayersActive, false)
			child_node.set_collision_mask_value(C.CollisionLayers.PlayersDead, true)  # Collide with "players_dead"
			child_node.set_collision_mask_value(C.CollisionLayers.Arrows, false)
			# Apply small random impulse
			child_node.apply_central_impulse(Vector2(randf_range(-30, 30), randf_range(-80, -10)))
	died.emit()

func get_current_health() -> int:
	return health

func set_initial_health_reference() -> void:
	initial_health = health
	if health_bar:
		health_bar.max_value = initial_health
		health_bar.value = health

func _on_screen_exited() -> void:
	print("body " + self.name + " left the screen...")
	die()


func call_queue_free()  -> void:
	if queue_free_tween:
		queue_free_tween.kill()
	
	await get_tree().create_timer(2).timeout
	queue_free_tween = get_tree().create_tween().set_ease(Tween.EASE_IN_OUT).set_trans(Tween.TRANS_CUBIC)
	
	queue_free_tween.tween_property(self, "modulate", Color.TRANSPARENT, 0.75) 
	await queue_free_tween.finished
	queue_free()
