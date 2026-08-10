extends RigidBody2D

@export var arrow_data: ArrowData = preload("uid://cutbot3ne5uq3")

var shooter: Node
var has_hit: bool = false

# Snapshot the shooter's values. The shot remains valid if its shooter dies.
var shot_damage_multiplier: float = 1.0
var shot_headshot_multiplier: float = 1.0
var shot_knockback_multiplier: float = 1.0

var attachment_joint: PinJoint2D
var vanish_tween: Tween
var is_fading_out: bool = false

@onready var lifespan_timer: Timer = %LifespanTimer


func _ready() -> void:
	body_entered.connect(_on_body_entered)
	lifespan_timer.timeout.connect(call_queue_free)


func _physics_process(_delta: float) -> void:
	if not has_hit and linear_velocity.length() > 0.1:
		rotation = linear_velocity.angle()


func set_shooter(value: Node) -> void:
	shooter = value

	if shooter is BaseWarrior:
		var warrior := shooter as BaseWarrior
		shot_damage_multiplier = warrior.arrow_damage_multiplier
		shot_headshot_multiplier = warrior.headshot_damage_multiplier
		shot_knockback_multiplier = warrior.arrow_knockback_multiplier


func _on_body_entered(body: Node) -> void:
	if has_hit:
		return

	var target_warrior := _find_warrior_for_body(body)

	# The old body == shooter check did not catch the shooter's limbs.
	if (
		target_warrior != null
		and is_instance_valid(shooter)
		and target_warrior == shooter
	):
		return

	has_hit = true

	var impact_direction := linear_velocity.normalized()

	# Prevent a second collision while the attachment is deferred.
	collision_layer = 0
	collision_mask = 0

	if target_warrior != null:
		var limb_name := body.name
		var limb_multiplier := 1.0
		var base_damage := 20.0
		var knockback_multiplier := shot_knockback_multiplier

		if arrow_data != null:
			base_damage = arrow_data.damage
			limb_multiplier = arrow_data.get_limb_damage_multiplier(
				limb_name
			)
			knockback_multiplier *= arrow_data.knockback_multiplier

		if limb_name == &"Head":
			limb_multiplier *= shot_headshot_multiplier

		var final_damage := maxi(
			1,
			roundi(
				base_damage
				* shot_damage_multiplier
				* limb_multiplier
			)
		)

		var killed := target_warrior.take_damage(
			final_damage,
			impact_direction,
			body as RigidBody2D,
			knockback_multiplier
		)

		if is_instance_valid(shooter) and shooter is BaseWarrior:
			(shooter as BaseWarrior).register_hit(
				limb_name,
				final_damage,
				killed
			)

	_stick_to_body.call_deferred(body)

	var stuck_lifetime := 2.0
	if arrow_data != null:
		stuck_lifetime = maxf(0.25, arrow_data.stuck_lifetime)

	lifespan_timer.start(stuck_lifetime)


func _find_warrior_for_body(body: Node) -> BaseWarrior:
	if body is BaseWarrior:
		return body as BaseWarrior

	if body.owner is BaseWarrior:
		return body.owner as BaseWarrior

	var current := body.get_parent()

	while current != null:
		if current is BaseWarrior:
			return current as BaseWarrior

		current = current.get_parent()

	return null


func _stick_to_body(body: Node) -> void:
	if not is_inside_tree() or not is_instance_valid(body):
		return

	linear_velocity *= 0.08
	angular_velocity = 0.0
	gravity_scale = 0.0
	mass = 0.05
	linear_damp = 12.0
	angular_damp = 12.0

	if body is PhysicsBody2D:
		var pin := PinJoint2D.new()
		pin.name = "ArrowAttachment"
		pin.disable_collision = true
		pin.softness = 0.0

		get_tree().current_scene.add_child(pin)
		pin.global_position = global_position

		# Both nodes must already be in the tree before these paths are made.
		pin.node_a = pin.get_path_to(self)
		pin.node_b = pin.get_path_to(body)

		attachment_joint = pin
	else:
		# Covers TileMap collision owners or other non-PhysicsBody contacts.
		freeze = true


func call_queue_free() -> void:
	if is_fading_out or is_queued_for_deletion():
		return

	is_fading_out = true
	lifespan_timer.stop()

	if vanish_tween:
		vanish_tween.kill()

	vanish_tween = get_tree().create_tween()
	vanish_tween.set_ease(Tween.EASE_IN_OUT)
	vanish_tween.set_trans(Tween.TRANS_CUBIC)
	vanish_tween.tween_property(self, "modulate", Color.TRANSPARENT, 0.75)
	vanish_tween.tween_callback(_free_arrow)


func _free_arrow() -> void:
	if is_instance_valid(attachment_joint):
		attachment_joint.queue_free()

	queue_free()