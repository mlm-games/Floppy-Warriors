class_name BaseWarrior
extends Node2D

signal died

@export var health: int = 100
@export var arrow_scene: PackedScene

# These will be assigned by the inheriting class or in _ready if paths are consistent
@onready var torso: RigidBody2D = get_node_or_null("Torso")
@onready var bow_pivot: Node2D = get_node_or_null("Torso/BowPivot")
@onready var arrow_spawn_point: Marker2D = get_node_or_null("Torso/BowPivot/ArrowSpawnPoint")
@onready var health_bar: ProgressBar = get_node_or_null("HealthBar")
@onready var bow_visual: Polygon2D = get_node_or_null("Torso/BowPivot/BowVisual")

#@onready var camera_2d: Camera2D = %Camera2D


var is_drawing_bow: bool = false
var current_draw_power: float = 0.0 # Renamed from draw_power for clarity
const MAX_DRAW_POWER: float = 100.0
const DRAW_SPEED: float = 160.0 # Can be overridden by subclasses if needed
const MIN_LAUNCH_FORCE: float = 100.0
const MAX_LAUNCH_FORCE_ADDITION: float = 1000.0

var is_dead: bool = false
var initial_health: int
var queue_free_tween: Tween

func _ready() -> void:
	# Ensure nodes are found, especially if paths differ slightly in inherited scenes
	if not torso: printerr(name + ": Torso node not found!")
	if not bow_pivot: printerr(name + ": BowPivot node not found!")
	if not arrow_spawn_point: printerr(name + ": ArrowSpawnPoint node not found!")
	if not health_bar: printerr(name + ": HealthBar node not found!")
	if not bow_visual: printerr(name + ": BowVisual node not found!")

	initial_health = health
	if health_bar:
		health_bar.max_value = initial_health
		health_bar.value = health
	
	# This will be called by Player and AIPlayer's _ready too
	# If you have specific _ready logic for Player/AI, use super()._ready()
	%VisibleOnScreenNotifier2D.screen_exited.connect(_on_screen_exited)

func _physics_process(delta: float) -> void:
	if is_dead: return

	update_health_bar_position()
	
	if is_drawing_bow:
		current_draw_power = min(current_draw_power + get_draw_speed() * delta, MAX_DRAW_POWER)
		if is_instance_valid(bow_visual):
			bow_visual.modulate = Color(1, 1 - current_draw_power / MAX_DRAW_POWER, 1 - current_draw_power / MAX_DRAW_POWER)
			%BowPointLight2D.energy =  current_draw_power / MAX_DRAW_POWER
	else:
		%BowPointLight2D.energy = lerpf(%BowPointLight2D.energy, 0, delta*5)
	
	# Aiming logic will be specific to Player (mouse) and AI (target), so it's not here.
	# But bow visual flip can be common
	if is_instance_valid(bow_pivot) and is_instance_valid(torso):
		var aim_direction_angle_deg = rad_to_deg(bow_pivot.global_rotation)
		if aim_direction_angle_deg > 90 or aim_direction_angle_deg < -90:
			bow_pivot.scale.y = -1
		else:
			bow_pivot.scale.y = 1

func update_health_bar_position() -> void:
	if health_bar and is_instance_valid(torso):
		health_bar.global_position = torso.global_position + Vector2(0, -60) # Adjust offset as needed
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
	if is_instance_valid(bow_visual):
		bow_visual.modulate = Color.WHITE # Reset bow color

func fire_arrow() -> void:
	if not arrow_scene:
		printerr(name + ": Arrow scene not set!")
		return
	if not is_instance_valid(arrow_spawn_point):
		printerr(name + ": ArrowSpawnPoint not found or invalid!")
		return

	var arrow_instance = arrow_scene.instantiate() as RigidBody2D
	get_tree().current_scene.add_child(arrow_instance)

	arrow_instance.global_transform = arrow_spawn_point.global_transform
	
	var launch_force = MIN_LAUNCH_FORCE + (current_draw_power / MAX_DRAW_POWER) * MAX_LAUNCH_FORCE_ADDITION
	arrow_instance.linear_velocity = arrow_instance.transform.x * launch_force
	arrow_instance.set_shooter(self) # 'self' will be the Player or AIPlayer instance

func take_damage(amount: int, knockback_direction: Vector2 = Vector2.ZERO) -> void:
	if is_dead: return
	health = max(0, health - amount)
	if health_bar:
		health_bar.value = health
	print(name + " health: ", health)

	if health <= 0:
		die()
	else:
		if is_instance_valid(torso):
			torso.apply_central_impulse(knockback_direction * amount * 7) # Adjust multiplier

func die() -> void:
	if is_dead: return
	is_dead = true
	if health_bar:
		health_bar.value = 0
		health_bar.visible = false
	print(name + " has died!")
	
	# Make ragdoll limp, adjust collision
	for child_node in get_children():
		if child_node is RigidBody2D:
			child_node.set_collision_layer_value(1, false) # Disable "players_active"
			child_node.set_collision_layer_value(2, true)  # Enable "players_dead"
			child_node.set_collision_mask_value(1, false) # Don't collide with "players_active"
			child_node.set_collision_mask_value(2, true)  # Collide with "players_dead"
			child_node.set_collision_mask_value(4, false) # Don't collide with "arrows"
			# Optional: apply small random impulse
			child_node.apply_central_impulse(Vector2(randf_range(-30, 30), randf_range(-80, -10)))
	died.emit()

# Helper methods for main.gd
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
	queue_free_tween 
	await queue_free_tween.finished
	queue_free()
