extends RigidBody2D

const DAMAGE: int = 30
const LIFESPAN: float = 5.0 # Seconds before self-destruct if nothing hit

var shooter: Node = null # Who shot this arrow
var has_hit: bool = false

var vanish_tween: Tween

func _ready() -> void:
	body_entered.connect(_on_body_entered)
	var timer = Timer.new()
	timer.wait_time = LIFESPAN
	timer.one_shot = true
	timer.timeout.connect(call_queue_free)
	add_child(timer)
	timer.start()

func _physics_process(_delta: float) -> void:
	if not has_hit and linear_velocity.length() > 0.1:
		rotation = linear_velocity.angle()

func set_shooter(s: Node) -> void:
	shooter = s

func _on_body_entered(body: Node) -> void:
	if has_hit: return
	if body == shooter: return # Prevent hitting self immediately

	has_hit = true
	set_collision_layer_value(1, false) # Disable further collisions for this arrow
	set_collision_mask_value(1, false)

	var current_parent = get_parent()
	if current_parent:
		current_parent.remove_child.call_deferred(self)
	body.add_child.call_deferred(self) # Stick arrow to the body it hit
	# Optional: Adjust local position/rotation to look better when stuck

	if body.get_parent() is BaseWarrior:
			var target_root = body.get_owner() # Try to get the scene root of the limb
			if target_root and target_root.has_method("take_damage") and target_root != shooter:
				# Pass normalized velocity of the arrow for knockback direction
				target_root.take_damage(DAMAGE, linear_velocity.normalized()) 

	# Extend the lifespan timer as it's now stuck
	for child in get_children():
		if child is Timer:
			child.wait_time += 5

func call_queue_free()  -> void:
	if vanish_tween:
		vanish_tween.kill()
	
	await get_tree().create_timer(2).timeout
	vanish_tween = get_tree().create_tween().set_ease(Tween.EASE_IN_OUT).set_trans(Tween.TRANS_CUBIC)
	
	vanish_tween.tween_property(self, "modulate", Color.TRANSPARENT, 0.75)
	vanish_tween 
	await vanish_tween.finished
	queue_free()
